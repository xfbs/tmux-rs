// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
#![allow(clippy::uninlined_format_args)] // for lalrpop generated code
use crate::*;

use lalrpop_util::lalrpop_mod;
use libc::_SC_MB_LEN_MAX;

use crate::compat::queue::{
    tailq_empty, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_last,
    tailq_remove,
};
use crate::xmalloc::xrecallocarray__;

unsafe fn yyparse() -> i32 {
    unsafe {
        let mut parser = cmd_parse::LinesParser::new();

        let mut ps = NonNull::new(&raw mut parse_state).unwrap();
        let mut lexer = lexer::Lexer::new(ps);

        match parser.parse(ps, lexer) {
            Ok(()) => 0,
            Err(parse_err) => {
                log_debug!("parsing error {parse_err:?}");
                1
            }
        }
    }
}

lalrpop_mod!(cmd_parse);

#[derive(Copy, Clone)]
pub struct yystype_elif {
    flag: i32,
    commands: *mut cmd_parse_commands,
}

crate::compat::impl_tailq_entry!(cmd_parse_scope, entry, tailq_entry<cmd_parse_scope>);
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

crate::compat::impl_tailq_entry!(cmd_parse_argument, entry, tailq_entry<cmd_parse_argument>);
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

crate::compat::impl_tailq_entry!(cmd_parse_command, entry, tailq_entry<cmd_parse_command>);
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

pub static mut parse_state: cmd_parse_state = unsafe { zeroed() };

pub unsafe extern "C" fn cmd_parse_get_error(
    file: *const c_char,
    line: u32,
    error: *const c_char,
) -> NonNull<c_char> {
    unsafe {
        if file.is_null() {
            xstrdup(error)
        } else {
            NonNull::new_unchecked(format_nul!("{}:{}: {}", _s(file), line, _s(error)))
        }
    }
}

pub unsafe extern "C" fn cmd_parse_print_commands(
    pi: *mut cmd_parse_input,
    cmdlist: *mut cmd_list,
) {
    unsafe {
        if (*pi).item.is_null()
            || !(*pi)
                .flags
                .intersects(cmd_parse_input_flags::CMD_PARSE_VERBOSE)
        {
            return;
        }
        let s = cmd_list_print(cmdlist, 0);
        if !(*pi).file.is_null() {
            cmdq_print!((*pi).item, "{}:{}: {}", _s((*pi).file), (*pi).line, _s(s));
        } else {
            cmdq_print!((*pi).item, "{}: {}", (*pi).line, _s(s));
        }
        free_(s);
    }
}

pub unsafe extern "C" fn cmd_parse_free_argument(arg: *mut cmd_parse_argument) {
    unsafe {
        match (*arg).type_ {
            cmd_parse_argument_type::CMD_PARSE_STRING => free_((*arg).string),
            cmd_parse_argument_type::CMD_PARSE_COMMANDS => cmd_parse_free_commands((*arg).commands),
            cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS => cmd_list_free((*arg).cmdlist),
        }
        free_(arg);
    }
}

pub unsafe extern "C" fn cmd_parse_free_arguments(args: *mut cmd_parse_arguments) {
    unsafe {
        for arg in tailq_foreach(args).map(NonNull::as_ptr) {
            tailq_remove(args, arg);
            cmd_parse_free_argument(arg);
        }
    }
}

pub unsafe extern "C" fn cmd_parse_free_command(cmd: *mut cmd_parse_command) {
    unsafe {
        cmd_parse_free_arguments(&raw mut (*cmd).arguments);
        free_(cmd);
    }
}

pub unsafe extern "C" fn cmd_parse_new_commands() -> NonNull<cmd_parse_commands> {
    unsafe {
        let cmds = xmalloc_::<cmd_parse_commands>();
        tailq_init(cmds.as_ptr());
        cmds
    }
}

pub unsafe extern "C" fn cmd_parse_free_commands(cmds: *mut cmd_parse_commands) {
    unsafe {
        for cmd in tailq_foreach(cmds).map(NonNull::as_ptr) {
            tailq_remove(cmds, cmd);
            cmd_parse_free_command(cmd);
        }
        free_(cmds);
    }
}

pub unsafe extern "C" fn cmd_parse_run_parser(cause: *mut *mut c_char) -> *mut cmd_parse_commands {
    unsafe {
        let ps = &raw mut parse_state;

        (*ps).commands = null_mut();
        tailq_init(&raw mut (*ps).stack);

        let retval = yyparse();
        for scope in tailq_foreach(&raw mut (*ps).stack).map(NonNull::as_ptr) {
            tailq_remove(&raw mut (*ps).stack, scope);
            free_(scope);
        }
        if retval != 0 {
            *cause = (*ps).error;
            return null_mut();
        }

        if (*ps).commands.is_null() {
            return cmd_parse_new_commands().as_ptr();
        }
        (*ps).commands
    }
}

