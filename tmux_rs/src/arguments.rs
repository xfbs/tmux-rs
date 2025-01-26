use compat_rs::{
    queue::{tailq_init, tailq_insert_tail},
    tree::{rb_find, rb_init, rb_insert},
};

use super::*;

unsafe extern "C" {
    // pub unsafe fn args_set(args: *mut args, flag: c_uchar, value: *mut args_value, flags: c_int);
    // pub unsafe fn args_create() -> *mut args;
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
    pub unsafe fn args_string(_: *mut args, _: c_uint) -> *const c_char;
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
    flag: c_uchar,
    values: args_values,
    count: c_uint,
    flags: c_int,

    entry: rb_entry<args_entry>,
}

impl GetEntry<args_entry> for args_entry {
    unsafe fn entry_mut(this: *mut Self) -> *mut rb_entry<args_entry> {
        unsafe { &raw mut (*this).entry }
    }

    unsafe fn cmp(this: *const Self, other: *const Self) -> i32 {
        unsafe { args_cmp(this, other) }
    }
}

#[repr(C)]
pub struct args {
    tree: args_tree,
    count: u32,
    values: args_value,
}

#[repr(C)]
pub struct args_command_state {
    cmdlist: *mut cmd_list,
    cmd: *mut c_char,
    pi: cmd_parse_input,
}

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
        (*to).value = (*from).value;
        match (*to).value {
            args_value_enum::None => (),
            args_value_enum::Commands(to_cmd_list) => {
                (*to_cmd_list).references += 1;
            }
            args_value_enum::String(to_string) => {
                (*to).value = args_value_enum::String(xstrdup(to_string).cast().as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_type_to_string(type_: *mut args_value_enum) -> *const c_char {
    match unsafe { *type_ } {
        args_value_enum::None => c"NONE".as_ptr(),
        args_value_enum::String(_) => c"STRING".as_ptr(),
        args_value_enum::Commands(_) => c"COMMANDS".as_ptr(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn args_value_as_string(value: *mut args_value) -> *const c_char {
    match unsafe { (*value).value } {
        args_value_enum::None => c"".as_ptr(),
        args_value_enum::Commands(value_cmd_list) => unsafe {
            if (*value).cached.is_null() {
                (*value).cached = cmd_list_print(value_cmd_list, 0);
            }
            (*value).cached
        },
        args_value_enum::String(value_string) => value_string,
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
        if !value.is_null() && !matches!((*value).value, args_value_enum::None) {
            tailq_insert_tail(&raw mut (*entry).values, value);
        } else {
            free(value as _);
        }
    }
}
