//! calloop-backed event loop.
//!
//! Provides safe RAII wrappers (`TimerHandle`, `SignalHandle`, `IoHandle`, defer)
//! and the core `event_init/event_loop` dispatch.

mod event_impl;

use std::ffi::{c_int, c_void};

use crate::{Evbuffer, evbuffer};

/// Opaque `event_base` handle.
#[derive(Debug, Copy, Clone)]
pub struct event_base {
    _unused: [u8; 0],
}

// Re-export all public API functions.
pub use event_impl::*;

// ---------------------------------------------------------------------------
// Evbuffer shim functions — bridge the C-shaped API to Evbuffer methods.
// ---------------------------------------------------------------------------

use super::evbuffer_eol_style;

pub fn evbuffer_new() -> *mut evbuffer {
    Box::into_raw(Box::new(Evbuffer::new()))
}

pub unsafe fn evbuffer_free(buf: *mut evbuffer) {
    if !buf.is_null() {
        unsafe { drop(Box::from_raw(buf)); }
    }
}

pub unsafe fn evbuffer_get_length(buf: *const evbuffer) -> usize {
    if buf.is_null() { return 0; }
    unsafe { (*buf).len() }
}

pub unsafe fn evbuffer_add(buf: *mut evbuffer, data: *const c_void, datlen: usize) -> c_int {
    if buf.is_null() || (data.is_null() && datlen > 0) { return -1; }
    unsafe {
        (*buf).add(std::slice::from_raw_parts(data.cast::<u8>(), datlen));
    }
    0
}

pub unsafe fn evbuffer_drain(buf: *mut evbuffer, len: usize) -> c_int {
    if buf.is_null() { return -1; }
    unsafe { (*buf).drain(len); }
    0
}

pub unsafe fn evbuffer_pullup(buf: *mut evbuffer, _size: isize) -> *mut u8 {
    if buf.is_null() { return std::ptr::null_mut(); }
    unsafe { (*buf).as_mut_ptr() }
}

pub unsafe fn evbuffer_readln(
    buffer: *mut evbuffer,
    n_read_out: *mut usize,
    _eol_style: evbuffer_eol_style,
) -> *mut u8 {
    if buffer.is_null() { return std::ptr::null_mut(); }
    unsafe {
        match (*buffer).readln_lf() {
            Some(line) => {
                if !n_read_out.is_null() {
                    *n_read_out = line.len();
                }
                let out = libc::malloc(line.len() + 1) as *mut u8;
                if !out.is_null() {
                    std::ptr::copy_nonoverlapping(line.as_ptr(), out, line.len());
                    *out.add(line.len()) = 0;
                }
                out
            }
            None => std::ptr::null_mut(),
        }
    }
}

/// Read and remove a line (NUL-terminated), legacy API.
pub unsafe fn evbuffer_readline(buffer: *mut evbuffer) -> *mut u8 {
    unsafe { evbuffer_readln(buffer, std::ptr::null_mut(), evbuffer_eol_style::EVBUFFER_EOL_ANY) }
}

pub unsafe fn evbuffer_read(buffer: *mut evbuffer, fd: c_int, howmuch: c_int) -> c_int {
    if buffer.is_null() { return -1; }
    unsafe { (*buffer).read_from_fd(fd, howmuch) }
}

pub unsafe fn evbuffer_write(buffer: *mut evbuffer, fd: c_int) -> c_int {
    if buffer.is_null() { return -1; }
    unsafe { (*buffer).write_to_fd(fd) }
}