pub unsafe extern "C" fn cmd_parse_do_file(
    f: *mut FILE,
    pi: *mut cmd_parse_input,
    cause: *mut *mut c_char,
) -> *mut cmd_parse_commands {
    let ps = &raw mut parse_state;
    unsafe {
        libc::memset(ps.cast(), 0, size_of::<cmd_parse_state>());
        (*ps).input = pi;
        (*ps).f = f;
        cmd_parse_run_parser(cause)
    }
}

pub unsafe extern "C" fn cmd_parse_do_buffer(
    buf: *const c_char,
    len: usize,
    pi: *mut cmd_parse_input,
    cause: *mut *mut c_char,
) -> *mut cmd_parse_commands {
    unsafe {
        let ps = &raw mut parse_state;

        libc::memset(ps.cast(), 0, size_of::<cmd_parse_state>());
        (*ps).input = pi;
        (*ps).buf = buf;
        (*ps).len = len;
        cmd_parse_run_parser(cause)
    }
}

pub unsafe extern "C" fn cmd_parse_log_commands(
    cmds: *mut cmd_parse_commands,
    prefix: *const c_char,
) {
    unsafe {
        for (i, cmd) in tailq_foreach(cmds).map(NonNull::as_ptr).enumerate() {
            for (j, arg) in tailq_foreach(&raw mut (*cmd).arguments)
                .map(NonNull::as_ptr)
                .enumerate()
            {
                match (*arg).type_ {
                    cmd_parse_argument_type::CMD_PARSE_STRING => {
                        log_debug!("{} {}:{}: {}", _s(prefix), i, j, _s((*arg).string))
                    }
                    cmd_parse_argument_type::CMD_PARSE_COMMANDS => {
                        let s = format_nul!("{} {}:{}", _s(prefix), i, j);
                        cmd_parse_log_commands((*arg).commands, s);
                        free_(s);
                    }
                    cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS => {
                        let s = cmd_list_print((*arg).cmdlist, 0);
                        log_debug!("{} {}:{}: {}", _s(prefix), i, j, _s(s));
                        free_(s);
                    }
                }
            }
        }
    }
}

