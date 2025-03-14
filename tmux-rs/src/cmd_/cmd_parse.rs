use compat_rs::queue::{
    tailq_empty, tailq_first, tailq_foreach, tailq_foreach_safe, tailq_init, tailq_insert_tail, tailq_last,
    tailq_remove,
};
use libc::memset;

use crate::{
    xmalloc::{Zeroable, xrecallocarray, xrecallocarray_},
    *,
};

#[rustfmt::skip]
unsafe extern "C" {
    fn yyparse() -> i32;

    // pub fn cmd_parse_from_file(_: *mut FILE, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    // pub fn cmd_parse_from_string(_: *const c_char, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    // pub fn cmd_parse_and_insert( _: *const c_char, _: *mut cmd_parse_input, _: *mut cmdq_item, _: *mut cmdq_state, _: *mut *mut c_char,) -> cmd_parse_status;
    // pub fn cmd_parse_and_append( _: *const c_char, _: *mut cmd_parse_input, _: *mut client, _: *mut cmdq_state, _: *mut *mut c_char,) -> cmd_parse_status;
    // pub fn cmd_parse_from_buffer(_: *const c_void, _: usize, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    // pub fn cmd_parse_from_arguments(_: *mut args_value, _: c_uint, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
}

compat_rs::impl_tailq_entry!(cmd_parse_scope, entry, tailq_entry<cmd_parse_scope>);
// #[derive(compat_rs::TailQEntry)]
#[repr(C)]
pub struct cmd_parse_scope {
    pub flag: i32,
    // #[entry]
    pub entry: tailq_entry<cmd_parse_scope>,
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum cmd_parse_argument_type {
    CMD_PARSE_STRING,
    CMD_PARSE_COMMANDS,
    CMD_PARSE_PARSED_COMMANDS,
}

unsafe impl Zeroable for cmd_parse_argument {}
compat_rs::impl_tailq_entry!(cmd_parse_argument, entry, tailq_entry<cmd_parse_argument>);
// #[derive(compat_rs::TailQEntry)]
#[repr(C)]
pub struct cmd_parse_argument {
    pub type_: cmd_parse_argument_type,
    pub string: *mut c_char,
    pub commands: *mut cmd_parse_commands,
    pub cmdlist: *mut cmd_list,

    // #[entry]
    pub entry: tailq_entry<cmd_parse_argument>,
}
pub type cmd_parse_arguments = tailq_head<cmd_parse_argument>;

unsafe impl Zeroable for cmd_parse_command {}
compat_rs::impl_tailq_entry!(cmd_parse_command, entry, tailq_entry<cmd_parse_command>);
// #[derive(compat_rs::TailQEntry)]
#[repr(C)]
pub struct cmd_parse_command {
    pub line: u32,
    pub arguments: cmd_parse_arguments,

    // #[entry]
    pub entry: tailq_entry<cmd_parse_command>,
}
pub type cmd_parse_commands = tailq_head<cmd_parse_command>;

#[repr(C)]
pub struct cmd_parse_state {
    pub f: *mut FILE,

    pub buf: *const c_char,
    pub len: usize,
    pub off: usize,

    pub condition: i32,
    pub eol: i32,
    pub eof: i32,
    pub input: *mut cmd_parse_input,
    pub escapes: u32,

    pub error: *mut c_char,
    pub commands: *mut cmd_parse_commands,

