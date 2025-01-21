use super::*;

pub const FORMAT_STATUS: u32 = 1;
pub const FORMAT_FORCE: u32 = 2;
pub const FORMAT_NOJOBS: u32 = 4;
pub const FORMAT_VERBOSE: u32 = 8;
pub const FORMAT_NONE: u32 = 0;
pub const FORMAT_PANE: u32 = 0x80000000;
pub const FORMAT_WINDOW: u32 = 0x40000000;

pub type format_cb = Option<unsafe extern "C" fn(_: *mut format_tree) -> *mut c_void>;

unsafe extern "C" {
    pub unsafe fn format_tidy_jobs();
    pub unsafe fn format_skip(_: *const c_char, _: *const c_char) -> *const c_char;
    pub unsafe fn format_true(_: *const c_char) -> c_int;
    pub unsafe fn format_create(_: *mut client, _: *mut cmdq_item, _: c_int, _: c_int) -> *mut format_tree;
    pub unsafe fn format_free(_: *mut format_tree);
    pub unsafe fn format_merge(_: *mut format_tree, _: *mut format_tree);
    pub unsafe fn format_get_pane(_: *mut format_tree) -> *mut window_pane;
    pub unsafe fn format_add(_: *mut format_tree, _: *const c_char, _: *const c_char, ...);
    pub unsafe fn format_add_tv(_: *mut format_tree, _: *const c_char, _: *mut timeval);
    pub unsafe fn format_add_cb(_: *mut format_tree, _: *const c_char, _: format_cb);
    pub unsafe fn format_log_debug(_: *mut format_tree, _: *const c_char);
    pub unsafe fn format_each(
        _: *mut format_tree,
        _: Option<unsafe extern "C" fn(_: *const c_char, _: *const c_char, _: *mut c_void)>,
        _: *mut c_void,
    );
    pub unsafe fn format_pretty_time(_: time_t, _: c_int) -> *mut c_char;
    pub unsafe fn format_expand_time(_: *mut format_tree, _: *const c_char) -> *mut c_char;
    pub unsafe fn format_expand(_: *mut format_tree, _: *const c_char) -> *mut c_char;
    pub unsafe fn format_single(
        _: *mut cmdq_item,
        _: *const c_char,
        _: *mut client,
        _: *mut session,
        _: *mut winlink,
        _: *mut window_pane,
    ) -> *mut c_char;
    pub unsafe fn format_single_from_state(
        _: *mut cmdq_item,
        _: *const c_char,
        _: *mut client,
        _: *mut cmd_find_state,
    ) -> *mut c_char;
    pub unsafe fn format_single_from_target(_: *mut cmdq_item, _: *const c_char) -> *mut c_char;
    pub unsafe fn format_create_defaults(
        _: *mut cmdq_item,
        _: *mut client,
        _: *mut session,
        _: *mut winlink,
        _: *mut window_pane,
    ) -> *mut format_tree;
    pub unsafe fn format_create_from_state(
        _: *mut cmdq_item,
        _: *mut client,
        _: *mut cmd_find_state,
    ) -> *mut format_tree;
    pub unsafe fn format_create_from_target(_: *mut cmdq_item) -> *mut format_tree;
    pub unsafe fn format_defaults(
        _: *mut format_tree,
        _: *mut client,
        _: *mut session,
        _: *mut winlink,
        _: *mut window_pane,
    );
    pub unsafe fn format_defaults_window(_: *mut format_tree, _: *mut window);
    pub unsafe fn format_defaults_pane(_: *mut format_tree, _: *mut window_pane);
    pub unsafe fn format_defaults_paste_buffer(_: *mut format_tree, _: *mut paste_buffer);
    pub unsafe fn format_lost_client(_: *mut client);
    pub unsafe fn format_grid_word(_: *mut grid, _: c_uint, _: c_uint) -> *mut c_char;
    pub unsafe fn format_grid_hyperlink(_: *mut grid, _: c_uint, _: c_uint, _: *mut screen) -> *mut c_char;
    pub unsafe fn format_grid_line(_: *mut grid, _: c_uint) -> *mut c_char;
}
