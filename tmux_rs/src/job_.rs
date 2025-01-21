use super::*;

pub type job_update_cb = Option<unsafe extern "C" fn(_: *mut job)>;
pub type job_complete_cb = Option<unsafe extern "C" fn(_: *mut job)>;
pub type job_free_cb = Option<unsafe extern "C" fn(_: *mut c_void)>;
unsafe extern "C" {
    pub fn job_run(
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
        _: *mut environ,
        _: *mut session,
        _: *const c_char,
        _: job_update_cb,
        _: job_complete_cb,
        _: job_free_cb,
        _: *mut c_void,
        _: c_int,
        _: c_int,
        _: c_int,
    ) -> *mut job;
    pub fn job_free(_: *mut job);
    pub fn job_transfer(_: *mut job, _: *mut pid_t, _: *mut c_char, _: usize) -> c_int;
    pub fn job_resize(_: *mut job, _: c_uint, _: c_uint);
    pub fn job_check_died(_: pid_t, _: c_int);
    pub fn job_get_status(_: *mut job) -> c_int;
    pub fn job_get_data(_: *mut job) -> *mut c_void;
    pub fn job_get_event(_: *mut job) -> *mut bufferevent;
    pub fn job_kill_all();
    pub fn job_still_running() -> c_int;
    pub fn job_print_summary(_: *mut cmdq_item, _: c_int);
}
