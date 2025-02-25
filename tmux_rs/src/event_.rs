// libeventsys
use core::ffi::{c_int, c_short, c_uchar, c_void};

use libevent_sys::bufferevent_get_output;
pub use libevent_sys::{bufferevent, evbuffer, evbuffer_get_length, evbuffer_pullup, event, event_base};
// /usr/include/event2/event.h

// #define evtimer_set(ev, cb, arg)	event_set((ev), -1, 0, (cb), (arg))
pub unsafe extern "C" fn evtimer_set(
    ev: *mut event,
    cb: Option<unsafe extern "C" fn(_: c_int, _: c_short, _: *mut c_void)>,
    arg: *mut c_void,
) {
    unsafe {
        libevent_sys::event_set(ev, -1, 0, cb, arg);
    }
}

// #define evtimer_add(ev, tv)		event_add((ev), (tv))
pub unsafe extern "C" fn evtimer_add(ev: *mut event, tv: *const libc::timeval) -> c_int {
    unsafe {
        libevent_sys::event_add(
            ev,
            core::mem::transmute::<*const libc::timeval, *const libevent_sys::timeval>(tv),
        )
    }
}

// #define evtimer_del(ev)			event_del(ev)
pub unsafe extern "C" fn evtimer_del(ev: *mut event) -> c_int {
    unsafe { libevent_sys::event_del(ev) }
}

// #define evtimer_pending(ev, tv)		event_pending((ev), EV_TIMEOUT, (tv))
pub unsafe extern "C" fn evtimer_pending(ev: *const event, tv: *mut libc::timeval) -> c_int {
    unsafe {
        libevent_sys::event_pending(
            ev,
            libevent_sys::EV_TIMEOUT as i16,
            core::mem::transmute::<*mut libc::timeval, *mut libevent_sys::timeval>(tv),
        )
    }
}

#[inline]
pub unsafe fn EVBUFFER_LENGTH(x: *mut evbuffer) -> usize {
    unsafe { evbuffer_get_length(x) }
}
#[inline]
pub unsafe fn EVBUFFER_DATA(x: *mut evbuffer) -> *mut c_uchar {
    unsafe { evbuffer_pullup(x, -1) }
}
#[inline]
pub unsafe fn EVBUFFER_OUTPUT(x: *mut bufferevent) -> *mut evbuffer {
    unsafe { bufferevent_get_output(x) }
}
