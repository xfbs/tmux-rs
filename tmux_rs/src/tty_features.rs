use super::*;

unsafe extern "C" {
    pub fn tty_add_features(_: *mut c_int, _: *const c_char, _: *const c_char);
    pub fn tty_get_features(_: c_int) -> *const c_char;
    pub fn tty_apply_features(_: *mut tty_term, _: c_int) -> c_int;
    pub fn tty_default_features(_: *mut c_int, _: *const c_char, _: c_uint);
}
