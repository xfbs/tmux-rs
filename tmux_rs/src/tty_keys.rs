use super::*;

unsafe extern "C" {
    pub fn tty_keys_build(_: *mut tty);
    pub fn tty_keys_free(_: *mut tty);
    pub fn tty_keys_next(_: *mut tty) -> c_int;
    pub fn tty_keys_colours(
        _: *mut tty,
        _: *const c_char,
        _: usize,
        _: *mut usize,
        _: *mut c_int,
        _: *mut c_int,
    ) -> c_int;
}
