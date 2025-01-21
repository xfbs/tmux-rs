use super::*;

unsafe extern "C" {
    pub unsafe fn client_main(_: *mut event_base, _: c_int, _: *mut *mut c_char, _: u64, _: c_int) -> c_int;
}
