use super::*;
unsafe extern "C" {
    pub fn grid_reader_start(_: *mut grid_reader, _: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_reader_get_cursor(_: *mut grid_reader, _: *mut c_uint, _: *mut c_uint);
    pub fn grid_reader_line_length(_: *mut grid_reader) -> c_uint;
    pub fn grid_reader_in_set(_: *mut grid_reader, _: *const c_char) -> c_int;
    pub fn grid_reader_cursor_right(_: *mut grid_reader, _: c_int, _: c_int);
    pub fn grid_reader_cursor_left(_: *mut grid_reader, _: c_int);
    pub fn grid_reader_cursor_down(_: *mut grid_reader);
    pub fn grid_reader_cursor_up(_: *mut grid_reader);
    pub fn grid_reader_cursor_start_of_line(_: *mut grid_reader, _: c_int);
    pub fn grid_reader_cursor_end_of_line(_: *mut grid_reader, _: c_int, _: c_int);
    pub fn grid_reader_cursor_next_word(_: *mut grid_reader, _: *const c_char);
    pub fn grid_reader_cursor_next_word_end(_: *mut grid_reader, _: *const c_char);
    pub fn grid_reader_cursor_previous_word(_: *mut grid_reader, _: *const c_char, _: c_int, _: c_int);
    pub fn grid_reader_cursor_jump(_: *mut grid_reader, _: *const utf8_data) -> c_int;
    pub fn grid_reader_cursor_jump_back(_: *mut grid_reader, _: *const utf8_data) -> c_int;
    pub fn grid_reader_cursor_back_to_indentation(_: *mut grid_reader);
}
