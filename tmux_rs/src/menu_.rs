use super::*;

unsafe extern "C" {
    pub unsafe fn menu_create(_: *const c_char) -> *mut menu;
    pub unsafe fn menu_add_items(
        _: *mut menu,
        _: *const menu_item,
        _: *mut cmdq_item,
        _: *mut client,
        _: *mut cmd_find_state,
    );
    pub unsafe fn menu_add_item(
        _: *mut menu,
        _: *const menu_item,
        _: *mut cmdq_item,
        _: *mut client,
        _: *mut cmd_find_state,
    );
    pub unsafe fn menu_free(_: *mut menu);
    pub unsafe fn menu_prepare(
        _: *mut menu,
        _: c_int,
        _: c_int,
        _: *mut cmdq_item,
        _: c_uint,
        _: c_uint,
        _: *mut client,
        _: box_lines,
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut cmd_find_state,
        _: menu_choice_cb,
        _: *mut c_void,
    ) -> *mut menu_data;
    pub unsafe fn menu_display(
        _: *mut menu,
        _: c_int,
        _: c_int,
        _: *mut cmdq_item,
        _: c_uint,
        _: c_uint,
        _: *mut client,
        _: box_lines,
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut cmd_find_state,
        _: menu_choice_cb,
        _: *mut c_void,
    ) -> c_int;
    pub unsafe fn menu_mode_cb(_: *mut client, _: *mut c_void, _: *mut c_uint, _: *mut c_uint) -> *mut screen;
    pub unsafe fn menu_check_cb(
        _: *mut client,
        _: *mut c_void,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut overlay_ranges,
    );
    pub unsafe fn menu_draw_cb(_: *mut client, _: *mut c_void, _: *mut screen_redraw_ctx);
    pub unsafe fn menu_free_cb(_: *mut client, _: *mut c_void);
    pub unsafe fn menu_key_cb(_: *mut client, _: *mut c_void, _: *mut key_event) -> c_int;
}
