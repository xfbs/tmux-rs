use super::*;
unsafe extern "C" {
    pub unsafe fn control_notify_pane_mode_changed(_: c_int);
    pub unsafe fn control_notify_window_layout_changed(_: *mut window);
    pub unsafe fn control_notify_window_pane_changed(_: *mut window);
    pub unsafe fn control_notify_window_unlinked(_: *mut session, _: *mut window);
    pub unsafe fn control_notify_window_linked(_: *mut session, _: *mut window);
    pub unsafe fn control_notify_window_renamed(_: *mut window);
    pub unsafe fn control_notify_client_session_changed(_: *mut client);
    pub unsafe fn control_notify_client_detached(_: *mut client);
    pub unsafe fn control_notify_session_renamed(_: *mut session);
    pub unsafe fn control_notify_session_created(_: *mut session);
    pub unsafe fn control_notify_session_closed(_: *mut session);
    pub unsafe fn control_notify_session_window_changed(_: *mut session);
    pub unsafe fn control_notify_paste_buffer_changed(_: *const c_char);
    pub unsafe fn control_notify_paste_buffer_deleted(_: *const c_char);
}
