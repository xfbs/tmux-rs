use super::*;

unsafe extern "C" {
    pub static mut cmd_table: [*const cmd_entry; 0usize];
    pub fn cmd_log_argv(_: c_int, _: *mut *mut c_char, _: *const c_char, ...);
    pub fn cmd_prepend_argv(_: *mut c_int, _: *mut *mut *mut c_char, _: *const c_char);
    pub fn cmd_append_argv(_: *mut c_int, _: *mut *mut *mut c_char, _: *const c_char);
    pub fn cmd_pack_argv(_: c_int, _: *mut *mut c_char, _: *mut c_char, _: usize) -> c_int;
    pub fn cmd_unpack_argv(_: *mut c_char, _: usize, _: c_int, _: *mut *mut *mut c_char) -> c_int;
    pub fn cmd_copy_argv(_: c_int, _: *mut *mut c_char) -> *mut *mut c_char;
    pub fn cmd_free_argv(_: c_int, _: *mut *mut c_char);
    pub fn cmd_stringify_argv(_: c_int, _: *mut *mut c_char) -> *mut c_char;
    pub fn cmd_get_alias(_: *const c_char) -> *mut c_char;
    pub fn cmd_get_entry(_: *mut cmd) -> *const cmd_entry;
    pub fn cmd_get_args(_: *mut cmd) -> *mut args;
    pub fn cmd_get_group(_: *mut cmd) -> c_uint;
    pub fn cmd_get_source(_: *mut cmd, _: *mut *const c_char, _: *mut c_uint);
    pub fn cmd_parse(_: *mut args_value, _: c_uint, _: *const c_char, _: c_uint, _: *mut *mut c_char) -> *mut cmd;
    pub fn cmd_copy(_: *mut cmd, _: c_int, _: *mut *mut c_char) -> *mut cmd;
    pub fn cmd_free(_: *mut cmd);
    pub fn cmd_print(_: *mut cmd) -> *mut c_char;
    pub fn cmd_list_new() -> *mut cmd_list;
    pub fn cmd_list_copy(_: *mut cmd_list, _: c_int, _: *mut *mut c_char) -> *mut cmd_list;
    pub fn cmd_list_append(_: *mut cmd_list, _: *mut cmd);
    pub fn cmd_list_append_all(_: *mut cmd_list, _: *mut cmd_list);
    pub fn cmd_list_move(_: *mut cmd_list, _: *mut cmd_list);
    pub fn cmd_list_free(_: *mut cmd_list);
    pub fn cmd_list_print(_: *mut cmd_list, _: c_int) -> *mut c_char;
    pub fn cmd_list_first(_: *mut cmd_list) -> *mut cmd;
    pub fn cmd_list_next(_: *mut cmd) -> *mut cmd;
    pub fn cmd_list_all_have(_: *mut cmd_list, _: c_int) -> c_int;
    pub fn cmd_list_any_have(_: *mut cmd_list, _: c_int) -> c_int;
    pub fn cmd_mouse_at(_: *mut window_pane, _: *mut mouse_event, _: *mut c_uint, _: *mut c_uint, _: c_int) -> c_int;
    pub fn cmd_mouse_window(_: *mut mouse_event, _: *mut *mut session) -> *mut winlink;
    pub fn cmd_mouse_pane(_: *mut mouse_event, _: *mut *mut session, _: *mut *mut winlink) -> *mut window_pane;
    pub fn cmd_template_replace(_: *const c_char, _: *const c_char, _: c_int) -> *mut c_char;
}
