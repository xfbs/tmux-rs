use super::*;

unsafe extern "C" {
    pub fn screen_init(_: *mut screen, _: c_uint, _: c_uint, _: c_uint);
    pub fn screen_reinit(_: *mut screen);
    pub fn screen_free(_: *mut screen);
    pub fn screen_reset_tabs(_: *mut screen);
    pub fn screen_reset_hyperlinks(_: *mut screen);
    pub fn screen_set_cursor_style(_: c_uint, _: *mut screen_cursor_style, _: *mut c_int);
    pub fn screen_set_cursor_colour(_: *mut screen, _: c_int);
    pub fn screen_set_title(_: *mut screen, _: *const c_char) -> c_int;
    pub fn screen_set_path(_: *mut screen, _: *const c_char);
    pub fn screen_push_title(_: *mut screen);
    pub fn screen_pop_title(_: *mut screen);
    pub fn screen_resize(_: *mut screen, _: c_uint, _: c_uint, _: c_int);
    pub fn screen_resize_cursor(_: *mut screen, _: c_uint, _: c_uint, _: c_int, _: c_int, _: c_int);
    pub fn screen_set_selection(
        _: *mut screen,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_int,
        _: *mut grid_cell,
    );
    pub fn screen_clear_selection(_: *mut screen);
    pub fn screen_hide_selection(_: *mut screen);
    pub fn screen_check_selection(_: *mut screen, _: c_uint, _: c_uint) -> c_int;
    pub fn screen_select_cell(_: *mut screen, _: *mut grid_cell, _: *const grid_cell);
    pub fn screen_alternate_on(_: *mut screen, _: *mut grid_cell, _: c_int);
    pub fn screen_alternate_off(_: *mut screen, _: *mut grid_cell, _: c_int);
    pub fn screen_mode_to_string(_: c_int) -> *const c_char;
}
