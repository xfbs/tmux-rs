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
    unsafe {
        if size == 0 {
            fatalx_(format_args!("xmalloc: zero size"));
        }

        match NonNull::new(malloc(size)) {
            None => fatalx_(format_args!(
                "xmalloc: allocating {size} bytes: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
            Some(ptr) => ptr,
        }
    }
}

pub fn xmalloc_<T>() -> NonNull<T> {
    xmalloc(size_of::<T>()).cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn xcalloc(nmemb: usize, size: usize) -> NonNull<c_void> {
    unsafe {
        if size == 0 || nmemb == 0 {
            fatalx_(format_args!("xcalloc: zero size"));
        }

        match NonNull::new(calloc(nmemb, size)) {
            None => fatalx_(format_args!(
                "xcalloc: allocating {nmemb} * {size} bytes: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
            Some(ptr) => ptr,
        }
    }
}

pub fn xcalloc_<T>(nmemb: usize) -> NonNull<T> {
    xcalloc(nmemb, size_of::<T>()).cast()
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
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx_(format_args!("xrecallocarray: zero size"));
        }

        match NonNull::new(recallocarray(ptr as _, oldnmemb, nmemb, size)) {
            None => fatalx_(format_args!(
                "xrecallocarray: allocating {nmemb} * {size} bytes: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
            Some(new_ptr) => new_ptr.cast(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrdup(str: *const c_char) -> NonNull<c_char> {
    unsafe {
        match NonNull::new(strdup(str)) {
            Some(cp) => cp,
            None => fatalx_(format_args!(
                "xstrdup: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
        }
    }
}

pub fn xstrdup_(str: &CStr) -> NonNull<c_char> {
    unsafe { xstrdup(str.as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrndup(str: *const c_char, maxlen: usize) -> NonNull<c_char> {
    unsafe {
        match NonNull::new(strndup(str, maxlen)) {
            Some(cp) => cp,
            None => fatalx_(format_args!(
                "xstrndup: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            )),
        }
    }
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
            fatalx_(format_args!(
                "xasprintf: {}",
                PercentS::from_raw(strerror(*__errno_location()))
            ));
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
            fatalx_(format_args!("xsnprintf: len > INT_MAX"));
        }

        let i = vsnprintf(str, len, fmt, args);
        if i < 0 || i >= len as c_int {
            fatalx_(format_args!("xsnprintf: overflow"));
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
