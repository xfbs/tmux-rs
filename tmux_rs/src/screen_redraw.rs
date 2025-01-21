use super::*;
unsafe extern "C" {
    pub fn screen_redraw_screen(_: *mut client);
    pub fn screen_redraw_pane(_: *mut client, _: *mut window_pane);
}
