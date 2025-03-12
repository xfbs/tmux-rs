use super::*;
unsafe extern "C" {
    pub fn layout_set_lookup(_: *const c_char) -> c_int;
    pub fn layout_set_select(_: *mut window, _: c_uint) -> c_uint;
    pub fn layout_set_next(_: *mut window) -> c_uint;
    pub fn layout_set_previous(_: *mut window) -> c_uint;
}
