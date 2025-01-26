use super::*;

unsafe extern "C" {
    pub fn cmd_parse_from_file(_: *mut FILE, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    pub fn cmd_parse_from_string(_: *const c_char, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    pub fn cmd_parse_and_insert(
        _: *const c_char,
        _: *mut cmd_parse_input,
        _: *mut cmdq_item,
        _: *mut cmdq_state,
        _: *mut *mut c_char,
    ) -> cmd_parse_status;
    pub fn cmd_parse_and_append(
        _: *const c_char,
        _: *mut cmd_parse_input,
        _: *mut client,
        _: *mut cmdq_state,
        _: *mut *mut c_char,
    ) -> cmd_parse_status;
    pub fn cmd_parse_from_buffer(_: *const c_void, _: usize, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
    pub fn cmd_parse_from_arguments(_: *mut args_value, _: c_uint, _: *mut cmd_parse_input) -> *mut cmd_parse_result;
}
