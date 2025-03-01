#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]
use core::ffi::{VaList, c_char, c_int, c_void};
use core::ptr::NonNull;
use std::ffi::CStr;

use libc::{__errno_location, calloc, malloc, reallocarray, strdup, strerror, strndup};

use compat_rs::recallocarray;

use crate::log::fatalx_;
use crate::{fatalx, vasprintf, vsnprintf};

#[unsafe(no_mangle)]
pub extern "C" fn xmalloc(size: usize) -> NonNull<c_void> {
    debug_assert!(size != 0, "xmalloc: zero size");

    NonNull::new(unsafe { malloc(size) }).unwrap_or_else(|| panic!("xmalloc: allocating {size}"))
}

#[inline]
pub fn malloc_(size: usize) -> *mut c_void {
    debug_assert!(size != 0);

    unsafe { malloc(size) }
}

pub fn xmalloc_<T>() -> NonNull<T> {
    let size = size_of::<T>();
    NonNull::new(malloc_(size))
        .unwrap_or_else(|| panic!("xmalloc: allocating {size} bytes"))
        .cast()
}

#[inline]
pub fn calloc_(nmemb: usize, size: usize) -> *mut c_void {
    unsafe { calloc(nmemb, size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcalloc(nmemb: usize, size: usize) -> NonNull<c_void> {
    debug_assert!(size != 0 && nmemb != 0, "xcalloc: zero size");

    NonNull::new(calloc_(nmemb, size)).unwrap_or_else(|| panic!("xcalloc: allocating {nmemb} * {size}"))
}

pub fn xcalloc_<T>(nmemb: usize) -> NonNull<T> {
    xcalloc(nmemb, size_of::<T>()).cast()
}

// a new signature could look like:
//  https://doc.rust-lang.org/nightly/core/ptr/index.html#pointer-to-reference-conversion
// - aligned
// - non-null
// - at least size_of::<T> bytes
// - initialized & valid
// - exclusive
//
// I'm not yet sure if it's sound, so probably won't use it yet
pub unsafe trait Zeroable {}
pub fn xcalloc__<'a, T: Zeroable>(nmemb: usize) -> &'a mut [T] {
    let ptr: *mut T = xcalloc(nmemb, size_of::<T>()).cast().as_ptr();
    unsafe { core::slice::from_raw_parts_mut(ptr, nmemb) }
}

pub fn xcalloc1<'a, T: Zeroable>() -> &'a mut T {
    let mut ptr: NonNull<T> = xcalloc(1, size_of::<T>()).cast();
    unsafe { ptr.as_mut() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xrealloc(ptr: *mut c_void, size: usize) -> NonNull<c_void> {
    unsafe { xrealloc_(ptr, size) }
}

pub unsafe fn xrealloc_<T>(ptr: *mut T, size: usize) -> NonNull<T> {
    unsafe { xreallocarray_old(ptr, 1, size) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn xreallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> NonNull<c_void> {
    unsafe { xreallocarray_old(ptr, nmemb, size) }
}

pub unsafe fn xreallocarray_old<T>(ptr: *mut T, nmemb: usize, size: usize) -> NonNull<T> {
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx_(format_args!("xreallocarray: zero size"));
        }

        match NonNull::new(reallocarray(ptr as _, nmemb, size)) {
            None => fatalx_(format_args!(
                "xreallocarray: allocating {nmemb} * {size} bytes: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
            Some(new_ptr) => new_ptr.cast(),
        }
    }
}

pub unsafe fn xreallocarray_<T>(ptr: *mut T, nmemb: usize) -> NonNull<T> {
    let size = size_of::<T>();
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx_(format_args!("xreallocarray: zero size"));
        }

        match NonNull::new(reallocarray(ptr as _, nmemb, size)) {
            None => fatalx_(format_args!(
                "xreallocarray: allocating {nmemb} * {size} bytes: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
            Some(new_ptr) => new_ptr.cast(),
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn xrecallocarray(ptr: *mut c_void, oldnmemb: usize, nmemb: usize, size: usize) -> NonNull<c_void> {
    unsafe { xrecallocarray_(ptr, oldnmemb, nmemb, size) }
}

pub unsafe fn xrecallocarray_<T>(ptr: *mut T, oldnmemb: usize, nmemb: usize, size: usize) -> NonNull<T> {
    if nmemb == 0 || size == 0 {
        panic!("xrecallocarray: zero size");
    }

    NonNull::new(unsafe { recallocarray(ptr as *mut c_void, oldnmemb, nmemb, size) })
        .unwrap_or_else(|| panic!("xrecallocarray: allocating {nmemb} * {size}"))
        .cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrdup(str: *const c_char) -> NonNull<c_char> {
    NonNull::new(unsafe { strdup(str) }).unwrap()
}

pub fn xstrdup_(str: &CStr) -> NonNull<c_char> {
    unsafe { xstrdup(str.as_ptr()) }
}

pub fn xstrdup__<'a>(str: &CStr) -> &'a CStr {
    unsafe { CStr::from_ptr(xstrdup(str.as_ptr()).as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrndup(str: *const c_char, maxlen: usize) -> NonNull<c_char> {
    NonNull::new(unsafe { strndup(str, maxlen) }).unwrap()
}

pub unsafe extern "C" fn xasprintf_(fmt: &CStr, mut args: ...) -> NonNull<c_char> {
    let mut ret = core::ptr::null_mut();
    unsafe { xvasprintf(&raw mut ret, fmt.as_ptr(), args.as_va_list()) };
    NonNull::new(ret).unwrap()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xasprintf(ret: *mut *mut c_char, fmt: *const c_char, mut args: ...) -> c_int {
    unsafe { xvasprintf(ret, fmt, args.as_va_list()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xvasprintf(ret: *mut *mut c_char, fmt: *const c_char, args: VaList) -> c_int {
    unsafe {
        let i = vasprintf(ret, fmt, args);

        if i == -1 {
            panic!("xasprintf");
        }

        i
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xsnprintf(str: *mut c_char, len: usize, fmt: *const c_char, mut args: ...) -> c_int {
    unsafe { xvsnprintf(str, len, fmt, args.as_va_list()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xvsnprintf(str: *mut c_char, len: usize, fmt: *const c_char, args: VaList) -> c_int {
    unsafe {
        if len > i32::MAX as usize {
            panic!("xsnprintf: len > INT_MAX");
        }

        let i = vsnprintf(str, len, fmt, args);
        if i < 0 || i >= len as c_int {
            panic!("xsnprintf: overflow");
        }

        i
    }
}

pub unsafe fn free_<T>(p: *mut T) {
    unsafe { libc::free(p as *mut c_void) }
}

pub unsafe fn memcpy_<T>(dest: *mut T, src: *const T, n: usize) -> *mut T {
    unsafe { libc::memcpy(dest as *mut c_void, src as *const c_void, n).cast() }
}

pub unsafe fn memcpy__<T>(dest: *mut T, src: *const T) -> *mut T {
    unsafe { libc::memcpy(dest as *mut c_void, src as *const c_void, size_of::<T>()).cast() }
}

// TODO struct should have some sort of lifetime
/// Display wrapper for a *c_char pointer
#[repr(transparent)]
struct PercentS(*const u8);
impl PercentS {
    unsafe fn from_raw(s: *const c_char) -> Self {
        PercentS(s as _)
    }
}
impl std::fmt::Display for PercentS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = unsafe { libc::strlen(self.0 as *const i8) };
        let s: &[u8] = unsafe { std::slice::from_raw_parts(self.0, len) };
        let s = std::str::from_utf8(s).expect("invalid utf8 in logging");
        f.write_str(s)
    }
}
