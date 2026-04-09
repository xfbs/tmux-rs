//! calloop-backed implementation of libevent's bufferevent API.
//!
//! A bufferevent provides buffered I/O on a file descriptor with
//! user-supplied read/write/error callbacks.

use std::ffi::{c_int, c_short, c_void};

use super::super::{EV_PERSIST, EV_READ, EV_WRITE};
use super::{bufferevent, event, event_base};
use crate::evbuffer_::Evbuffer;
use ::libc::timeval;

use super::event_impl;
use super::event_watermark;

// ---------------------------------------------------------------------------
// Internal callbacks
// ---------------------------------------------------------------------------

/// Internal read callback: reads data from fd into input buffer,
/// then calls the user's read callback.
unsafe extern "C-unwind" fn bufferevent_readcb(fd: c_int, _events: c_short, arg: *mut c_void) {
    let bev = arg as *mut bufferevent;
    if bev.is_null() {
        return;
    }
    unsafe {
        let input = (*bev).input;
        let n = (*input).read_from_fd(fd, 4096);
        if n > 0 {
            if let Some(cb) = (*bev).readcb {
                cb(bev, (*bev).cbarg);
            }
        } else if n == 0
            || (n < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock)
        {
            if let Some(cb) = (*bev).errorcb {
                let what: c_short = EV_READ | 0x01;
                cb(bev, what, (*bev).cbarg);
            }
        }
    }
}

/// Internal write callback: writes data from output buffer to fd,
/// then calls the user's write callback if buffer is drained.
unsafe extern "C-unwind" fn bufferevent_writecb(fd: c_int, _events: c_short, arg: *mut c_void) {
    let bev = arg as *mut bufferevent;
    if bev.is_null() {
        return;
    }
    unsafe {
        let output = (*bev).output;
        if (*output).len() > 0 {
            let n = (*output).write_to_fd(fd);
            if n < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                    return;
                }
                if let Some(cb) = (*bev).errorcb {
                    let what: c_short = EV_WRITE | 0x01;
                    cb(bev, what, (*bev).cbarg);
                }
                return;
            }
        }

        if (*output).len() == 0 {
            event_impl::event_del(&raw mut (*bev).ev_write);
            if let Some(cb) = (*bev).writecb {
                cb(bev, (*bev).cbarg);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub unsafe fn bufferevent_new(
    fd: c_int,
    readcb: super::bufferevent_data_cb,
    writecb: super::bufferevent_data_cb,
    errorcb: super::bufferevent_event_cb,
    cbarg: *mut c_void,
) -> *mut bufferevent {
    let input = Box::into_raw(Box::new(Evbuffer::new()));
    let output = Box::into_raw(Box::new(Evbuffer::new()));

    let bev = Box::new(bufferevent {
        ev_base: std::ptr::null_mut(),
        ev_read: unsafe { std::mem::zeroed::<event>() },
        ev_write: unsafe { std::mem::zeroed::<event>() },
        input,
        output,
        wm_read: event_watermark { low: 0, high: 0 },
        wm_write: event_watermark { low: 0, high: 0 },
        readcb,
        writecb,
        errorcb,
        cbarg,
        timeout_read: timeval { tv_sec: 0, tv_usec: 0 },
        timeout_write: timeval { tv_sec: 0, tv_usec: 0 },
        enabled: 0,
    });

    let ptr = Box::into_raw(bev);

    unsafe {
        event_impl::event_set(
            &raw mut (*ptr).ev_read,
            fd,
            EV_READ | EV_PERSIST,
            Some(bufferevent_readcb),
            ptr as *mut c_void,
        );
        event_impl::event_set(
            &raw mut (*ptr).ev_write,
            fd,
            EV_WRITE | EV_PERSIST,
            Some(bufferevent_writecb),
            ptr as *mut c_void,
        );
    }

    ptr
}

pub unsafe fn bufferevent_free(bufev: *mut bufferevent) {
    if bufev.is_null() {
        return;
    }
    unsafe {
        event_impl::event_del(&raw mut (*bufev).ev_read);
        event_impl::event_del(&raw mut (*bufev).ev_write);
        drop(Box::from_raw((*bufev).input));
        drop(Box::from_raw((*bufev).output));
        drop(Box::from_raw(bufev));
    }
}

pub unsafe fn bufferevent_write(
    bufev: *mut bufferevent,
    data: *const c_void,
    size: usize,
) -> c_int {
    if bufev.is_null() {
        return -1;
    }
    unsafe {
        let output = (*bufev).output;
        let slice = std::slice::from_raw_parts(data.cast::<u8>(), size);
        (*output).add(slice);

        if ((*bufev).enabled & EV_WRITE) != 0 {
            event_impl::event_add(&raw mut (*bufev).ev_write, std::ptr::null());
        }
    }
    0
}

pub unsafe fn bufferevent_write_buffer(
    bufev: *mut bufferevent,
    buf: *mut Evbuffer,
) -> c_int {
    if bufev.is_null() || buf.is_null() {
        return -1;
    }
    unsafe {
        let data = (*buf).as_slice();
        if data.is_empty() {
            return 0;
        }
        let output = (*bufev).output;
        (*output).add(data);
        let len = (*buf).len();
        (*buf).drain(len);

        if ((*bufev).enabled & EV_WRITE) != 0 {
            event_impl::event_add(&raw mut (*bufev).ev_write, std::ptr::null());
        }
    }
    0
}

pub unsafe fn bufferevent_get_output(bufev: *mut bufferevent) -> *mut Evbuffer {
    if bufev.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { (*bufev).output }
}

pub unsafe fn bufferevent_enable(bufev: *mut bufferevent, events: i16) -> c_int {
    if bufev.is_null() {
        return -1;
    }
    unsafe {
        (*bufev).enabled |= events;
        if (events & EV_READ) != 0 {
            event_impl::event_add(&raw mut (*bufev).ev_read, std::ptr::null());
        }
        if (events & EV_WRITE) != 0 {
            event_impl::event_add(&raw mut (*bufev).ev_write, std::ptr::null());
        }
    }
    0
}

pub unsafe fn bufferevent_disable(bufev: *mut bufferevent, events: i16) -> c_int {
    if bufev.is_null() {
        return -1;
    }
    unsafe {
        (*bufev).enabled &= !events;
        if (events & EV_READ) != 0 {
            event_impl::event_del(&raw mut (*bufev).ev_read);
        }
        if (events & EV_WRITE) != 0 {
            event_impl::event_del(&raw mut (*bufev).ev_write);
        }
    }
    0
}

pub unsafe fn bufferevent_setwatermark(
    bufev: *mut bufferevent,
    events: i16,
    lowmark: usize,
    highmark: usize,
) {
    if bufev.is_null() {
        return;
    }
    unsafe {
        if (events & EV_READ) != 0 {
            (*bufev).wm_read.low = lowmark;
            (*bufev).wm_read.high = highmark;
        }
        if (events & EV_WRITE) != 0 {
            (*bufev).wm_write.low = lowmark;
            (*bufev).wm_write.high = highmark;
        }
    }
}

pub unsafe fn bufferevent_setcb(
    bufev: *mut bufferevent,
    readcb: super::bufferevent_data_cb,
    writecb: super::bufferevent_data_cb,
    errorcb: super::bufferevent_event_cb,
    cbarg: *mut c_void,
) {
    if bufev.is_null() {
        return;
    }
    unsafe {
        (*bufev).readcb = readcb;
        (*bufev).writecb = writecb;
        (*bufev).errorcb = errorcb;
        (*bufev).cbarg = cbarg;
    }
}
