// Copyright (c) 2024 Patrick Elsen
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

//! Calloop-backed event loop and evbuffer implementation for tmux-rs.
//!
//! Provides safe RAII wrappers (`TimerHandle`, `SignalHandle`, `IoHandle`,
//! `defer`) and the core `event_init`/`event_loop` dispatch, plus a
//! pure-Rust `Evbuffer` replacement for libevent's evbuffer.
//!
//! Module `backend` is named to avoid the name clash with the external
//! `::calloop` crate that it wraps.

#![allow(clippy::transmute_ptr_to_ptr)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use std::ffi::c_int;

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
pub enum evbuffer_eol_style {
    EVBUFFER_EOL_ANY = 0,
    EVBUFFER_EOL_CRLF = 1,
    EVBUFFER_EOL_CRLF_STRICT = 2,
    EVBUFFER_EOL_LF = 3,
    EVBUFFER_EOL_NUL = 4,
}

pub type event_log_cb = Option<unsafe extern "C-unwind" fn(severity: c_int, msg: *const u8)>;

// ---------------------------------------------------------------------------
// Evbuffer — pure-Rust buffered I/O replacement for libevent's evbuffer.
// ---------------------------------------------------------------------------

mod buf;
pub use buf::Evbuffer;
// Legacy lowercase alias, matches the original C type spelling used across
// tmux-rs call sites.
#[allow(non_camel_case_types)]
pub type evbuffer = Evbuffer;

// ---------------------------------------------------------------------------
// Backend — calloop-based event loop (named `backend` to avoid the clash
// with the external `::calloop` crate this module wraps).
// ---------------------------------------------------------------------------

mod backend;
pub use backend::*;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Format into an evbuffer (`format_args!`-based). Convenience macro that
/// expands to `$crate::evbuffer_add_vprintf(buf, format_args!(...))`.
#[macro_export]
macro_rules! evbuffer_add_printf {
   ($buf:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::evbuffer_add_vprintf($buf, format_args!($fmt $(, $args)*))
    };
}

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
