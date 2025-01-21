use super::*;
unsafe extern "C" {
    pub fn notify_hook(_: *mut cmdq_item, _: *const c_char);
    pub fn notify_client(_: *const c_char, _: *mut client);
    pub fn notify_session(_: *const c_char, _: *mut session);
    pub fn notify_winlink(_: *const c_char, _: *mut winlink);
    pub fn notify_session_window(_: *const c_char, _: *mut session, _: *mut window);
    pub fn notify_window(_: *const c_char, _: *mut window);
    pub fn notify_pane(_: *const c_char, _: *mut window_pane);
    pub fn notify_paste_buffer(_: *const c_char, _: c_int);
}
