use super::*;

unsafe extern "C" {
    pub unsafe fn paste_buffer_name(_: *mut paste_buffer) -> *const c_char;
    pub unsafe fn paste_buffer_order(_: *mut paste_buffer) -> c_uint;
    pub unsafe fn paste_buffer_created(_: *mut paste_buffer) -> time_t;
    pub unsafe fn paste_buffer_data(_: *mut paste_buffer, _: *mut usize) -> *const c_char;
    pub unsafe fn paste_walk(_: *mut paste_buffer) -> *mut paste_buffer;
    pub unsafe fn paste_is_empty() -> c_int;
    pub unsafe fn paste_get_top(_: *mut *const c_char) -> *mut paste_buffer;
    pub unsafe fn paste_get_name(_: *const c_char) -> *mut paste_buffer;
    pub unsafe fn paste_free(_: *mut paste_buffer);
    pub unsafe fn paste_add(_: *const c_char, _: *mut c_char, _: usize);
    pub unsafe fn paste_rename(_: *const c_char, _: *const c_char, _: *mut *mut c_char) -> c_int;
    pub unsafe fn paste_set(_: *mut c_char, _: usize, _: *const c_char, _: *mut *mut c_char) -> c_int;
    pub unsafe fn paste_replace(_: *mut paste_buffer, _: *mut c_char, _: usize);
    pub unsafe fn paste_make_sample(_: *mut paste_buffer) -> *mut c_char;
}
