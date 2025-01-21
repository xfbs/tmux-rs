use super::*;

unsafe extern "C" {
    pub fn args_set(_: *mut args, _: c_uchar, _: *mut args_value, _: c_int);
    pub fn args_create() -> *mut args;
    pub fn args_parse(_: *const args_parse, _: *mut args_value, _: c_uint, _: *mut *mut c_char) -> *mut args;
    pub fn args_copy(_: *mut args, _: c_int, _: *mut *mut c_char) -> *mut args;
    pub fn args_to_vector(_: *mut args, _: *mut c_int, _: *mut *mut *mut c_char);
    pub fn args_from_vector(_: c_int, _: *mut *mut c_char) -> *mut args_value;
    pub fn args_free_value(_: *mut args_value);
    pub fn args_free_values(_: *mut args_value, _: c_uint);
    pub fn args_free(_: *mut args);
    pub fn args_print(_: *mut args) -> *mut c_char;
    pub fn args_escape(_: *const c_char) -> *mut c_char;
    pub fn args_has(_: *mut args, _: c_uchar) -> c_int;
    pub fn args_get(_: *mut args, _: c_uchar) -> *const c_char;
    pub fn args_first(_: *mut args, _: *mut *mut args_entry) -> c_uchar;
    pub fn args_next(_: *mut *mut args_entry) -> c_uchar;
    pub fn args_count(_: *mut args) -> c_uint;
    pub fn args_values(_: *mut args) -> *mut args_value;
    pub fn args_value(_: *mut args, _: c_uint) -> *mut args_value;
    pub fn args_string(_: *mut args, _: c_uint) -> *const c_char;
    pub fn args_make_commands_now(_: *mut cmd, _: *mut cmdq_item, _: c_uint, _: c_int) -> *mut cmd_list;
    pub fn args_make_commands_prepare(
        _: *mut cmd,
        _: *mut cmdq_item,
        _: c_uint,
        _: *const c_char,
        _: c_int,
        _: c_int,
    ) -> *mut args_command_state;
    pub fn args_make_commands(
        _: *mut args_command_state,
        _: c_int,
        _: *mut *mut c_char,
        _: *mut *mut c_char,
    ) -> *mut cmd_list;
    pub fn args_make_commands_free(_: *mut args_command_state);
    pub fn args_make_commands_get_command(_: *mut args_command_state) -> *mut c_char;
    pub fn args_first_value(_: *mut args, _: c_uchar) -> *mut args_value;
    pub fn args_next_value(_: *mut args_value) -> *mut args_value;
    pub fn args_strtonum(_: *mut args, _: c_uchar, _: c_longlong, _: c_longlong, _: *mut *mut c_char) -> c_longlong;
    pub fn args_strtonum_and_expand(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_percentage(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_string_percentage(
        _: *const c_char,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_percentage_and_expand(
        _: *mut args,
        _: c_uchar,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
    pub fn args_string_percentage_and_expand(
        _: *const c_char,
        _: c_longlong,
        _: c_longlong,
        _: c_longlong,
        _: *mut cmdq_item,
        _: *mut *mut c_char,
    ) -> c_longlong;
}
