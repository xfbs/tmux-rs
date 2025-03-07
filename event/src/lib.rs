#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![feature(c_variadic)]

use ::core::ffi::{c_char, c_int, c_short, c_uchar, c_void};
use ::libc::timeval;

unsafe extern "C" {
    pub fn evbuffer_add_printf(buf: *mut evbuffer, fmt: *const c_char, ...) -> i32;
    pub fn evbuffer_add_vprintf(buf: *mut evbuffer, fmt: *const c_char, ap: core::ffi::va_list::VaList) -> i32;
}

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

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// /usr/include/event2/event.h

// #define evtimer_set(ev, cb, arg)	event_set((ev), -1, 0, (cb), (arg))
pub unsafe extern "C" fn evtimer_set(
    ev: *mut event,
    cb: Option<unsafe extern "C" fn(_: c_int, _: c_short, _: *mut c_void)>,
    arg: *mut c_void,
) {
    unsafe {
        event_set(ev, -1, 0, cb, arg);
    }
}

// #define evtimer_add(ev, tv)		event_add((ev), (tv))
pub unsafe extern "C" fn evtimer_add(ev: *mut event, tv: *const timeval) -> c_int { unsafe { event_add(ev, tv) } }

pub unsafe extern "C" fn evtimer_initialized(ev: *mut event) -> c_int { unsafe { event_initialized(ev) } }

// #define evtimer_del(ev)			event_del(ev)
pub unsafe extern "C" fn evtimer_del(ev: *mut event) -> c_int { unsafe { event_del(ev) } }

// #define evtimer_pending(ev, tv)		event_pending((ev), EV_TIMEOUT, (tv))
pub unsafe extern "C" fn evtimer_pending(ev: *const event, tv: *mut libc::timeval) -> c_int {
    unsafe { event_pending(ev, EV_TIMEOUT, tv) }
}

// #define signal_add(ev, tv)		event_add((ev), (tv))
#[inline]
pub unsafe extern "C" fn signal_add(ev: *mut event, tv: *const timeval) -> i32 { unsafe { event_add(ev, tv) } }

// #define signal_set(ev, x, cb, arg)				 event_set((ev), (x), EV_SIGNAL|EV_PERSIST, (cb), (arg))
#[inline]
pub unsafe extern "C" fn signal_set(
    ev: *mut event,
    x: i32,
    cb: Option<unsafe extern "C" fn(c_int, c_short, *mut c_void)>,
    arg: *mut c_void,
) {
    unsafe { event_set(ev, x, EV_SIGNAL | EV_PERSIST, cb, arg) }
}

// #define signal_del(ev)			event_del(ev)
// #define signal_pending(ev, tv)		event_pending((ev), EV_SIGNAL, (tv))
// #define signal_initialized(ev)		event_initialized(ev)

#[inline]
pub unsafe fn EVBUFFER_LENGTH(x: *mut evbuffer) -> usize { unsafe { evbuffer_get_length(x) } }

#[inline]
pub unsafe fn EVBUFFER_DATA(x: *mut evbuffer) -> *mut c_uchar { unsafe { evbuffer_pullup(x, -1) } }

#[inline]
pub unsafe fn EVBUFFER_OUTPUT(x: *mut bufferevent) -> *mut evbuffer { unsafe { bufferevent_get_output(x) } }
