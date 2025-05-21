use super::*;

pub type mode_tree_build_cb = Option<unsafe extern "C" fn(_: NonNull<c_void>, _: *mut mode_tree_sort_criteria, _: *mut u64, _: *const c_char)>;
pub type mode_tree_draw_cb = Option<unsafe extern "C" fn(_: *mut c_void, _: NonNull<c_void>, _: *mut screen_write_ctx, _: c_uint, _: c_uint)>;
pub type mode_tree_search_cb = Option<unsafe extern "C" fn(_: *mut c_void, _: NonNull<c_void>, _: *const c_char) -> boolint>;
pub type mode_tree_menu_cb = Option<unsafe extern "C" fn(_: NonNull<c_void>, _: *mut client, _: key_code)>;
pub type mode_tree_height_cb = Option<unsafe extern "C" fn(_: *mut c_void, _: c_uint) -> c_uint>;
pub type mode_tree_key_cb = Option<unsafe extern "C" fn(_: NonNull<c_void>, _: NonNull<c_void>, _: c_uint) -> key_code>;
pub type mode_tree_each_cb = Option<unsafe extern "C" fn(_: NonNull<c_void>, _: NonNull<c_void>, _: *mut client, _: key_code)>;
unsafe extern "C" {
    pub fn mode_tree_count_tagged(_: *mut mode_tree_data) -> c_uint;
    pub fn mode_tree_get_current(_: *mut mode_tree_data) -> NonNull<c_void>;
    pub fn mode_tree_get_current_name(_: *mut mode_tree_data) -> *const c_char;
    pub fn mode_tree_expand_current(_: *mut mode_tree_data);
    pub fn mode_tree_collapse_current(_: *mut mode_tree_data);
    pub fn mode_tree_expand(_: *mut mode_tree_data, _: u64);
    pub fn mode_tree_set_current(_: *mut mode_tree_data, _: u64) -> c_int;
    pub fn mode_tree_each_tagged(_: *mut mode_tree_data, _: mode_tree_each_cb, _: *mut client, _: key_code, _: c_int);
    pub fn mode_tree_up(_: *mut mode_tree_data, _: c_int);
    pub fn mode_tree_down(_: *mut mode_tree_data, _: c_int) -> c_int;
    pub fn mode_tree_start(
        _: *mut window_pane,
        _: *mut args,
        _: mode_tree_build_cb,
        _: mode_tree_draw_cb,
        _: mode_tree_search_cb,
        _: mode_tree_menu_cb,
        _: mode_tree_height_cb,
        _: mode_tree_key_cb,
        _: *mut c_void,
        _: *const menu_item,
        _: *mut *const c_char,
        _: c_uint,
        _: *mut *mut screen,
    ) -> *mut mode_tree_data;
    pub fn mode_tree_zoom(_: *mut mode_tree_data, _: *mut args);
    pub fn mode_tree_build(_: *mut mode_tree_data);
    pub fn mode_tree_free(_: *mut mode_tree_data);
    pub fn mode_tree_resize(_: *mut mode_tree_data, _: c_uint, _: c_uint);
    pub fn mode_tree_add(_: *mut mode_tree_data, _: *mut mode_tree_item, _: *mut c_void, _: u64, _: *const c_char, _: *const c_char, _: c_int) -> *mut mode_tree_item;
    pub fn mode_tree_draw_as_parent(_: *mut mode_tree_item);
    pub fn mode_tree_no_tag(_: *mut mode_tree_item);
    pub fn mode_tree_remove(_: *mut mode_tree_data, _: *mut mode_tree_item);
    pub fn mode_tree_draw(_: *mut mode_tree_data);
    pub fn mode_tree_key(_: *mut mode_tree_data, _: *mut client, _: *mut key_code, _: *mut mouse_event, _: *mut c_uint, _: *mut c_uint) -> c_int;
    pub fn mode_tree_run_command(_: *mut client, _: *mut cmd_find_state, _: *const c_char, _: *const c_char);
}
