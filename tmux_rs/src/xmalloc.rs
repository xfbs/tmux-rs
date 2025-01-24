#![allow(unused_variables)]
#![allow(clippy::missing_safety_doc)]
use core::ffi::{VaList, c_char, c_int, c_void};
use core::ptr::NonNull;

use libc::{__errno_location, calloc, malloc, reallocarray, strdup, strerror, strndup};

use compat_rs::recallocarray;

use crate::{fatalx, vasprintf, vsnprintf};

#[unsafe(no_mangle)]
pub extern "C" fn xmalloc(size: usize) -> NonNull<c_void> {
    unsafe {
        if size == 0 {
            fatalx(c"xmalloc: zero size".as_ptr());
        }

        match NonNull::new(malloc(size)) {
            None => fatalx(
                c"xmalloc: allocating %zu bytes: %s".as_ptr(),
                size,
                strerror(*__errno_location()),
            ),
            Some(ptr) => ptr,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn xcalloc(nmemb: usize, size: usize) -> NonNull<c_void> {
    unsafe {
        if size == 0 || nmemb == 0 {
            fatalx(c"xcalloc: zero size".as_ptr());
        }

        match NonNull::new(calloc(nmemb, size)) {
            None => fatalx(
                c"xcalloc: allocating %zu * %zu bytes: %s".as_ptr(),
                nmemb,
                size,
                strerror(*__errno_location()),
            ),
            Some(ptr) => ptr,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xrealloc(ptr: *mut c_void, size: usize) -> NonNull<c_void> {
    unsafe { xreallocarray(ptr, 1, size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xreallocarray(ptr: *mut c_void, nmemb: usize, size: usize) -> NonNull<c_void> {
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx(c"xreallocarray: zero size".as_ptr());
        }

        match NonNull::new(reallocarray(ptr, nmemb, size)) {
            None => fatalx(
                c"xreallocarray: allocating %zu * %zu bytes: %s".as_ptr(),
                nmemb,
                size,
                strerror(*__errno_location()),
            ),
            Some(new_ptr) => new_ptr,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xrecallocarray(
    ptr: *mut c_void,
    oldnmemb: usize,
    nmemb: usize,
    size: usize,
) -> NonNull<c_void> {
    unsafe {
        if nmemb == 0 || size == 0 {
            fatalx(c"xrecallocarray: zero size".as_ptr());
        }

        match NonNull::new(recallocarray(ptr, oldnmemb, nmemb, size)) {
            None => fatalx(
                c"xrecallocarray: allocating %zu * %zu bytes: %s".as_ptr(),
                nmemb,
                size,
                strerror(*__errno_location()),
            ),
            Some(new_ptr) => new_ptr,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrdup(str: *const c_char) -> NonNull<c_char> {
    unsafe {
        match NonNull::new(strdup(str)) {
            Some(cp) => cp,
            None => fatalx(c"xstrdup: %s".as_ptr(), strerror(*__errno_location())),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn xstrndup(str: *const c_char, maxlen: usize) -> NonNull<c_char> {
    unsafe {
        match NonNull::new(strndup(str, maxlen)) {
            Some(cp) => cp,
            None => fatalx(c"xstrndup: %s".as_ptr(), strerror(*__errno_location())),
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
            fatalx(c"xasprintf: %s".as_ptr(), strerror(*__errno_location()));
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
            fatalx(c"xsnprintf: len > INT_MAX".as_ptr());
        }

        let i = vsnprintf(str, len, fmt, args);
        if i < 0 || i >= len as c_int {
            fatalx(c"xsnprintf: overflow".as_ptr());
        }

        i
    }
}
