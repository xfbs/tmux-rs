use super::*;
unsafe extern "C" {
    pub unsafe fn osdep_get_name(_: c_int, _: *mut c_char) -> *mut c_char;
    pub unsafe fn osdep_get_cwd(_: c_int) -> *mut c_char;
    pub unsafe fn osdep_event_init() -> *mut event_base;
}
