use super::*;
unsafe extern "C" {
    pub unsafe fn regsub(_: *const c_char, _: *const c_char, _: *const c_char, _: c_int) -> *mut c_char;
}
