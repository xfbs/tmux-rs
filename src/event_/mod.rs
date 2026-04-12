#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use std::{
    ffi::{c_int, c_short, c_void},
    option::Option,
    ptr::NonNull,
};

use ::libc::timeval;

// ---------------------------------------------------------------------------
// Shared constants
// ---------------------------------------------------------------------------

pub const EVLOOP_NO_EXIT_ON_EMPTY: i32 = 0x04;
pub const EVLOOP_NONBLOCK: i32 = 0x02;
pub const EVLOOP_ONCE: i32 = 0x01;

pub const EV_CLOSED: i16 = 0x80;
pub const EV_ET: i16 = 0x20;
pub const EV_FINALIZE: i16 = 0x40;
pub const EV_PERSIST: i16 = 0x10;
pub const EV_READ: i16 = 0x02;
pub const EV_SIGNAL: i16 = 0x08;
pub const EV_TIMEOUT: i16 = 0x01;
pub const EV_WRITE: i16 = 0x04;

// ---------------------------------------------------------------------------
// Shared types
// ---------------------------------------------------------------------------

#[repr(u32)]
pub(crate) enum evbuffer_eol_style {
    EVBUFFER_EOL_ANY = 0,
    EVBUFFER_EOL_CRLF = 1,
    EVBUFFER_EOL_CRLF_STRICT = 2,
    EVBUFFER_EOL_LF = 3,
    EVBUFFER_EOL_NUL = 4,
}

pub type event_log_cb = Option<unsafe extern "C-unwind" fn(severity: c_int, msg: *const u8)>;

// ---------------------------------------------------------------------------
// Backend — calloop-based event loop
// ---------------------------------------------------------------------------

mod event_calloop;
pub use event_calloop::*;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

macro_rules! evbuffer_add_printf {
   ($buf:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::event_::evbuffer_add_vprintf($buf, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use evbuffer_add_printf;

#[expect(clippy::disallowed_methods)]
pub unsafe fn evbuffer_add_vprintf(buf: *mut evbuffer, args: std::fmt::Arguments) -> i32 {
    let s = args.to_string();
    unsafe { evbuffer_add(buf, s.as_ptr().cast(), s.len()) }
}

#[expect(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_LENGTH(x: *mut evbuffer) -> usize {
    unsafe { evbuffer_get_length(x) }
}

#[expect(non_snake_case)]
#[inline]
pub unsafe fn EVBUFFER_DATA(x: *mut evbuffer) -> *mut u8 {
    unsafe { evbuffer_pullup(x, -1) }
}
