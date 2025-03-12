use super::*;
unsafe extern "C" {
    pub fn input_key_build();
    pub fn input_key_pane(_: *mut window_pane, _: key_code, _: *mut mouse_event) -> c_int;
    pub fn input_key(_: *mut screen, _: *mut bufferevent, _: key_code) -> c_int;
    pub fn input_key_get_mouse(
        _: *mut screen,
        _: *mut mouse_event,
        _: c_uint,
        _: c_uint,
        _: *mut *const c_char,
        _: *mut usize,
    ) -> c_int;
}
