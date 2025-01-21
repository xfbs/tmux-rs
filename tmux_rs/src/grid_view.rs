use super::*;
unsafe extern "C" {
    pub fn grid_view_get_cell(_: *mut grid, _: c_uint, _: c_uint, _: *mut grid_cell);
    pub fn grid_view_set_cell(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell);
    pub fn grid_view_set_padding(_: *mut grid, _: c_uint, _: c_uint);
    pub fn grid_view_set_cells(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell, _: *const c_char, _: usize);
    pub fn grid_view_clear_history(_: *mut grid, _: c_uint);
    pub fn grid_view_clear(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_scroll_region_up(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_scroll_region_down(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_insert_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_insert_lines_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_delete_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_delete_lines_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_insert_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_delete_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn grid_view_string_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint) -> *mut c_char;
}
