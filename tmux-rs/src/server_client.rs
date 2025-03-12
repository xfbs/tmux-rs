use super::*;

unsafe extern "C" {
    pub fn client_windows_RB_INSERT_COLOR(_: *mut client_windows, _: *mut client_window);
    pub fn client_windows_RB_REMOVE_COLOR(_: *mut client_windows, _: *mut client_window, _: *mut client_window);
    pub fn client_windows_RB_REMOVE(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn client_windows_RB_INSERT(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn client_windows_RB_FIND(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn client_windows_RB_NFIND(_: *mut client_windows, _: *mut client_window) -> *mut client_window;
    pub fn server_client_how_many() -> c_uint;
    pub fn server_client_set_overlay(
        _: *mut client,
        _: c_uint,
        _: overlay_check_cb,
        _: overlay_mode_cb,
        _: overlay_draw_cb,
        _: overlay_key_cb,
        _: overlay_free_cb,
        _: overlay_resize_cb,
        _: *mut c_void,
    );
    pub fn server_client_clear_overlay(_: *mut client);
    pub fn server_client_overlay_range(
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut overlay_ranges,
    );
    pub fn server_client_set_key_table(_: *mut client, _: *const c_char);
    pub fn server_client_get_key_table(_: *mut client) -> *const c_char;
    pub fn server_client_check_nested(_: *mut client) -> c_int;
    pub fn server_client_handle_key(_: *mut client, _: *mut key_event) -> c_int;
    pub fn server_client_create(_: c_int) -> *mut client;
    pub fn server_client_open(_: *mut client, _: *mut *mut c_char) -> c_int;
    pub fn server_client_unref(_: *mut client);
    pub fn server_client_set_session(_: *mut client, _: *mut session);
    pub fn server_client_lost(_: *mut client);
    pub fn server_client_suspend(_: *mut client);
    pub fn server_client_detach(_: *mut client, _: msgtype);
    pub fn server_client_exec(_: *mut client, _: *const c_char);
    pub fn server_client_loop();
    pub fn server_client_get_cwd(_: *mut client, _: *mut session) -> *const c_char;
    pub fn server_client_set_flags(_: *mut client, _: *const c_char);
    pub fn server_client_get_flags(_: *mut client) -> *const c_char;
    pub fn server_client_get_client_window(_: *mut client, _: c_uint) -> *mut client_window;
    pub fn server_client_add_client_window(_: *mut client, _: c_uint) -> *mut client_window;
    pub fn server_client_get_pane(_: *mut client) -> *mut window_pane;
    pub fn server_client_set_pane(_: *mut client, _: *mut window_pane);
    pub fn server_client_remove_pane(_: *mut window_pane);
    pub fn server_client_print(_: *mut client, _: c_int, _: *mut evbuffer);
}
