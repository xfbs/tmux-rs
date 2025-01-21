use super::*;

unsafe extern "C" {
    pub fn tty_acs_needed(_: *mut tty) -> c_int;
    pub fn tty_acs_get(_: *mut tty, _: c_uchar) -> *const c_char;
    pub fn tty_acs_reverse_get(_: *mut tty, _: *const c_char, _: usize) -> c_int;
    pub fn tty_acs_double_borders(_: c_int) -> *const utf8_data;
    pub fn tty_acs_heavy_borders(_: c_int) -> *const utf8_data;
    pub fn tty_acs_rounded_borders(_: c_int) -> *const utf8_data;
}
