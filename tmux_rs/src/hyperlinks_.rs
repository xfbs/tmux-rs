use super::*;

unsafe extern "C" {
    pub unsafe fn hyperlinks_put(_: *mut hyperlinks, _: *const c_char, _: *const c_char) -> c_uint;
    pub unsafe fn hyperlinks_get(
        _: *mut hyperlinks,
        _: c_uint,
        _: *mut *const c_char,
        _: *mut *const c_char,
        _: *mut *const c_char,
    ) -> c_int;
    pub unsafe fn hyperlinks_init() -> *mut hyperlinks;
    pub unsafe fn hyperlinks_copy(_: *mut hyperlinks) -> *mut hyperlinks;
    pub unsafe fn hyperlinks_reset(_: *mut hyperlinks);
    pub unsafe fn hyperlinks_free(_: *mut hyperlinks);
}
