use super::*;

unsafe extern "C" {
    pub static mut window_copy_mode: window_mode;
    pub static mut window_view_mode: window_mode;
    pub fn window_copy_add(_: *mut window_pane, _: c_int, _: *const c_char, ...);
    pub fn window_copy_vadd(_: *mut window_pane, _: c_int, _: *const c_char, _: *mut VaList);
    pub fn window_copy_pageup(_: *mut window_pane, _: c_int);
    pub fn window_copy_pagedown(_: *mut window_pane, _: c_int, _: c_int);
    pub fn window_copy_start_drag(_: *mut client, _: *mut mouse_event);
    pub fn window_copy_get_word(_: *mut window_pane, _: c_uint, _: c_uint) -> *mut c_char;
    pub fn window_copy_get_line(_: *mut window_pane, _: c_uint) -> *mut c_char;
}
