use super::*;

unsafe extern "C" {
    pub unsafe fn format_draw(
        _: *mut screen_write_ctx,
        _: *const grid_cell,
        _: c_uint,
        _: *const c_char,
        _: *mut style_ranges,
        _: c_int,
    );
    pub unsafe fn format_width(_: *const c_char) -> c_uint;
    pub unsafe fn format_trim_left(_: *const c_char, _: c_uint) -> *mut c_char;
    pub unsafe fn format_trim_right(_: *const c_char, _: c_uint) -> *mut c_char;
}