pub unsafe extern "C" fn cmd_parse_expand_alias(
    cmd: *mut cmd_parse_command,
    pi: *mut cmd_parse_input,
    pr: *mut cmd_parse_result,
) -> i32 {
    let __func__ = c"cmd_parse_expand_alias".as_ptr();
    unsafe {
        if (*pi)
            .flags
            .intersects(cmd_parse_input_flags::CMD_PARSE_NOALIAS)
        {
            return 0;
        }
        libc::memset(pr.cast(), 0, size_of::<cmd_parse_result>());

        let first = tailq_first(&raw mut (*cmd).arguments);
        if first.is_null() || (*first).type_ != cmd_parse_argument_type::CMD_PARSE_STRING {
            (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
            (*pr).cmdlist = cmd_list_new();
            return 1;
        }
        let name = (*first).string;

        let alias = cmd_get_alias(name);
        if alias.is_null() {
            return 0;
        }
        log_debug!(
            "{}: {} alias {} = {}",
            _s(__func__),
            (*pi).line,
            _s(name),
            _s(alias)
        );

        let mut cause = null_mut();
        let cmds = cmd_parse_do_buffer(alias, strlen(alias), pi, &raw mut cause);
        free_(alias);
        if cmds.is_null() {
            (*pr).status = cmd_parse_status::CMD_PARSE_ERROR;
            (*pr).error = cause;
            return 1;
        }

        let last = tailq_last(cmds);
        if last.is_null() {
            (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
            (*pr).cmdlist = cmd_list_new();
            return 1;
        }

        tailq_remove(&raw mut (*cmd).arguments, first);
        cmd_parse_free_argument(first);

        for arg in tailq_foreach(&raw mut (*cmd).arguments).map(NonNull::as_ptr) {
            tailq_remove(&raw mut (*cmd).arguments, arg);
            tailq_insert_tail(&raw mut (*last).arguments, arg);
        }
        cmd_parse_log_commands(cmds, __func__);

        (*pi).flags |= cmd_parse_input_flags::CMD_PARSE_NOALIAS;
        cmd_parse_build_commands(cmds, pi, pr);
        (*pi).flags &= !cmd_parse_input_flags::CMD_PARSE_NOALIAS;
        1
    }
}

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
        let idx = 0u32;
        libc::memset(pr.cast(), 0, size_of::<cmd_parse_result>());

        if cmd_parse_expand_alias(cmd, pi, pr) != 0 {
            return;
        }

        'out: {
            for arg in tailq_foreach(&raw mut (*cmd).arguments).map(NonNull::as_ptr) {
                values = xrecallocarray__::<args_value>(values, count as usize, count as usize + 1)
                    .as_ptr();
                match (*arg).type_ {
                    cmd_parse_argument_type::CMD_PARSE_STRING => {
                        (*values.add(count as usize)).type_ = args_type::ARGS_STRING;
                        (*values.add(count as usize)).union_.string =
                            xstrdup((*arg).string).as_ptr();
                    }
                    cmd_parse_argument_type::CMD_PARSE_COMMANDS => {
                        cmd_parse_build_commands((*arg).commands, pi, pr);
                        if (*pr).status != cmd_parse_status::CMD_PARSE_SUCCESS {
                            break 'out;
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
            }

            let add = cmd_parse(values, count, (*pi).file, (*pi).line, &raw mut cause);
            if add.is_null() {
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
        if tailq_empty(cmds) {
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
        for cmd in tailq_foreach(cmds).map(NonNull::as_ptr) {
            if !(*pi)
                .flags
                .intersects(cmd_parse_input_flags::CMD_PARSE_ONEGROUP)
                && (*cmd).line != line
            {
                if !current.is_null() {
                    cmd_parse_print_commands(pi, current);
                    cmd_list_move(result, current);
                    cmd_list_free(current);
                }
                current = cmd_list_new();
            }
            if current.is_null() {
                current = cmd_list_new();
            }
            line = (*cmd).line;
            line = (*pi).line;

            cmd_parse_build_command(cmd, pi, pr);
            if (*pr).status != cmd_parse_status::CMD_PARSE_SUCCESS {
                cmd_list_free(result);
                cmd_list_free(current);
                return;
            }
            cmd_list_append_all(current, (*pr).cmdlist);
            cmd_list_free((*pr).cmdlist);
        }

        if !current.is_null() {
            cmd_parse_print_commands(pi, current);
            cmd_list_move(result, current);
            cmd_list_free(current);
        }

        let s = cmd_list_print(result, 0);
        log_debug!("{}: {}", _s(__func__), _s(s));
        free_(s);

        (*pr).status = cmd_parse_status::CMD_PARSE_SUCCESS;
        (*pr).cmdlist = result;
    }
}

pub unsafe extern "C" fn cmd_parse_from_file(
    f: *mut FILE,
    mut pi: *mut cmd_parse_input,
) -> *mut cmd_parse_result {
    unsafe {
        static mut pr: cmd_parse_result = unsafe { zeroed() };
        let mut input: cmd_parse_input = zeroed();
        let mut cause = null_mut();

        if pi.is_null() {
            input = zeroed();
            pi = &raw mut input;
        }
        pr = zeroed();

        let cmds = cmd_parse_do_file(f, pi, &raw mut cause);
        if cmds.is_null() {
            pr.status = cmd_parse_status::CMD_PARSE_ERROR;
            pr.error = cause;
            return (&raw mut pr);
        }
        cmd_parse_build_commands(cmds, pi, &raw mut pr);
        cmd_parse_free_commands(cmds);
        &raw mut pr
    }
}

pub unsafe extern "C" fn cmd_parse_from_string(
    s: *const c_char,
    mut pi: *mut cmd_parse_input,
) -> *mut cmd_parse_result {
    unsafe {
        let mut input = MaybeUninit::<cmd_parse_input>::uninit();
        let input = input.as_mut_ptr();

        if pi.is_null() {
            memset0(input);
            pi = input;
        }

        (*pi).flags |= cmd_parse_input_flags::CMD_PARSE_ONEGROUP;
        cmd_parse_from_buffer(s.cast(), strlen(s), pi)
    }
}

pub unsafe extern "C" fn cmd_parse_and_insert(
    s: *mut c_char,
    pi: *mut cmd_parse_input,
    after: *mut cmdq_item,
    state: *mut cmdq_state,
    error: *mut *mut c_char,
) -> cmd_parse_status {
    unsafe {
        let pr = cmd_parse_from_string(s, pi);
        match (*pr).status {
            cmd_parse_status::CMD_PARSE_ERROR => {
                if !error.is_null() {
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

pub unsafe extern "C" fn cmd_parse_and_append(
    s: *mut c_char,
    pi: *mut cmd_parse_input,
    c: *mut client,
    state: *mut cmdq_state,
    error: *mut *mut c_char,
) -> cmd_parse_status {
    unsafe {
        let pr = cmd_parse_from_string(s, pi);
        match (*pr).status {
            cmd_parse_status::CMD_PARSE_ERROR => {
                if !error.is_null() {
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

        if pi.is_null() {
            input = unsafe { zeroed() };
            pi = &raw mut input;
        }
        pr = unsafe { zeroed() };

        if len == 0 {
            pr.status = cmd_parse_status::CMD_PARSE_SUCCESS;
            pr.cmdlist = cmd_list_new();
            return (&raw mut pr);
        }

        let cmds = cmd_parse_do_buffer(buf.cast(), len, pi, &raw mut cause);
        if cmds.is_null() {
            pr.status = cmd_parse_status::CMD_PARSE_ERROR;
            pr.error = cause;
            return (&raw mut pr);
        }
        cmd_parse_build_commands(cmds, pi, &raw mut pr);
        cmd_parse_free_commands(cmds);
        &raw mut pr
    }
}

pub unsafe extern "C" fn cmd_parse_from_arguments(
    values: *mut args_value,
    count: u32,
    mut pi: *mut cmd_parse_input,
) -> *mut cmd_parse_result {
    unsafe {
        static mut pr: cmd_parse_result = unsafe { zeroed() };
        let mut input: cmd_parse_input;

        if pi.is_null() {
            input = zeroed();
            pi = &raw mut input;
        }
        pr = zeroed();

        let cmds = cmd_parse_new_commands().as_ptr();

        let mut cmd = xcalloc1::<cmd_parse_command>() as *mut cmd_parse_command;
        (*cmd).line = (*pi).line;
        tailq_init(&raw mut (*cmd).arguments);

        for i in 0..count {
            let mut end = 0;
            if (*values.add(i as usize)).type_ == args_type::ARGS_STRING {
                let copy = xstrdup((*values.add(i as usize)).union_.string).as_ptr();
                let mut size = strlen(copy);
                if size != 0 && *copy.add(size - 1) == b';' as _ {
                    size -= 1;
                    *copy.add(size) = b'\0' as _;
                    if size > 0 && *copy.add(size - 1) == b'\\' as _ {
                        *copy.add(size - 1) = b';' as _;
                    } else {
                        end = 1;
                    }
                }
                if end == 0 || size != 0 {
                    let arg = xcalloc1::<cmd_parse_argument>() as *mut cmd_parse_argument;
                    (*arg).type_ = cmd_parse_argument_type::CMD_PARSE_STRING;
                    (*arg).string = copy;
                    tailq_insert_tail(&raw mut (*cmd).arguments, arg);
                } else {
                    free_(copy);
                }
            } else if (*values.add(i as usize)).type_ == args_type::ARGS_COMMANDS {
                let arg = xcalloc1::<cmd_parse_argument>() as *mut cmd_parse_argument;
                (*arg).type_ = cmd_parse_argument_type::CMD_PARSE_PARSED_COMMANDS;
                (*arg).cmdlist = (*values.add(i as usize)).union_.cmdlist;
                (*(*arg).cmdlist).references += 1;
                tailq_insert_tail(&raw mut (*cmd).arguments, arg);
            } else {
                fatalx(c"unknown argument type");
            }
            if end != 0 {
                tailq_insert_tail(cmds, cmd);
                cmd = xcalloc1::<cmd_parse_command>();
                (*cmd).line = (*pi).line;
                tailq_init(&raw mut (*cmd).arguments);
            }
        }
        if !tailq_empty(&raw mut (*cmd).arguments) {
            tailq_insert_tail(cmds, cmd);
        } else {
            free_(cmd);
        }

        cmd_parse_build_commands(cmds, pi, &raw mut pr);
        cmd_parse_free_commands(cmds);
        &raw mut pr
    }
}

mod lexer {
    use crate::{cmd_parse_state, transmute_ptr};
    use core::ffi::c_char;
    use core::ptr::NonNull;

    pub struct Lexer {
        ps: NonNull<cmd_parse_state>,
    }
    impl Lexer {
        pub fn new(ps: NonNull<cmd_parse_state>) -> Self {
            Lexer { ps }
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub enum Tok {
        Zero, // invalid
        Newline,
        Semicolon,
        LeftBrace,
        RightBrace,

        Error,
        Hidden,
        If,
        Else,
        Elif,
        Endif,

        Format(Option<NonNull<c_char>>),
        Token(Option<NonNull<c_char>>),
        Equals(Option<NonNull<c_char>>),
    }
    impl std::fmt::Display for Tok {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Tok::Zero => write!(f, "zero"),
                Tok::Newline => write!(f, "\\n"),
                Tok::Semicolon => write!(f, ";"),
                Tok::LeftBrace => write!(f, "{{"),
                Tok::RightBrace => write!(f, "}}"),
                Tok::Error => write!(f, "%error"),
                Tok::Hidden => write!(f, "%hidden"),
                Tok::If => write!(f, "%if"),
                Tok::Else => write!(f, "%else"),
                Tok::Elif => write!(f, "%elif"),
                Tok::Endif => write!(f, "%endif"),
                Tok::Format(non_null) => {
                    write!(f, "format({})", crate::_s(transmute_ptr(*non_null)))
                }
                Tok::Token(non_null) => write!(f, "token({})", crate::_s(transmute_ptr(*non_null))),
                Tok::Equals(non_null) => {
                    write!(f, "equals({})", crate::_s(transmute_ptr(*non_null)))
                }
            }
        }
    }

    #[derive(Debug)]
    pub enum LexicalError {
        // Not possible
    }
    type Loc = usize;
    impl Iterator for Lexer {
        type Item = Result<(Loc, Tok, Loc), LexicalError>;

        fn next(&mut self) -> Option<Result<(Loc, Tok, Loc), LexicalError>> {
            unsafe { super::yylex_(self.ps.as_ptr()).map(|tok| Ok((0, tok, 0))) }
        }
    }
}

macro_rules! yyerror {
   ($ps:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_parse::yyerror_($ps, format_args!($fmt $(, $args)*))
    };
}
unsafe fn yyerror_(ps: *mut cmd_parse_state, args: std::fmt::Arguments) -> i32 {
    unsafe {
        let mut pi: *mut cmd_parse_input = (*ps).input;

        if !(*ps).error.is_null() {
            return 0;
        }

        let mut error = args.to_string();
        error.push('\0');

        (*ps).error = cmd_parse_get_error((*pi).file, (*pi).line, error.as_ptr().cast()).as_ptr();
        0
    }
}

fn yylex_is_var(ch: c_char, first: bool) -> bool {
    if ch == b'=' as i8 || (first && (ch as u8).is_ascii_digit()) {
        false
    } else {
        (ch as u8).is_ascii_alphanumeric() || ch == b'_' as i8
    }
}

unsafe fn yylex_append(buf: *mut *mut c_char, len: *mut usize, add: *const c_char, addlen: usize) {
    unsafe {
        if (addlen > usize::MAX - 1 || *len > usize::MAX - 1 - addlen) {
            fatalx(c"buffer is too big");
        }
        *buf = xrealloc_(*buf, (*len) + 1 + addlen).as_ptr();
        libc::memcpy((*buf).add(*len).cast(), add.cast(), addlen);
        (*len) += addlen;
    }
}

unsafe fn yylex_append1(buf: *mut *mut c_char, len: *mut usize, add: c_char) {
    unsafe {
        yylex_append(buf, len, &raw const add, 1);
    }
}

unsafe fn yylex_getc1(ps: *mut cmd_parse_state) -> i32 {
    let ch;
    unsafe {
        if !(*ps).f.is_null() {
            ch = libc::fgetc((*ps).f);
        } else if (*ps).off == (*ps).len {
            ch = libc::EOF;
        } else {
            ch = *(*ps).buf.add((*ps).off) as i32;
            (*ps).off += 1;
        }
    }
    ch
}

unsafe fn yylex_ungetc(ps: *mut cmd_parse_state, ch: i32) {
    unsafe {
        if !(*ps).f.is_null() {
            libc::ungetc(ch, (*ps).f);
        } else if (*ps).off > 0 && ch != libc::EOF {
            (*ps).off -= 1;
        }
    }
}

unsafe fn yylex_getc(ps: *mut cmd_parse_state) -> i32 {
    unsafe {
        if (*ps).escapes != 0 {
            (*ps).escapes -= 1;
            return '\\' as i32;
        }
        loop {
            let ch = yylex_getc1(ps);
            if ch == '\\' as i32 {
                (*ps).escapes += 1;
                continue;
            }
            if ch == '\n' as i32 && (*ps).escapes % 2 == 1 {
                (*(*ps).input).line += 1;
                (*ps).escapes -= 1;
                continue;
            }

            if ((*ps).escapes != 0) {
                yylex_ungetc(ps, ch);
                (*ps).escapes -= 1;
                return '\\' as i32;
            }
            return ch;
        }
    }
}

unsafe fn yylex_get_word(ps: *mut cmd_parse_state, mut ch: i32) -> *mut c_char {
    unsafe {
        let mut len = 0;
        let mut buf: *mut i8 = xmalloc(1).cast().as_ptr();

        loop {
            yylex_append1(&raw mut buf, &raw mut len, ch as i8);
            ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c" \t\n".as_ptr(), ch).is_null() {
                break;
            }
        }
        yylex_ungetc(ps, ch);

        *buf.add(len) = b'\0' as i8;
        // log_debug("%s: %s", __func__, buf);
        buf
    }
}

use lexer::Tok;

unsafe fn yylex_(ps: *mut cmd_parse_state) -> Option<Tok> {
    unsafe {
        let mut next: i32 = 0;

        if ((*ps).eol != 0) {
            (*(*ps).input).line += 1;
        }
        (*ps).eol = 0;

        let mut condition = (*ps).condition;
        (*ps).condition = 0;

        loop {
            let mut ch = yylex_getc(ps);

            if ch == libc::EOF {
                /*
                 * Ensure every file or string is terminated by a
                 * newline. This keeps the parser simpler and avoids
                 * having to add a newline to each string.
                 */
                if (*ps).eof != 0 {
                    break;
                }
                (*ps).eof = 1;
                return Some(Tok::Newline);
            }

            if (ch == ' ' as i32 || ch == '\t' as i32) {
                /*
                 * Ignore whitespace.
                 */
                continue;
            }

            if (ch == '\r' as i32) {
                /*
                 * Treat \r\n as \n.
                 */
                ch = yylex_getc(ps);
                if (ch != '\n' as i32) {
                    yylex_ungetc(ps, ch);
                    ch = '\r' as i32;
                }
            }
            if (ch == '\n' as i32) {
                /*
                 * End of line. Update the line number.
                 */
                (*ps).eol = 1;
                return Some(Tok::Newline);
            }

            if ch == ';' as i32 {
                return Some(Tok::Semicolon);
            }
            if ch == '{' as i32 {
                return Some(Tok::LeftBrace);
            }
            if ch == '}' as i32 {
                return Some(Tok::RightBrace);
            }

            if (ch == '#' as i32) {
                /*
                 * #{ after a condition opens a format; anything else
                 * is a comment, ignore up to the end of the line.
                 */
                next = yylex_getc(ps);
                if (condition != 0 && next == '{' as i32) {
                    let yylval_token = yylex_format(ps);
                    if yylval_token.is_none() {
                        return Some(Tok::Error);
                    }
                    return Some(Tok::Format(yylval_token));
                }
                while (next != '\n' as i32 && next != libc::EOF) {
                    next = yylex_getc(ps);
                }
                if next == '\n' as i32 {
                    (*(*ps).input).line += 1;
                    return Some(Tok::Newline);
                }
                continue;
            }

            if ch == '%' as i32 {
                /*
                 * % is a condition unless it is all % or all numbers,
                 * then it is a token.
                 */
                let yylval_token = yylex_get_word(ps, '%' as i32);
                let mut cp = yylval_token;
                while *cp != b'\0' as i8 {
                    if *cp != b'%' as i8 && !(*cp as u8).is_ascii_digit() {
                        break;
                    }
                    cp = cp.add(1);
                }
                if (*cp == b'\0' as i8) {
                    return Some(Tok::Token(NonNull::new(yylval_token)));
                }
                (*ps).condition = 1;
                if streq_(yylval_token, "%hidden") {
                    free_(yylval_token);
                    return Some(Tok::Hidden);
                }
                if streq_(yylval_token, "%if") {
                    free_(yylval_token);
                    return Some(Tok::If);
                }
                if streq_(yylval_token, "%else") {
                    free_(yylval_token);
                    return Some(Tok::Else);
                }
                if streq_(yylval_token, "%elif") {
                    free_(yylval_token);
                    return Some(Tok::Elif);
                }
                if streq_(yylval_token, "%endif") {
                    free_(yylval_token);
                    return Some(Tok::Endif);
                }
                free_(yylval_token);
                return Some(Tok::Error);
            }

            /*
             * Otherwise this is a token.
             */
            let token = yylex_token(ps, ch);
            if token.is_null() {
                return Some(Tok::Error);
            }
            let yylval_token = token;

            if !libc::strchr(token, b'=' as i32).is_null() && yylex_is_var(*token, true) {
                let mut cp = token.add(1);
                while *cp != '=' as i8 {
                    if !yylex_is_var(*cp, false) {
                        break;
                    }
                    cp = cp.add(1);
                }
                if *cp == b'=' as i8 {
                    return Some(Tok::Equals(NonNull::new(yylval_token)));
                }
            }
            return Some(Tok::Token(NonNull::new(yylval_token)));
        }

        None
    }
}

unsafe fn yylex_format(ps: *mut cmd_parse_state) -> Option<NonNull<c_char>> {
    unsafe {
        let mut brackets = 1;
        let mut len = 0;
        let mut buf = xmalloc_::<c_char>().as_ptr();

        'error: {
            yylex_append(&raw mut buf, &raw mut len, c"#{".as_ptr(), 2);
            loop {
                let mut ch = yylex_getc(ps);
                if (ch == libc::EOF || ch == '\n' as i32) {
                    break 'error;
                }
                if (ch == '#' as i32) {
                    ch = yylex_getc(ps);
                    if (ch == libc::EOF || ch == '\n' as i32) {
                        break 'error;
                    }
                    if ch == '{' as i32 {
                        brackets += 1;
                    }
                    yylex_append1(&raw mut buf, &raw mut len, b'#' as c_char);
                } else if (ch == '}' as i32)
                    && brackets != 0
                    && ({
                        brackets -= 1;
                        brackets == 0
                    })
                {
                    yylex_append1(&raw mut buf, &raw mut len, ch as c_char);
                    break;
                }
                yylex_append1(&raw mut buf, &raw mut len, ch as c_char);
            }
            if (brackets != 0) {
                break 'error;
            }

            *buf.add(len) = b'\0' as i8;
            // log_debug("%s: %s", __func__, buf);
            return NonNull::new(buf);
        } // error:

        free_(buf);
        None
    }
}

unsafe fn yylex_token_variable(
    ps: *mut cmd_parse_state,
    buf: *mut *mut c_char,
    len: *mut usize,
) -> bool {
    unsafe {
        // struct environ_entry	*envent;
        // int			 ch, brackets = 0;
        // char			 name[1024];
        // size_t			 namelen = 0;
        // const char		*value;

        let mut namelen: usize = 0;
        let mut name: [c_char; 1024] = [0; 1024];
        const sizeof_name: usize = 1024;
        let mut brackets = 0;

        let mut ch = yylex_getc(ps);
        if (ch == libc::EOF) {
            return false;
        }
        if (ch == '{' as i32) {
            brackets = 1;
        } else {
            if !yylex_is_var(ch as c_char, true) {
                yylex_append1(buf, len, b'$' as i8);
                yylex_ungetc(ps, ch);
                return true;
            }
            name[namelen] = ch as i8;
            namelen += 1;
        }

        loop {
            ch = yylex_getc(ps);
            if (brackets != 0 && ch == '}' as i32) {
                break;
            }
            if (ch == libc::EOF || !yylex_is_var(ch as c_char, false)) {
                if brackets == 0 {
                    yylex_ungetc(ps, ch);
                    break;
                }
                yyerror!(ps, "invalid environment variable");
                return false;
            }
            if namelen == sizeof_name - 2 {
                yyerror!(ps, "environment variable is too long");
                return false;
            }
            name[namelen] = ch as i8;
            namelen += 1;
        }
        name[namelen] = b'\0' as i8;

        let mut envent = environ_find(global_environ, (&raw const name).cast());
        if !envent.is_null() && (*envent).value.is_some() {
            let value = (*envent).value;
            // log_debug("%s: %s -> %s", __func__, name, value);
            yylex_append(
                buf,
                len,
                transmute_ptr(value),
                libc::strlen(transmute_ptr(value)),
            );
        }
        true
    }
}

unsafe fn yylex_token_tilde(
    ps: *mut cmd_parse_state,
    buf: *mut *mut c_char,
    len: *mut usize,
) -> bool {
    unsafe {
        let mut home = null();
        let mut namelen: usize = 0;
        let mut name: [c_char; 1024] = [0; 1024];
        const sizeof_name: usize = 1024;

        loop {
            let ch = yylex_getc(ps);
            if ch == libc::EOF || !libc::strchr(c"/ \t\n\"'".as_ptr(), ch).is_null() {
                yylex_ungetc(ps, ch);
                break;
            }
            if namelen == sizeof_name - 2 {
                yyerror!(ps, "user name is too long");
                return false;
            }
            name[namelen] = ch as i8;
            namelen += 1;
        }
        name[namelen] = b'\0' as i8;

        if name[0] == b'\0' as i8 {
            let envent = environ_find(global_environ, c"HOME".as_ptr());
            if (!envent.is_null() && (*(*envent).value.unwrap().as_ptr()) != b'\0' as i8) {
                home = transmute_ptr((*envent).value);
            } else if let Some(pw) = NonNull::new(libc::getpwuid(libc::getuid())) {
                home = (*pw.as_ptr()).pw_dir;
            }
        } else if let Some(pw) = NonNull::new(libc::getpwnam((&raw const name) as *const i8)) {
            home = (*pw.as_ptr()).pw_dir;
        }
        if home.is_null() {
            return false;
        }

        // log_debug("%s: ~%s -> %s", __func__, name, home);
        yylex_append(buf, len, home, strlen(home));
        true
    }
}

unsafe fn yylex_token(ps: *mut cmd_parse_state, mut ch: i32) -> *mut c_char {
    unsafe {
        #[derive(Copy, Clone, Eq, PartialEq)]
        enum State {
            Start,
            None,
            DoubleQuotes,
            SingleQuotes,
        }

        let mut state = State::None;
        let mut last = State::Start;

        let mut len = 0;
        let mut buf = xmalloc_::<c_char>().as_ptr();

        'error: {
            'aloop: loop {
                'next: {
                    'skip: {
                        /* EOF or \n are always the end of the token. */
                        if (ch == libc::EOF) {
                            // log_debug("%s: end at EOF", __func__);
                            break 'aloop;
                        }
                        if (state == State::None && ch == '\r' as i32) {
                            ch = yylex_getc(ps);
                            if (ch != '\n' as i32) {
                                yylex_ungetc(ps, ch);
                                ch = '\r' as i32;
                            }
                        }
                        if (state == State::None && ch == '\n' as i32) {
                            // log_debug("%s: end at EOL", __func__);
                            break 'aloop;
                        }

                        /* Whitespace or ; or } ends a token unless inside quotes. */
                        if state == State::None && (ch == ' ' as i32 || ch == '\t' as i32) {
                            // log_debug("%s: end at WS", __func__);
                            break 'aloop;
                        }
                        if (state == State::None && (ch == ';' as i32 || ch == '}' as i32)) {
                            // log_debug("%s: end at %c", __func__, ch);
                            break 'aloop;
                        }

                        /*
                         * Spaces and comments inside quotes after \n are removed but
                         * the \n is left.
                         */
                        if (ch == '\n' as i32 && state != State::None) {
                            yylex_append1(&raw mut buf, &raw mut len, b'\n' as i8);
                            while ({
                                ch = yylex_getc(ps);
                                ch == b' ' as i32
                            }) || ch == '\t' as i32
                            {}
                            if (ch != '#' as i32) {
                                continue 'aloop;
                            }
                            ch = yylex_getc(ps);
                            if !libc::strchr(c",#{}:".as_ptr(), ch).is_null() {
                                yylex_ungetc(ps, ch);
                                ch = '#' as i32;
                            } else {
                                while ({
                                    ch = yylex_getc(ps);
                                    ch != '\n' as i32 && ch != libc::EOF
                                }) { /* nothing */ }
                            }
                            continue 'aloop;
                        }

                        /* \ ~ and $ are expanded except in single quotes. */
                        if ch == '\\' as i32 && state != State::SingleQuotes {
                            if !yylex_token_escape(ps, &raw mut buf, &raw mut len) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '~' as i32 && last != state && state != State::SingleQuotes {
                            if !yylex_token_tilde(ps, &raw mut buf, &raw mut len) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '$' as i32 && state != State::SingleQuotes {
                            if !yylex_token_variable(ps, &raw mut buf, &raw mut len) {
                                break 'error;
                            }
                            break 'skip;
                        }
                        if ch == '}' as i32 && state == State::None {
                            break 'error; /* unmatched (matched ones were handled) */
                        }

                        /* ' and " starts or end quotes (and is consumed). */
                        if ch == '\'' as i32 {
                            if (state == State::None) {
                                state = State::SingleQuotes;
                                break 'next;
                            }
                            if (state == State::SingleQuotes) {
                                state = State::None;
                                break 'next;
                            }
                        }
                        if ch == b'"' as i32 {
                            if (state == State::None) {
                                state = State::DoubleQuotes;
                                break 'next;
                            }
                            if (state == State::DoubleQuotes) {
                                state = State::None;
                                break 'next;
                            }
                        }

                        /* Otherwise add the character to the buffer. */
                        yylex_append1(&raw mut buf, &raw mut len, ch as c_char);
                    }
                    // skip:
                    last = state;
                }
                // next:
                ch = yylex_getc(ps);
            }
            yylex_ungetc(ps, ch);

            *buf.add(len) = b'\0' as i8;
            // log_debug("%s: %s", __func__, buf);
            return (buf);
        } // error:
        free_(buf);

        null_mut()
    }
}

unsafe fn yylex_token_escape(
    ps: *mut cmd_parse_state,
    buf: *mut *mut c_char,
    len: *mut usize,
) -> bool {
    unsafe {
        const sizeof_m: usize = libc::_SC_MB_LEN_MAX as usize;

        let mut tmp: u32 = 0;
        let mut s: [c_char; 9] = [0; 9];
        let mut m: [c_char; libc::_SC_MB_LEN_MAX as usize] = [0; libc::_SC_MB_LEN_MAX as usize];
        let mut size: usize = 0;
        let mut type_: i32 = 0;

        'unicode: {
            let mut ch = yylex_getc(ps);

            if (ch >= '4' as i32 && ch <= '7' as i32) {
                yyerror!(ps, "invalid octal escape");
                return false;
            }
            if (ch >= '0' as i32 && ch <= '3' as i32) {
                let o2 = yylex_getc(ps);
                if (o2 >= '0' as i32 && o2 <= '7' as i32) {
                    let o3 = yylex_getc(ps);
                    if (o3 >= '0' as i32 && o3 <= '7' as i32) {
                        ch = 64 * (ch - '0' as i32) + 8 * (o2 - '0' as i32) + (o3 - '0' as i32);
                        yylex_append1(buf, len, ch as i8);
                        return true;
                    }
                }
                yyerror!(ps, "invalid octal escape");
                return false;
            }

            if ch == libc::EOF {
                return false;
            }

            match ch as u8 as char {
                'a' => ch = '\x07' as i32,
                'b' => ch = '\x08' as i32,
                'e' => ch = '\x1B' as i32,
                'f' => ch = '\x0C' as i32,
                's' => ch = ' ' as i32,
                'v' => ch = '\x0B' as i32,
                'r' => ch = '\r' as i32,
                'n' => ch = '\n' as i32,
                't' => ch = '\t' as i32,
                'u' => {
                    type_ = 'u' as i32;
                    size = 4;
                    break 'unicode;
                }
                'U' => {
                    type_ = 'U' as i32;
                    size = 8;
                    break 'unicode;
                }
                _ => (),
            }

            yylex_append1(buf, len, ch as i8);
            return true;
        } // unicode:
        let mut i = 0;
        for i_ in 0..size {
            i = i_;
            let ch = yylex_getc(ps);
            if ch == libc::EOF || ch == '\n' as i32 {
                return false;
            }
            if !(ch as u8).is_ascii_hexdigit() {
                yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
                return false;
            }
            s[i] = ch as i8;
        }
        s[i] = b'\0' as c_char;

        if ((size == 4 && libc::sscanf((&raw mut s).cast(), c"%4x".as_ptr(), &raw mut tmp) != 1)
            || (size == 8 && libc::sscanf((&raw mut s).cast(), c"%8x".as_ptr(), &raw mut tmp) != 1))
        {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        let mlen = wctomb((&raw mut m).cast(), tmp as i32);
        if mlen <= 0 || mlen > sizeof_m as i32 {
            yyerror!(ps, "invalid \\{} argument", type_ as u8 as char);
            return false;
        }
        yylex_append(buf, len, (&raw const m).cast(), mlen as usize);

        true
    }
}

// # Notes:
//
// <https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html>
// <https://github.com/lalrpop/lalrpop/blob/master/README.md>
