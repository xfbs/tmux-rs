use super::*;
unsafe extern "C" {
    pub fn layout_dump(_: *mut layout_cell) -> *mut c_char;
    pub fn layout_parse(_: *mut window, _: *const c_char, _: *mut *mut c_char) -> c_int;
}
