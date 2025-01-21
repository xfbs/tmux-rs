use super::*;
unsafe extern "C" {
    pub unsafe fn spawn_window(_: *mut spawn_context, _: *mut *mut c_char) -> *mut winlink;
    pub unsafe fn spawn_pane(_: *mut spawn_context, _: *mut *mut c_char) -> *mut window_pane;
}
