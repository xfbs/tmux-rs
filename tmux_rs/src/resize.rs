use super::*;

unsafe extern "C" {
    pub fn resize_window(_: *mut window, _: c_uint, _: c_uint, _: c_int, _: c_int);
    pub fn default_window_size(
        _: *mut client,
        _: *mut session,
        _: *mut window,
        _: *mut c_uint,
        _: *mut c_uint,
        _: *mut c_uint,
        _: *mut c_uint,
        _: c_int,
    );
    pub fn recalculate_size(_: *mut window, _: c_int);
    pub fn recalculate_sizes();
    pub fn recalculate_sizes_now(_: c_int);
}
