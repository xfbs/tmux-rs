//! calloop-backed event loop replacement for libevent2.
//!
//! This module provides drop-in replacements for libevent's `event`,
//! `bufferevent`, and `evbuffer` types, backed by calloop. The public
//! API matches libevent's C signatures so existing call sites compile
//! unchanged.

mod event_impl;
mod bufferevent_impl;

use std::ffi::{c_int, c_short, c_void};

use super::{bufferevent_data_cb, bufferevent_event_cb, event_log_cb, event_watermark};
use crate::evbuffer_::Evbuffer;
use ::libc::timeval;

/// calloop-backed event struct.
///
/// Replaces libevent's `struct event` with a simplified layout.
/// No code outside event_.rs should access fields directly.
#[repr(C)]
pub struct event {
    /// Unique id assigned by `event_set` (0 = uninitialized).
    pub(crate) id: u64,
    /// File descriptor, or -1 for timers.
    pub(crate) ev_fd: c_int,
    /// Event flags (`EV_READ`, `EV_WRITE`, `EV_SIGNAL`, `EV_PERSIST`, `EV_TIMEOUT`).
    pub(crate) ev_events: c_short,
    /// Pending result flags (set when the event fires).
    pub(crate) ev_res: c_short,
    /// Callback function.
    pub(crate) ev_callback:
        Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    /// Callback argument.
    pub(crate) ev_arg: *mut c_void,
    /// Pointer to the event base this event is registered with.
    pub(crate) ev_base: *mut event_base,
    /// Timeout value.
    pub(crate) ev_timeout: timeval,
    /// Whether this event is currently registered (has a calloop source).
    pub(crate) added: bool,
}

/// Opaque event_base handle.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event_base {
    _unused: [u8; 0],
}

/// calloop-backed bufferevent struct.
///
/// Provides buffered I/O on a file descriptor with read/write callbacks.
#[repr(C)]
pub struct bufferevent {
    pub ev_base: *mut event_base,
    pub ev_read: event,
    pub ev_write: event,
    pub input: *mut Evbuffer,
    pub output: *mut Evbuffer,
    pub wm_read: event_watermark,
    pub wm_write: event_watermark,
    pub readcb: bufferevent_data_cb,
    pub writecb: bufferevent_data_cb,
    pub errorcb: bufferevent_event_cb,
    pub cbarg: *mut c_void,
    pub timeout_read: timeval,
    pub timeout_write: timeval,
    pub enabled: c_short,
}

// Re-export all public API functions.
pub use bufferevent_impl::*;
pub use event_impl::*;

// Re-export Evbuffer as evbuffer so existing code using `*mut evbuffer` works.
pub use crate::evbuffer_::Evbuffer as evbuffer;

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
