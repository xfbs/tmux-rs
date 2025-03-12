use super::*;
unsafe extern "C" {
    pub static grid_default_cell: grid_cell;
    pub fn grid_empty_line(_: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_cells_equal(_: *const grid_cell, _: *const grid_cell) -> c_int;
    pub fn grid_cells_look_equal(_: *const grid_cell, _: *const grid_cell) -> c_int;
    pub fn grid_create(_: c_uint, _: c_uint, _: c_uint) -> *mut grid;
    pub fn grid_destroy(_: *mut grid);
    pub fn grid_compare(_: *mut grid, _: *mut grid) -> c_int;
    pub fn grid_collect_history(_: *mut grid);
    pub fn grid_remove_history(_: *mut grid, _: c_uint);
    pub fn grid_scroll_history(_: *mut grid, _: c_uint);
    pub fn grid_scroll_history_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_clear_history(_: *mut grid);
    pub fn grid_peek_line(_: *mut grid, _: c_uint) -> *const grid_line;
    pub fn grid_get_cell(_: *mut grid, _: c_uint, _: c_uint, _: *mut grid_cell);
    pub fn grid_set_cell(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell);
    pub fn grid_set_padding(_: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_set_cells(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell, _: *const c_char, _: usize);
    pub fn grid_get_line(_: *mut grid, _: c_uint) -> *mut grid_line;
    pub fn grid_adjust_lines(_: *mut grid, _: c_uint);
    pub fn grid_clear(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_clear_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_move_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_move_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_string_cells(
        _: *mut grid,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut *mut grid_cell,
        _: c_int,
        _: *mut screen,
    ) -> *mut c_char;
    pub fn grid_duplicate_lines(_: *mut grid, _: c_uint, _: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_reflow(_: *mut grid, _: c_uint);
    pub fn grid_wrap_position(_: *mut grid, _: c_uint, _: c_uint, _: *mut c_uint, _: *mut c_uint);
    pub fn grid_unwrap_position(_: *mut grid, _: *mut c_uint, _: *mut c_uint, _: c_uint, _: c_uint);
    pub fn grid_line_length(_: *mut grid, _: c_uint) -> c_uint;
}