    pub scope: *mut cmd_parse_scope,
    pub stack: tailq_head<cmd_parse_scope>,
}

#[unsafe(no_mangle)]
pub static mut parse_state: cmd_parse_state = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_get_error(file: *const c_char, line: u32, error: *const c_char) -> NonNull<c_char> {
    unsafe {
        if (file.is_null()) {
            xstrdup(error)
        } else {
            let mut s = null_mut();
            xasprintf(&raw mut s, c"%s:%u: %s".as_ptr(), file, line, error);
            NonNull::new_unchecked(s)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_print_commands(pi: *mut cmd_parse_input, cmdlist: *mut cmd_list) {
    unsafe {
        if ((*pi).item.is_null() || (!(*pi).flags & CMD_PARSE_VERBOSE != 0)) {
            return;
        }
        let s = cmd_list_print(cmdlist, 0);
        if (!(*pi).file.is_null()) {
            cmdq_print((*pi).item, c"%s:%u: %s".as_ptr(), (*pi).file, (*pi).line, s);
        } else {
            cmdq_print((*pi).item, c"%u: %s".as_ptr(), (*pi).line, s);
        }
        free_(s);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_free_argument(arg: *mut cmd_parse_argument) {
    unsafe {
        match ((*arg).type_) {
            cmd_parse_argument_type::CMD_PARSE_STRING => free_((*arg).string),
            cmd_parse_argument_type::CMD_PARSE_COMMANDS => cmd_parse_free_commands((*arg).commands),
            cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS => cmd_list_free((*arg).cmdlist),
        }
        free_(arg);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_free_arguments(args: *mut cmd_parse_arguments) {
    unsafe {
        tailq_foreach_safe(args, |arg| {
            tailq_remove(args, arg);
            cmd_parse_free_argument(arg);
            ControlFlow::<(), ()>::Continue(())
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_free_command(cmd: *mut cmd_parse_command) {
    unsafe {
        cmd_parse_free_arguments(&raw mut (*cmd).arguments);
        free_(cmd);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_new_commands() -> *mut cmd_parse_commands {
    unsafe {
        let cmds = xmalloc_::<cmd_parse_commands>().as_ptr();
        tailq_init(cmds);
        cmds
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_free_commands(cmds: *mut cmd_parse_commands) {
    unsafe {
        tailq_foreach_safe(cmds, |cmd| {
            tailq_remove(cmds, cmd);
            cmd_parse_free_command(cmd);
            ControlFlow::<(), ()>::Continue(())
        });
        free_(cmds);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_run_parser(cause: *mut *mut c_char) -> *mut cmd_parse_commands {
    unsafe {
        let mut ps = &raw mut parse_state;

        (*ps).commands = null_mut();
        tailq_init(&raw mut (*ps).stack);

        let retval = yyparse();
        tailq_foreach_safe(&raw mut (*ps).stack, |scope| {
            tailq_remove(&raw mut (*ps).stack, scope);
            free_(scope);
            ControlFlow::<(), ()>::Continue(())
        });
        if (retval != 0) {
            *cause = (*ps).error;
            return null_mut();
        }

        if ((*ps).commands.is_null()) {
            return (cmd_parse_new_commands());
        }
        (*ps).commands
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_do_file(
    f: *mut FILE,
    pi: *mut cmd_parse_input,
    cause: *mut *mut c_char,
) -> *mut cmd_parse_commands {
    let mut ps = &raw mut parse_state;
    unsafe {
        memset(ps.cast(), 0, size_of::<cmd_parse_state>());
        (*ps).input = pi;
        (*ps).f = f;
        cmd_parse_run_parser(cause)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_do_buffer(
    buf: *const c_char,
    len: usize,
    pi: *mut cmd_parse_input,
    cause: *mut *mut c_char,
) -> *mut cmd_parse_commands {
    unsafe {
        let mut ps = &raw mut parse_state;

        memset(ps.cast(), 0, size_of::<cmd_parse_state>());
        (*ps).input = pi;
        (*ps).buf = buf;
        (*ps).len = len;
        cmd_parse_run_parser(cause)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_log_commands(cmds: *mut cmd_parse_commands, prefix: *const c_char) {
    unsafe {
        let mut i = 0;
        tailq_foreach(cmds, |cmd| {
            let mut j = 0;
            tailq_foreach(&raw mut (*cmd).arguments, |arg| {
                match ((*arg).type_) {
                    cmd_parse_argument_type::CMD_PARSE_STRING => {
                        log_debug(c"%s %u:%u: %s".as_ptr(), prefix, i, j, (*arg).string)
                    }
                    cmd_parse_argument_type::CMD_PARSE_COMMANDS => {
                        let mut s = null_mut();
                        xasprintf(&raw mut s, c"%s %u:%u".as_ptr(), prefix, i, j);
                        cmd_parse_log_commands((*arg).commands, s);
                        free_(s);
                    }
                    cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS => {
                        let s = cmd_list_print((*arg).cmdlist, 0);
                        log_debug(c"%s %u:%u: %s".as_ptr(), prefix, i, j, s);
                        free_(s);
                    }
                }
                j += 1;

                ControlFlow::<(), ()>::Continue(())
            });
            i += 1;
            ControlFlow::<(), ()>::Continue(())
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_expand_alias(
    cmd: *mut cmd_parse_command,
    pi: *mut cmd_parse_input,
    pr: *mut cmd_parse_result,
) -> i32 {
    let __func__ = c"cmd_parse_expand_alias".as_ptr();
    unsafe {
        if ((*pi).flags & CMD_PARSE_NOALIAS != 0) {
            return (0);
        }
        memset(pr.cast(), 0, size_of::<cmd_parse_result>());

        let first = tailq_first(&raw mut (*cmd).arguments);
        if (first.is_null() || (*first).type_ != cmd_parse_argument_type::CMD_PARSE_STRING) {
            (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
            (*pr).cmdlist = cmd_list_new();
            return 1;
        }
        let mut name = (*first).string;

        let mut alias = cmd_get_alias(name);
        if (alias.is_null()) {
            return (0);
        }
        log_debug(c"%s: %u alias %s = %s".as_ptr(), __func__, (*pi).line, name, alias);

        let mut cause = null_mut();
        let cmds = cmd_parse_do_buffer(alias, strlen(alias), pi, &raw mut cause);
        free_(alias);
        if (cmds.is_null()) {
            (*pr).status = cmd_parse_status::CMD_PARSE_ERROR;
            (*pr).error = cause;
            return (1);
        }

        let last = tailq_last(cmds);
        if (last.is_null()) {
            (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
            (*pr).cmdlist = cmd_list_new();
            return (1);
        }

        tailq_remove(&raw mut (*cmd).arguments, first);
        cmd_parse_free_argument(first);

        tailq_foreach_safe(&raw mut (*cmd).arguments, |arg| {
            tailq_remove(&raw mut (*cmd).arguments, arg);
            tailq_insert_tail(&raw mut (*last).arguments, arg);
            ControlFlow::<(), ()>::Continue(())
        });
        cmd_parse_log_commands(cmds, __func__);

        (*pi).flags |= CMD_PARSE_NOALIAS;
        cmd_parse_build_commands(cmds, pi, pr);
        (*pi).flags &= !CMD_PARSE_NOALIAS;
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_build_command(
    cmd: *mut cmd_parse_command,
    pi: *mut cmd_parse_input,
    pr: *mut cmd_parse_result,
) {
    unsafe {
        let mut cause = null_mut();
        let mut values: *mut args_value = null_mut();
        // struct cmd_parse_argument	*arg;
        // struct cmd			*add;
        // char				*cause;
        // struct args_value		*values = NULL;
        let mut count: u32 = 0;
        let mut idx = 0u32;
        memset(pr.cast(), 0, size_of::<cmd_parse_result>());

        if (cmd_parse_expand_alias(cmd, pi, pr) != 0) {
            return;
        }

        'out: {
            if tailq_foreach(&raw mut (*cmd).arguments, |arg| {
                values = xrecallocarray_(values, count as usize, count as usize + 1, size_of::<args_value>()).as_ptr();
                match ((*arg).type_) {
                    cmd_parse_argument_type::CMD_PARSE_STRING => {
                        (*values.add(count as usize)).type_ = args_type::ARGS_STRING;
                        (*values.add(count as usize)).union_.string = xstrdup((*arg).string).as_ptr();
                    }
                    cmd_parse_argument_type::CMD_PARSE_COMMANDS => {
                        cmd_parse_build_commands((*arg).commands, pi, pr);
                        if ((*pr).status != cmd_parse_status::CMD_PARSE_SUCCESS) {
                            return ControlFlow::<(), ()>::Break(());
                        }
                        (*values.add(count as _)).type_ = args_type::ARGS_COMMANDS;
                        (*values.add(count as _)).union_.cmdlist = (*pr).cmdlist;
                    }
                    cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS => {
                        (*values.add(count as _)).type_ = args_type::ARGS_COMMANDS;
                        (*values.add(count as _)).union_.cmdlist = (*arg).cmdlist;
                        (*(*values.add(count as _)).union_.cmdlist).references += 1;
                    }
                }
                count += 1;
                ControlFlow::<(), ()>::Continue(())
            })
            .is_break()
            {
                break 'out;
            }

            let add = cmd_parse(values, count, (*pi).file, (*pi).line, &raw mut cause);
            if (add.is_null()) {
                (*pr).status = cmd_parse_status::CMD_PARSE_ERROR;
                (*pr).error = cmd_parse_get_error((*pi).file, (*pi).line, cause).as_ptr();
                free_(cause);
                break 'out;
            }
            (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
            (*pr).cmdlist = cmd_list_new();
            cmd_list_append((*pr).cmdlist, add);
        }
        // out:
        for idx in 0..count {
            args_free_value(values.add(idx as usize));
        }
        free_(values);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_build_commands(
    cmds: *mut cmd_parse_commands,
    pi: *mut cmd_parse_input,
    pr: *mut cmd_parse_result,
) {
    let __func__ = c"cmd_parse_build_commands".as_ptr();
    unsafe {
        // struct cmd_parse_command	*cmd;
        let mut line = u32::MAX;
        let mut current: *mut cmd_list = null_mut();
        // struct cmd_list			*current = NULL, *result;
        // char				*s;

        *pr = zeroed();

        /* Check for an empty list. */
        if (tailq_empty(cmds)) {
            (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
            (*pr).cmdlist = cmd_list_new();
            return;
        }
        cmd_parse_log_commands(cmds, __func__);

        /*
         * Parse each command into a command list. Create a new command list
         * for each line (unless the flag is set) so they get a new group (so
         * the queue knows which ones to remove if a command fails when
         * executed).
         */
        let result = cmd_list_new();
        if tailq_foreach(cmds, |cmd| {
            if ((!(*pi).flags & CMD_PARSE_ONEGROUP != 0) && (*cmd).line != line) {
                if (!current.is_null()) {
                    cmd_parse_print_commands(pi, current);
                    cmd_list_move(result, current);
                    cmd_list_free(current);
                }
                current = cmd_list_new();
            }
            if (current.is_null()) {
                current = cmd_list_new();
            }
            line = (*cmd).line;
            line = (*pi).line;

            cmd_parse_build_command(cmd, pi, pr);
            if ((*pr).status != cmd_parse_status::CMD_PARSE_SUCCESS) {
                cmd_list_free(result);
                cmd_list_free(current);
                return ControlFlow::<(), ()>::Break(());
            }
            cmd_list_append_all(current, (*pr).cmdlist);
            cmd_list_free((*pr).cmdlist);
            ControlFlow::<(), ()>::Continue(())
        })
        .is_break()
        {
            return;
        }
        if (!current.is_null()) {
            cmd_parse_print_commands(pi, current);
            cmd_list_move(result, current);
            cmd_list_free(current);
        }

        let s = cmd_list_print(result, 0);
        log_debug(c"%s: %s".as_ptr(), __func__, s);
        free_(s);

        (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
        (*pr).cmdlist = result;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_from_file(f: *mut FILE, mut pi: *mut cmd_parse_input) -> *mut cmd_parse_result {
    unsafe {
        static mut pr: cmd_parse_result = unsafe { zeroed() };
        let mut input: cmd_parse_input = zeroed();
        let mut cause = null_mut();

        if (pi.is_null()) {
            input = zeroed();
            pi = &raw mut input;
        }
        pr = zeroed();

        let cmds = cmd_parse_do_file(f, pi, &raw mut cause);
        if (cmds.is_null()) {
            pr.status = cmd_parse_status::CMD_PARSE_ERROR;
            pr.error = cause;
            return (&raw mut pr);
        }
        cmd_parse_build_commands(cmds, pi, &raw mut pr);
        cmd_parse_free_commands(cmds);
        &raw mut pr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_from_string(
    s: *const c_char,
    mut pi: *mut cmd_parse_input,
) -> *mut cmd_parse_result {
    unsafe {
        let mut input = MaybeUninit::<cmd_parse_input>::uninit();
        let input = input.as_mut_ptr();

        if (pi.is_null()) {
            memset0(input);
            pi = input;
        }

        (*pi).flags |= CMD_PARSE_ONEGROUP;
        cmd_parse_from_buffer(s.cast(), strlen(s), pi)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_and_insert(
    s: *mut c_char,
    pi: *mut cmd_parse_input,
    after: *mut cmdq_item,
    state: *mut cmdq_state,
    error: *mut *mut c_char,
) -> cmd_parse_status {
    unsafe {
        let pr = cmd_parse_from_string(s, pi);
        match ((*pr).status) {
            cmd_parse_status::CMD_PARSE_ERROR => {
                if (!error.is_null()) {
                    *error = (*pr).error;
                } else {
                    free_((*pr).error);
                }
            }
            cmd_parse_status::CMD_PARSE_SUCCESS => {
                let item = cmdq_get_command((*pr).cmdlist, state);
                cmdq_insert_after(after, item);
                cmd_list_free((*pr).cmdlist);
            }
        }
        (*pr).status
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_and_append(
    s: *mut c_char,
    pi: *mut cmd_parse_input,
    c: *mut client,
    state: *mut cmdq_state,
    error: *mut *mut c_char,
) -> cmd_parse_status {
    unsafe {
        let pr = cmd_parse_from_string(s, pi);
        match ((*pr).status) {
            cmd_parse_status::CMD_PARSE_ERROR => {
                if (!error.is_null()) {
                    *error = (*pr).error;
                } else {
                    free_((*pr).error);
                }
            }
            cmd_parse_status::CMD_PARSE_SUCCESS => {
                let item = cmdq_get_command((*pr).cmdlist, state);
                cmdq_append(c, item);
                cmd_list_free((*pr).cmdlist);
            }
        }
        (*pr).status
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_from_buffer(
    buf: *const c_void,
    len: usize,
    mut pi: *mut cmd_parse_input,
) -> *mut cmd_parse_result {
    static mut pr: cmd_parse_result = unsafe { zeroed() };
    let mut input: cmd_parse_input;
    let mut cause = null_mut();
    unsafe {
        // struct cmd_parse_commands	*cmds;
        // char				*cause;

        if (pi.is_null()) {
            input = unsafe { zeroed() };
            pi = &raw mut input;
        }
        pr = unsafe { zeroed() };

        if (len == 0) {
            pr.status = cmd_parse_status::CMD_PARSE_SUCCESS;
            pr.cmdlist = cmd_list_new();
            return (&raw mut pr);
        }

        let cmds = cmd_parse_do_buffer(buf.cast(), len, pi, &raw mut cause);
        if (cmds.is_null()) {
            pr.status = cmd_parse_status::CMD_PARSE_ERROR;
            pr.error = cause;
            return (&raw mut pr);
        }
        cmd_parse_build_commands(cmds, pi, &raw mut pr);
        cmd_parse_free_commands(cmds);
        &raw mut pr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse_from_arguments(
    values: *mut args_value,
    count: u32,
    mut pi: *mut cmd_parse_input,
) -> *mut cmd_parse_result {
    unsafe {
        static mut pr: cmd_parse_result = unsafe { zeroed() };
        let mut input: cmd_parse_input;

        if (pi.is_null()) {
            input = zeroed();
            pi = &raw mut input;
        }
        pr = zeroed();

        let cmds = cmd_parse_new_commands();

        let mut cmd = xcalloc1::<cmd_parse_command>() as *mut cmd_parse_command;
        (*cmd).line = (*pi).line;
        tailq_init(&raw mut (*cmd).arguments);

        for i in 0..count {
            let mut end = 0;
            if ((*values.add(i as usize)).type_ == args_type::ARGS_STRING) {
                let copy = xstrdup((*values.add(i as usize)).union_.string).as_ptr();
                let mut size = strlen(copy);
                if (size != 0 && *copy.add(size - 1) == b';' as _) {
                    size -= 1;
                    *copy.add(size) = b'\0' as _;
                    if (size > 0 && *copy.add(size - 1) == b'\\' as _) {
                        *copy.add(size - 1) = b';' as _;
                    } else {
                        end = 1;
                    }
                }
                if (end == 0 || size != 0) {
                    let arg = xcalloc1::<cmd_parse_argument>() as *mut cmd_parse_argument;
                    (*arg).type_ = cmd_parse_argument_type::CMD_PARSE_STRING;
                    (*arg).string = copy;
                    tailq_insert_tail(&raw mut (*cmd).arguments, arg);
                } else {
                    free_(copy);
                }
            } else if ((*values.add(i as usize)).type_ == args_type::ARGS_COMMANDS) {
                let arg = xcalloc1::<cmd_parse_argument>() as *mut cmd_parse_argument;
                (*arg).type_ = cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS;
                (*arg).cmdlist = (*values.add(i as usize)).union_.cmdlist;
                (*(*arg).cmdlist).references += 1;
                tailq_insert_tail(&raw mut (*cmd).arguments, arg);
            } else {
                fatalx(c"unknown argument type".as_ptr());
            }
            if (end != 0) {
                tailq_insert_tail(cmds, cmd);
                cmd = xcalloc1::<cmd_parse_command>();
                (*cmd).line = (*pi).line;
                tailq_init(&raw mut (*cmd).arguments);
            }
        }
        if (!tailq_empty(&raw mut (*cmd).arguments)) {
            tailq_insert_tail(cmds, cmd);
        } else {
            free_(cmd);
        }

        cmd_parse_build_commands(cmds, pi, &raw mut pr);
        cmd_parse_free_commands(cmds);
        &raw mut pr
    }
}
