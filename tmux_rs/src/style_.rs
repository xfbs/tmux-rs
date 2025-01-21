use super::*;
unsafe extern "C" {
    pub unsafe fn style_parse(_: *mut style, _: *const grid_cell, _: *const c_char) -> c_int;
    pub unsafe fn style_tostring(_: *mut style) -> *const c_char;
    pub unsafe fn style_add(_: *mut grid_cell, _: *mut options, _: *const c_char, _: *mut format_tree);
    pub unsafe fn style_apply(_: *mut grid_cell, _: *mut options, _: *const c_char, _: *mut format_tree);
    pub unsafe fn style_set(_: *mut style, _: *const grid_cell);
    pub unsafe fn style_copy(_: *mut style, _: *mut style);
}
