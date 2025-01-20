// libeventsys
use core::ffi::{c_int, c_short, c_uchar, c_void};

pub use libevent_sys::{bufferevent, evbuffer, evbuffer_get_length, evbuffer_pullup, event, event_base};

// #define evtimer_set(ev, cb, arg)	event_set((ev), -1, 0, (cb), (arg))
pub unsafe fn evtimer_set(
    ev: *mut event,
    cb: Option<unsafe extern "C" fn(_: c_int, _: c_short, _: *mut c_void)>,
    arg: *mut c_void,
) {
    unsafe {
        libevent_sys::event_set(ev, -1, 0, cb, arg);
    }
}

// #define evtimer_add(ev, tv)		event_add((ev), (tv))
pub unsafe fn evtimer_add(ev: *mut event, tv: *const libc::timeval) {
    unsafe {
        libevent_sys::event_add(ev, core::mem::transmute(tv));
    }
}

pub unsafe fn EVBUFFER_LENGTH(x: *mut evbuffer) -> usize {
    unsafe { evbuffer_get_length(x) }
}
pub unsafe fn EVBUFFER_DATA(x: *mut evbuffer) -> *mut c_uchar {
    unsafe { evbuffer_pullup(x, -1) }
}
