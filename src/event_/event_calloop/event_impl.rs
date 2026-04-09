//! calloop-backed implementation of libevent's event API.
//!
//! Provides `event_init`, `event_set`, `event_add`, `event_del`,
//! `event_loop`, `event_active`, `event_pending`, etc.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_int, c_short, c_void};
use std::io;
use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd};
use std::time::Duration;

use calloop::generic::Generic;
use calloop::signals::{Signal, Signals};
use calloop::timer::{TimeoutAction, Timer};
use calloop::{EventLoop, Interest, LoopHandle, Mode, PostAction, RegistrationToken};

use super::{event, event_base};
use super::super::{
    EV_PERSIST, EV_READ, EV_SIGNAL, EV_TIMEOUT, EV_WRITE, EVLOOP_NONBLOCK,
    event_log_cb,
};
use crate::log::log_debug;
use ::libc::timeval;

/// A ready event notification — queued when a calloop source fires.
struct ReadyEvent {
    id: u64,
    fd: c_int,
    events: c_short,
    callback: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
}

/// Registration entry tracked per event id.
struct Registration {
    token: RegistrationToken,
}


/// Shared state passed to calloop callbacks and the dispatch loop.
struct LoopData {
    /// Events ready to be dispatched after calloop returns.
    ready: Vec<ReadyEvent>,
}

/// The calloop-backed event base.
///
/// Owns the calloop `EventLoop` and tracks registrations.
pub(crate) struct EventBase {
    event_loop: EventLoop<'static, LoopData>,
    handle: LoopHandle<'static, LoopData>,
    registrations: HashMap<u64, Registration>,
    next_id: u64,
}

