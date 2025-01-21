use super::*;

unsafe extern "C" {
    pub unsafe fn proc_send(_: *mut tmuxpeer, _: msgtype, _: c_int, _: *const c_void, _: usize) -> c_int;
    pub unsafe fn proc_start(_: *const c_char) -> *mut tmuxproc;
    pub unsafe fn proc_loop(_: *mut tmuxproc, _: Option<unsafe extern "C" fn() -> c_int>);
    pub unsafe fn proc_exit(_: *mut tmuxproc);
    pub unsafe fn proc_set_signals(_: *mut tmuxproc, _: Option<unsafe extern "C" fn(_: c_int)>);
    pub unsafe fn proc_clear_signals(_: *mut tmuxproc, _: c_int);
    pub unsafe fn proc_add_peer(
        _: *mut tmuxproc,
        _: c_int,
        _: Option<unsafe extern "C" fn(_: *mut imsg, _: *mut c_void)>,
        _: *mut c_void,
    ) -> *mut tmuxpeer;
    pub unsafe fn proc_remove_peer(_: *mut tmuxpeer);
    pub unsafe fn proc_kill_peer(_: *mut tmuxpeer);
    pub unsafe fn proc_flush_peer(_: *mut tmuxpeer);
    pub unsafe fn proc_toggle_log(_: *mut tmuxproc);
    pub unsafe fn proc_fork_and_daemon(_: *mut c_int) -> pid_t;
    pub unsafe fn proc_get_peer_uid(_: *mut tmuxpeer) -> uid_t;
}
