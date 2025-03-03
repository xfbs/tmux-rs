use core::ffi::{c_char, c_void};

pub type wchar_t = core::ffi::c_int;

#[inline]
pub unsafe fn bsearch_<T>(
    key: *const T,
    base: *const T,
    num: usize,
    size: usize,
    compar: unsafe extern "C" fn(*const c_void, *const c_void) -> i32,
) -> *mut T {
    unsafe { ::libc::bsearch(key.cast(), base.cast(), num, size, Some(compar)).cast() }
}

#[macro_export]
macro_rules! errno {
    () => {
        *::libc::__errno_location()
    };
}
pub use errno;

#[inline]
pub fn MB_CUR_MAX() -> usize {
    unsafe extern "C" {
        unsafe fn __ctype_get_mb_cur_max() -> usize;
    }

    unsafe { __ctype_get_mb_cur_max() }
}

unsafe extern "C" {
    pub fn wcwidth(c: wchar_t) -> i32;
    pub fn mbtowc(pwc: *mut wchar_t, s: *const c_char, n: usize) -> i32;
    pub fn wctomb(s: *mut c_char, wc: wchar_t) -> i32;
}