impl EventBase {
    fn new() -> io::Result<Self> {
        let event_loop: EventLoop<LoopData> = EventLoop::try_new()?;
        let handle = event_loop.handle();
        Ok(Self {
            event_loop,
            handle,
            registrations: HashMap::new(),
            next_id: 1,
        })
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Global event base pointer.
static mut GLOBAL_BASE: *mut EventBase = std::ptr::null_mut();

fn timeval_to_duration(tv: &timeval) -> Duration {
    Duration::new(tv.tv_sec as u64, (tv.tv_usec as u32) * 1000)
}

fn signal_from_number(signum: c_int) -> Option<Signal> {
    match signum {
        libc::SIGCHLD => Some(Signal::SIGCHLD),
        libc::SIGHUP => Some(Signal::SIGHUP),
        libc::SIGINT => Some(Signal::SIGINT),
        libc::SIGTERM => Some(Signal::SIGTERM),
        libc::SIGUSR1 => Some(Signal::SIGUSR1),
        libc::SIGUSR2 => Some(Signal::SIGUSR2),
        libc::SIGWINCH => Some(Signal::SIGWINCH),
        libc::SIGCONT => Some(Signal::SIGCONT),
        libc::SIGTSTP => Some(Signal::SIGTSTP),
        libc::SIGPIPE => Some(Signal::SIGPIPE),
        libc::SIGQUIT => Some(Signal::SIGQUIT),
        _ => {
            log_debug!("event_calloop: unsupported signal {signum}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub unsafe fn event_init() -> *mut event_base {
    match EventBase::new() {
        Ok(base) => {
            let ptr = Box::into_raw(Box::new(base));
            unsafe { GLOBAL_BASE = ptr; }
            log_debug!("event_init: calloop base={ptr:p} pid={}", std::process::id());
            ptr as *mut event_base
        }
        Err(e) => {
            log_debug!("event_init: calloop failed: {e}");
            std::ptr::null_mut()
        }
    }
}

pub unsafe fn event_reinit(_base: *mut event_base) -> c_int {
    // After fork: create a fresh event loop. The old one's epoll fd is
    // an inherited copy and won't work correctly in the child.
    match EventBase::new() {
        Ok(base) => {
            let ptr = Box::into_raw(Box::new(base));
            unsafe { GLOBAL_BASE = ptr; }
            0
        }
        Err(_) => -1,
    }
}

pub unsafe fn event_set(
    ev_ptr: *mut event,
    fd: c_int,
    events: c_short,
    cb: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
) {
    if ev_ptr.is_null() {
        return;
    }
    let base = unsafe { GLOBAL_BASE };
    let ev = unsafe { &mut *ev_ptr };
    if ev.id == 0 {
        ev.id = if base.is_null() {
            1
        } else {
            unsafe { (*base).alloc_id() }
        };
    }
    ev.ev_fd = fd;
    ev.ev_events = events;
    ev.ev_callback = cb;
    ev.ev_arg = arg;
    ev.ev_base = base as *mut event_base;
    ev.ev_timeout = timeval { tv_sec: 0, tv_usec: 0 };
    ev.ev_res = 0;
}

pub unsafe fn event_add(ev_ptr: *mut event, timeout: *const timeval) -> c_int {
    if ev_ptr.is_null() {
        return -1;
    }
    let ev = unsafe { &mut *ev_ptr };
    let base_ptr = ev.ev_base as *mut EventBase;
    if base_ptr.is_null() {
        return -1;
    }
    let base = unsafe { &mut *base_ptr };

    if !timeout.is_null() {
        ev.ev_timeout = unsafe { *timeout };
    }

    let has_timeout = !timeout.is_null()
        && unsafe { (*timeout).tv_sec != 0 || (*timeout).tv_usec != 0 };
    let timeout_duration = if has_timeout {
        Some(timeval_to_duration(unsafe { &*timeout }))
    } else {
        None
    };

    // Remove existing registration if any, then re-register.
    if let Some(reg) = base.registrations.remove(&ev.id) {
        base.handle.remove(reg.token);
    }

    let id = ev.id;
    let fd = ev.ev_fd;
    let events = ev.ev_events;
    let callback = ev.ev_callback;
    let arg = ev.ev_arg;
    let persist = (events & EV_PERSIST) != 0;

    // --- Signal events ---
    if (events & EV_SIGNAL) != 0 {
        if let Some(signal) = signal_from_number(fd) {
            if let Ok(signals) = Signals::new(&[signal]) {
                if let Ok(token) = base.handle.insert_source(signals, move |evt, _, data| {
                    data.ready.push(ReadyEvent {
                        id,
                        fd: evt.signal() as c_int,
                        events: EV_SIGNAL,
                        callback,
                        arg,
                    });
                }) {
                    base.registrations.insert(id, Registration { token });
                }
            }
        }
        ev.added = true;
        return 0;
    }

    // --- I/O events (fd >= 0) ---
    if fd >= 0 && (events & (EV_READ | EV_WRITE)) != 0 {
        let interest = match ((events & EV_READ) != 0, (events & EV_WRITE) != 0) {
            (true, true) => Interest::BOTH,
            (true, false) => Interest::READ,
            (false, true) => Interest::WRITE,
            _ => unreachable!(),
        };

        // dup() the fd so calloop gets its own epoll registration.
        // Multiple events can watch the same fd (e.g. bufferevent's ev_read
        // + ev_write), and epoll only allows one registration per fd.
        let dup_fd = unsafe { libc::dup(fd) };
        if dup_fd < 0 {
            return -1;
        }
        let owned_fd = unsafe { OwnedFd::from_raw_fd(dup_fd) };
        let generic = Generic::new(owned_fd, interest, Mode::Level);

        if let Ok(token) = base.handle.insert_source(generic, move |readiness, _, data| {
            let mut fired: c_short = 0;
            if readiness.readable {
                fired |= EV_READ;
            }
            if readiness.writable {
                fired |= EV_WRITE;
            }
            if fired != 0 {
                data.ready.push(ReadyEvent {
                    id,
                    fd,
                    events: fired,
                    callback,
                    arg,
                });
            }
            // Always continue — we handle persistence via event_del in user
            // callbacks. Using PostAction::Remove would cause calloop to
            // internally remove the source, but our registrations HashMap
            // and ev.added flag would go stale.
            Ok(PostAction::Continue)
        }) {
            base.registrations.insert(id, Registration { token });
        }

        ev.added = true;
        return 0;
    }

    // --- Pure timer (fd == -1, no signal, timeout set) ---
    if let Some(dur) = timeout_duration {
        let timer = Timer::from_duration(dur);
        if let Ok(token) = base.handle.insert_source(timer, move |_, _, data| {
            data.ready.push(ReadyEvent {
                id,
                fd,
                events: EV_TIMEOUT,
                callback,
                arg,
            });
            if persist {
                TimeoutAction::ToDuration(dur)
            } else {
                TimeoutAction::Drop
            }
        }) {
            base.registrations.insert(id, Registration { token });
        }
        ev.added = true;
        return 0;
    }

    ev.added = true;
    0
}

pub unsafe fn event_del(ev_ptr: *mut event) -> c_int {
    if ev_ptr.is_null() {
        return -1;
    }
    let ev = unsafe { &mut *ev_ptr };
    if !ev.added {
        return 0;
    }

    let base_ptr = ev.ev_base as *mut EventBase;
    if !base_ptr.is_null() {
        let base = unsafe { &mut *base_ptr };
        if let Some(reg) = base.registrations.remove(&ev.id) {
            base.handle.remove(reg.token);
        }
    }

    ev.added = false;
    0
}

pub unsafe fn event_active(ev_ptr: *mut event, res: c_int, _ncalls: c_short) {
    if ev_ptr.is_null() {
        return;
    }
    let ev = unsafe { &*ev_ptr };
    let base_ptr = ev.ev_base as *mut EventBase;
    if base_ptr.is_null() {
        return;
    }
    PENDING_ACTIVE.with(|cell| {
        cell.borrow_mut().push(ReadyEvent {
            id: ev.id,
            fd: ev.ev_fd,
            events: res as c_short,
            callback: ev.ev_callback,
            arg: ev.ev_arg,
        });
    });
}

thread_local! {
    static PENDING_ACTIVE: RefCell<Vec<ReadyEvent>> = const { RefCell::new(Vec::new()) };
}

pub unsafe fn event_pending(ev_ptr: *const event, events: c_short, tv: *mut timeval) -> c_int {
    if ev_ptr.is_null() {
        return 0;
    }
    let ev = unsafe { &*ev_ptr };
    if !ev.added {
        return 0;
    }
    let mut result: c_int = 0;
    if (events & EV_TIMEOUT) != 0
        && (ev.ev_timeout.tv_sec != 0 || ev.ev_timeout.tv_usec != 0)
    {
        result |= EV_TIMEOUT as c_int;
        if !tv.is_null() {
            unsafe { *tv = ev.ev_timeout; }
        }
    }
    if (events & EV_READ) != 0 && (ev.ev_events & EV_READ) != 0 {
        result |= EV_READ as c_int;
    }
    if (events & EV_WRITE) != 0 && (ev.ev_events & EV_WRITE) != 0 {
        result |= EV_WRITE as c_int;
    }
    if (events & EV_SIGNAL) != 0 && (ev.ev_events & EV_SIGNAL) != 0 {
        result |= EV_SIGNAL as c_int;
    }
    result
}

pub unsafe fn event_initialized(ev_ptr: *const event) -> c_int {
    if ev_ptr.is_null() {
        return 0;
    }
    let ev = unsafe { &*ev_ptr };
    (ev.id != 0) as c_int
}


pub unsafe fn event_loop(flags: c_int) -> c_int {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return -1;
    }
    let base = unsafe { &mut *base_ptr };

    let mut data = LoopData { ready: Vec::new() };

    // Drain any events queued by event_active.
    PENDING_ACTIVE.with(|cell| {
        data.ready.append(&mut cell.borrow_mut());
    });

    if (flags & EVLOOP_NONBLOCK) != 0 {
        let _ = base.event_loop.dispatch(Some(Duration::ZERO), &mut data);
    } else if !data.ready.is_empty() {
        // Already have events from event_active — don't block.
    } else {
        // EVLOOP_ONCE: block until at least one event fires.
        let _ = base.event_loop.dispatch(None, &mut data);
    }

    // Drain any events queued by event_active during dispatch.
    PENDING_ACTIVE.with(|cell| {
        data.ready.append(&mut cell.borrow_mut());
    });

    // Dispatch all ready events.
    for ready in data.ready {
        if let Some(cb) = ready.callback {
            unsafe { cb(ready.fd, ready.events, ready.arg); }
        }
    }

    0
}

pub unsafe fn event_once(
    fd: c_int,
    events: c_short,
    cb: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
    tv: *const timeval,
) -> c_int {
    let ev = Box::into_raw(Box::new(unsafe { std::mem::zeroed::<event>() }));
    unsafe {
        event_set(ev, fd, events & !EV_PERSIST, cb, arg);
        event_add(ev, tv)
    }
}

static VERSION_STR: &[u8] = b"calloop-event/1.0\0";
static METHOD_STR: &[u8] = b"calloop\0";

pub fn event_get_version() -> *const u8 {
    VERSION_STR.as_ptr()
}

pub fn event_get_method() -> *const u8 {
    METHOD_STR.as_ptr()
}

pub fn event_set_log_callback(_cb: event_log_cb) {
    // TODO: wire up to calloop's logging or a global callback.
}
