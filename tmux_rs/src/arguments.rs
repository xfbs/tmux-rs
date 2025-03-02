use crate::*;

use compat_rs::{
    queue::{tailq_init, tailq_insert_tail},
    tree::{rb_find, rb_init, rb_insert},
};

unsafe extern "C" {
    pub unsafe fn args_set(args: *mut args, flag: c_uchar, value: *mut args_value, flags: c_int);
    pub unsafe fn args_create() -> *mut args;
    pub unsafe fn args_parse(_: *const args_parse, _: *mut args_value, _: c_uint, _: *mut *mut c_char) -> *mut args;
    pub unsafe fn args_copy(_: *mut args, _: c_int, _: *mut *mut c_char) -> *mut args;
    pub unsafe fn args_to_vector(_: *mut args, _: *mut c_int, _: *mut *mut *mut c_char);
    pub unsafe fn args_from_vector(_: c_int, _: *mut *mut c_char) -> *mut args_value;
    pub unsafe fn args_free_value(_: *mut args_value);
    pub unsafe fn args_free_values(_: *mut args_value, _: c_uint);
    pub unsafe fn args_free(_: *mut args);
    pub unsafe fn args_print(_: *mut args) -> *mut c_char;
    pub unsafe fn args_escape(_: *const c_char) -> *mut c_char;
    pub unsafe fn args_has(_: *mut args, _: c_uchar) -> c_int;
    pub unsafe fn args_get(_: *mut args, _: c_uchar) -> *const c_char;
    pub unsafe fn args_first(_: *mut args, _: *mut *mut args_entry) -> c_uchar;
    pub unsafe fn args_next(_: *mut *mut args_entry) -> c_uchar;
    pub unsafe fn args_count(_: *mut args) -> c_uint;
    pub unsafe fn args_values(_: *mut args) -> *mut args_value;
    pub unsafe fn args_value(_: *mut args, _: c_uint) -> *mut args_value;
    pub unsafe fn args_string(_: *mut args, _: c_uint) -> *mut c_char;
    pub unsafe fn args_make_commands_now(_: *mut cmd, _: *mut cmdq_item, _: c_uint, _: c_int) -> *mut cmd_list;
    pub unsafe fn args_make_commands_prepare(
        _: *mut cmd,
        _: *mut cmdq_item,
        _: c_uint,
        _: *const c_char,
        _: c_int,
        _: c_int,
    ) -> *mut args_command_state;
    pub unsafe fn args_make_commands(
        _: *mut args_command_state,
        _: c_int,
        _: *mut *mut c_char,
        _: *mut *mut c_char,
    ) -> *mut cmd_list;
    pub unsafe fn args_make_commands_free(_: *mut args_command_state);
    pub unsafe fn args_make_commands_get_command(_: *mut args_command_state) -> *mut c_char;
    pub unsafe fn args_first_value(_: *mut args, _: c_uchar) -> *mut args_value;
    pub unsafe fn args_next_value(_: *mut args_value) -> *mut args_value;
    pub unsafe fn args_strtonum(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub unsafe fn args_strtonum_and_expand(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub unsafe fn args_percentage(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub unsafe fn args_string_percentage(
        _: *const c_char,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub unsafe fn args_percentage_and_expand(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub unsafe fn args_string_percentage_and_expand(
        _: *const c_char,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
}

type args_values = tailq_head<args_value>;

const ARGS_ENTRY_OPTIONAL_VALUE: c_int = 1;
#[repr(C)]
pub struct args_entry {
    pub flag: c_uchar,
    pub values: args_values,
    pub count: c_uint,

    pub flags: c_int,

    pub entry: rb_entry<args_entry>,
}

unsafe extern "C" {
    fn args_cmp(a1: *const args_entry, a2: *const args_entry) -> i32;
}
RB_GENERATE!(args_tree, args_entry, entry, args_cmp);

#[repr(C)]
pub struct args {
    pub tree: args_tree,
    pub count: u32,
    pub values: *mut args_value,
}

#[repr(C)]
pub struct args_command_state {
    pub cmdlist: *mut cmd_list,
    pub cmd: *mut c_char,
    pub pi: cmd_parse_input,
}

/*
#[unsafe(no_mangle)]
unsafe extern "C" fn args_cmp(a1: *const args_entry, a2: *const args_entry) -> i32 {
    unsafe { ((*a1).flag as i32).wrapping_sub((*a2).flag as i32) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_find(args: *mut args, flag: c_uchar) -> *mut args_entry {
    unsafe {
        let mut entry = args_entry {
            flag,
            values: zeroed(), // TODO can be uninit
            count: 0,
            flags: 0,
            entry: zeroed(), // TODO can be uninit
        };

        rb_find(&raw mut (*args).tree, &raw mut entry)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_copy_value(to: *mut args_value, from: *const args_value) {
    unsafe {
        (*to).type_ = (*from).type_;
        match ((*from).type_) {
            args_type::ARGS_NONE => (),
            args_type::ARGS_COMMANDS => {
                (*to).union_.cmdlist = (*from).union_.cmdlist;
                (*(*to).union_.cmdlist).references += 1;
            }
            args_type::ARGS_STRING => {
                (*to).union_.string = xstrdup((*from).union_.string).cast().as_ptr();
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn args_type_to_string(type_: args_type) -> *const c_char {
    match type_ {
        args_type::ARGS_NONE => c"NONE".as_ptr(),
        args_type::ARGS_STRING => c"STRING".as_ptr(),
        args_type::ARGS_COMMANDS => c"COMMANDS".as_ptr(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_value_as_string(value: *mut args_value) -> *const c_char {
    unsafe {
        match (*value).type_ {
            args_type::ARGS_NONE => c"".as_ptr(),
            args_type::ARGS_STRING => (*value).union_.string,
            args_type::ARGS_COMMANDS => {
                if (*value).cached.is_null() {
                    (*value).cached = cmd_list_print((*value).union_.cmdlist, 0);
                }
                (*value).cached
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_create() -> *mut args {
    unsafe {
        let mut args: *mut args = xcalloc(1, size_of::<args>()).cast().as_ptr();
        rb_init(&raw mut (*args).tree);
        args
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_parse_flag_argument(
    values: *mut args_value,
    count: u32,
    cause: *mut *mut c_char,
    args: *mut args,
    i: *mut u32,
    string: *mut c_char,
    flag: i32,
    optional_argument: i32,
) -> i32 {
    let mut argument: *mut args_value;
    let mut new: *mut args_value;
    let mut s: *mut c_char;
    unsafe {
        'out: {
            new = xcalloc(1, size_of::<args_value>()).cast().as_ptr();

            if *string != b'\0' as c_char {
                (*new).type_ = args_type::ARGS_STRING;
                (*new).union_.string = xstrdup(string).cast().as_ptr();
                // goto out;
                break 'out;
            }

            if *i == count {
                argument = null_mut();
            } else {
                argument = values.add(*i as usize);
                if (*argument).type_ != args_type::ARGS_STRING {
                    xasprintf(cause, c"-%c argument must be a string".as_ptr(), flag);
                    args_free_value(new);
                    free(new as _);
                    return -1;
                }
            }

            if argument.is_null() {
                args_free_value(new);
                free(new as _);
                if optional_argument != 0 {
                    log_debug(
                        c"%s: -%c (optional)".as_ptr(),
                        c"args_parse_flag_argument".as_ptr(),
                        flag,
                    );
                    args_set(args, flag as c_uchar, null_mut(), ARGS_ENTRY_OPTIONAL_VALUE);
                    return 0; /* either - or end */
                }
                xasprintf(cause, c"-%c expects an argument".as_ptr(), flag);
                return -1;
            }

            args_copy_value(new, argument);
            (*i) += 1;

            break 'out;
        }
        // out:
        let s = args_value_as_string(new);
        log_debug(c"%s: -%c = %s".as_ptr(), c"args_parse_flag_argument".as_ptr(), flag, s);
        args_set(args, flag as c_uchar, new, 0);
    }

    0
}

// --------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_set(args: *mut args, flag: c_uchar, value: *mut args_value, flags: c_int) {
    unsafe {
        let mut entry: *mut args_entry = args_find(args, flag);

        if entry.is_null() {
            entry = xcalloc(1, size_of::<args_entry>()).cast().as_ptr();
            (*entry).flag = flag;
            (*entry).count = 1;
            (*entry).flags = flags;
            tailq_init(&raw mut (*entry).values);
            rb_insert(&raw mut (*args).tree, entry);
        } else {
            (*entry).count += 1;
        }
        if !value.is_null() && (*value).type_ != args_type::ARGS_NONE {
            tailq_insert_tail(&raw mut (*entry).values, value);
        } else {
            free(value as _);
        }
    }
}
*/
