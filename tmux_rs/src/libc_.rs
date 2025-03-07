use core::ffi::{c_char, c_void};

use libc::timeval;

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

#[inline]
pub unsafe fn memset0<T>(dest: *mut T) -> *mut T { unsafe { libc::memset(dest.cast(), 0, size_of::<T>()).cast() } }

#[inline]
pub unsafe fn timerclear(tv: *mut timeval) {
    // implemented as a macro by most libc's
    unsafe {
        (*tv).tv_sec = 0;
        (*tv).tv_usec = 0;
    }
}

#[inline]
pub unsafe fn timersub(a: *const timeval, b: *const timeval, result: *mut timeval) {
    // implemented as a macro by most libc's
    unsafe {
        (*result).tv_sec = (*a).tv_sec - (*b).tv_sec;
        (*result).tv_usec = (*a).tv_usec - (*b).tv_usec;
        if (*result).tv_usec < 0 {
            (*result).tv_sec -= 1;
            (*result).tv_usec += 1000000;
        }
    }
}

pub struct timer(*const libc::timeval);
impl timer {
    pub unsafe fn new(ptr: *const libc::timeval) -> Self { Self(ptr) }
}
impl Eq for timer {}
impl PartialEq for timer {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.0).tv_sec == (*other.0).tv_sec && (*self.0).tv_usec == (*other.0).tv_usec }
    }
}
impl Ord for timer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.partial_cmp(other).unwrap() }
}
impl PartialOrd for timer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        unsafe {
            if (*self.0).tv_sec == (*other.0).tv_sec {
                (*self.0).tv_usec.partial_cmp(&(*other.0).tv_usec)
            } else {
                (*self.0).tv_sec.partial_cmp(&(*other.0).tv_sec)
            }
        }
    }
}

macro_rules! function_name {
    () => {{
        fn f() {}
        const { trim_fn(f) }
    }};
}
// https://stackoverflow.com/questions/38088067/equivalent-of-func-or-function-in-rust
const fn trim_fn<T: Copy>(_: T) -> &'static str {
    let name = std::intrinsics::type_name::<T>();
    let (name_without_last_three, _) = name.split_at(name.len() - 3);
    let bytes = name_without_last_three.as_bytes();

    let mut pos = None;
    let mut i = (bytes.len() - 1) as isize;
    while i >= 0 {
        if bytes[i as usize] == b':' {
            pos = Some(i as usize);
            break;
        }

        i -= 1;
    }

    match pos {
        Some(pos) => name_without_last_three.split_at(pos + 1).1,
        None => name_without_last_three,
    }
}
