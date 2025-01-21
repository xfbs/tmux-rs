use super::*;

unsafe extern "C" {
    pub fn server_redraw_client(_: *mut client);
    pub fn server_status_client(_: *mut client);
    pub fn server_redraw_session(_: *mut session);
    pub fn server_redraw_session_group(_: *mut session);
    pub fn server_status_session(_: *mut session);
    pub fn server_status_session_group(_: *mut session);
    pub fn server_redraw_window(_: *mut window);
    pub fn server_redraw_window_borders(_: *mut window);
    pub fn server_status_window(_: *mut window);
    pub fn server_lock();
    pub fn server_lock_session(_: *mut session);
    pub fn server_lock_client(_: *mut client);
    pub fn server_kill_pane(_: *mut window_pane);
    pub fn server_kill_window(_: *mut window, _: c_int);
    pub fn server_renumber_session(_: *mut session);
    pub fn server_renumber_all();
    pub fn server_link_window(
        _: *mut session,
        _: *mut winlink,
        _: *mut session,
        _: c_int,
        _: c_int,
        _: c_int,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn server_unlink_window(_: *mut session, _: *mut winlink);
    pub fn server_destroy_pane(_: *mut window_pane, _: c_int);
    pub fn server_destroy_session(_: *mut session);
    pub fn server_check_unattached();
    pub fn server_unzoom_window(_: *mut window);
}
