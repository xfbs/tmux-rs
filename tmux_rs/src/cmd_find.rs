use super::*;

unsafe extern "C" {
    pub fn cmd_find_target(
        _: *mut cmd_find_state,
        _: *mut cmdq_item,
        _: *const c_char,
        _: cmd_find_type,
        _: c_int,
    ) -> c_int;
    pub fn cmd_find_best_client(_: *mut session) -> *mut client;
    pub fn cmd_find_client(_: *mut cmdq_item, _: *const c_char, _: c_int) -> *mut client;
    pub fn cmd_find_clear_state(_: *mut cmd_find_state, _: c_int);
    pub fn cmd_find_empty_state(_: *mut cmd_find_state) -> c_int;
    pub fn cmd_find_valid_state(_: *mut cmd_find_state) -> c_int;
    pub fn cmd_find_copy_state(_: *mut cmd_find_state, _: *mut cmd_find_state);
    pub fn cmd_find_from_session(_: *mut cmd_find_state, _: *mut session, _: c_int);
    pub fn cmd_find_from_winlink(_: *mut cmd_find_state, _: *mut winlink, _: c_int);
    pub fn cmd_find_from_session_window(_: *mut cmd_find_state, _: *mut session, _: *mut window, _: c_int) -> c_int;
    pub fn cmd_find_from_window(_: *mut cmd_find_state, _: *mut window, _: c_int) -> c_int;
    pub fn cmd_find_from_winlink_pane(_: *mut cmd_find_state, _: *mut winlink, _: *mut window_pane, _: c_int);
    pub fn cmd_find_from_pane(_: *mut cmd_find_state, _: *mut window_pane, _: c_int) -> c_int;
    pub fn cmd_find_from_client(_: *mut cmd_find_state, _: *mut client, _: c_int) -> c_int;
    pub fn cmd_find_from_mouse(_: *mut cmd_find_state, _: *mut mouse_event, _: c_int) -> c_int;
    pub fn cmd_find_from_nothing(_: *mut cmd_find_state, _: c_int) -> c_int;
}
