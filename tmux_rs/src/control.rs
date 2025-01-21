use super::*;
unsafe extern "C" {
    pub unsafe fn control_discard(_: *mut client);
    pub unsafe fn control_start(_: *mut client);
    pub unsafe fn control_ready(_: *mut client);
    pub unsafe fn control_stop(_: *mut client);
    pub unsafe fn control_set_pane_on(_: *mut client, _: *mut window_pane);
    pub unsafe fn control_set_pane_off(_: *mut client, _: *mut window_pane);
    pub unsafe fn control_continue_pane(_: *mut client, _: *mut window_pane);
    pub unsafe fn control_pause_pane(_: *mut client, _: *mut window_pane);
    pub unsafe fn control_pane_offset(_: *mut client, _: *mut window_pane, _: *mut c_int) -> *mut window_pane_offset;
    pub unsafe fn control_reset_offsets(_: *mut client);
    pub unsafe fn control_write(_: *mut client, _: *const c_char, ...);
    pub unsafe fn control_write_output(_: *mut client, _: *mut window_pane);
    pub unsafe fn control_all_done(_: *mut client) -> c_int;
    pub unsafe fn control_add_sub(_: *mut client, _: *const c_char, _: control_sub_type, _: c_int, _: *const c_char);
    pub unsafe fn control_remove_sub(_: *mut client, _: *const c_char);
}
