use super::*;

unsafe extern "C" {
    pub fn layout_count_cells(_: *mut layout_cell) -> c_uint;
    pub fn layout_create_cell(_: *mut layout_cell) -> *mut layout_cell;
    pub fn layout_free_cell(_: *mut layout_cell);
    pub fn layout_print_cell(_: *mut layout_cell, _: *const c_char, _: c_uint);
    pub fn layout_destroy_cell(_: *mut window, _: *mut layout_cell, _: *mut *mut layout_cell);
    pub fn layout_resize_layout(_: *mut window, _: *mut layout_cell, _: layout_type, _: c_int, _: c_int);
    pub fn layout_search_by_border(_: *mut layout_cell, _: c_uint, _: c_uint) -> *mut layout_cell;
    pub fn layout_set_size(_: *mut layout_cell, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn layout_make_leaf(_: *mut layout_cell, _: *mut window_pane);
    pub fn layout_make_node(_: *mut layout_cell, _: layout_type);
    pub fn layout_fix_offsets(_: *mut window);
    pub fn layout_fix_panes(_: *mut window, _: *mut window_pane);
    pub fn layout_resize_adjust(_: *mut window, _: *mut layout_cell, _: layout_type, _: c_int);
    pub fn layout_init(_: *mut window, _: *mut window_pane);
    pub fn layout_free(_: *mut window);
    pub fn layout_resize(_: *mut window, _: c_uint, _: c_uint);
    pub fn layout_resize_pane(_: *mut window_pane, _: layout_type, _: c_int, _: c_int);
    pub fn layout_resize_pane_to(_: *mut window_pane, _: layout_type, _: c_uint);
    pub fn layout_assign_pane(_: *mut layout_cell, _: *mut window_pane, _: c_int);
    pub fn layout_split_pane(_: *mut window_pane, _: layout_type, _: c_int, _: c_int) -> *mut layout_cell;
    pub fn layout_close_pane(_: *mut window_pane);
    pub fn layout_spread_cell(_: *mut window, _: *mut layout_cell) -> c_int;
    pub fn layout_spread_out(_: *mut window_pane);
}
