//! calloop-backed implementation of libevent's event API.
//!
//! Provides `event_init`, `event_set`, `event_add`, `event_del`,
//! `event_loop`, `event_active`, `event_pending`, etc.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_int, c_short, c_void};
use std::io;
use std::os::fd::{FromRawFd, OwnedFd};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Safe timer API
// ---------------------------------------------------------------------------

/// RAII handle for a registered timer.  Dropping it deregisters the timer
/// from the event loop automatically.
pub struct TimerHandle {
    id: u64,
}

impl Drop for TimerHandle {
    fn drop(&mut self) {
        let base_ptr = unsafe { GLOBAL_BASE };
        if base_ptr.is_null() {
            return;
        }
        let base = unsafe { &mut *base_ptr };
        if let Some(reg) = base.registrations.remove(&self.id) {
            base.handle.remove(reg.token);
        }
        base.timer_callbacks.remove(&self.id);
        // Mark cancelled so any already-queued ReadyEvent for this id is
        // skipped in the current dispatch cycle.
        CANCELLED.with(|cell| { cell.borrow_mut().insert(self.id); });
    }
}

// ---------------------------------------------------------------------------
// Safe I/O event API
// ---------------------------------------------------------------------------

/// RAII handle for a registered I/O event.  Dropping it deregisters the
/// fd from the event loop.
pub struct IoHandle {
    id: u64,
}

impl Drop for IoHandle {
    fn drop(&mut self) {
        let base_ptr = unsafe { GLOBAL_BASE };
        if base_ptr.is_null() {
            return;
        }
        let base = unsafe { &mut *base_ptr };
        if let Some(reg) = base.registrations.remove(&self.id) {
            base.handle.remove(reg.token);
        }
        base.timer_callbacks.remove(&self.id);
        base.fallback_events.remove(&self.id);
        CANCELLED.with(|cell| { cell.borrow_mut().insert(self.id); });
    }
}

/// Register an I/O event that calls `callback` when the fd is readable
/// and/or writable.  The callback receives `(fd, fired_events)`.
///
/// `events` should be a combination of `EV_READ` and `EV_WRITE`.
/// `EV_PERSIST` is always implied — the registration stays active until
/// dropped.
///
/// Returns `Some(IoHandle)` on success.  Drop the handle to deregister.
pub fn io_register(
    fd: c_int,
    events: c_short,
    callback: Box<dyn Fn(c_int, c_short)>,
) -> Option<IoHandle> {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() || fd < 0 {
        return None;
    }
    let base = unsafe { &mut *base_ptr };
    let id = base.alloc_id();

    let interest = match ((events & super::super::EV_READ) != 0, (events & super::super::EV_WRITE) != 0) {
        (true, true) => Interest::BOTH,
        (true, false) => Interest::READ,
        (false, true) => Interest::WRITE,
        _ => return None,
    };

    // Dup the fd so calloop gets its own epoll registration.
    let dup_fd = unsafe { libc::dup(fd) };
    if dup_fd < 0 {
        return None;
    }
    let owned_fd = unsafe { OwnedFd::from_raw_fd(dup_fd) };
    let generic = Generic::new(owned_fd, interest, Mode::Level);

    match base.handle.insert_source(generic, move |readiness, _, data| {
        let mut fired: c_short = 0;
        if readiness.readable {
            fired |= super::super::EV_READ;
        }
        if readiness.writable {
            fired |= super::super::EV_WRITE;
        }
        if fired != 0 {
            data.ready.push(ReadyEvent {
                id,
                fd,
                events: fired,
                callback: None,
                arg: std::ptr::null_mut(),
            });
        }
        Ok(PostAction::Continue)
    }) {
        Ok(token) => {
            base.registrations.insert(id, Registration { token });
            // Store the callback as a Fn(c_int, c_short) wrapper.
            base.io_callbacks.insert(id, callback);
            Some(IoHandle { id })
        }
        Err(_) => {
            // Non-epolable fd (regular file, etc.) — store as fallback.
            // The closure is called from the fallback drain path.
            base.fallback_events.insert(id, FallbackEvent {
                id,
                fd,
                events: events & (super::super::EV_READ | super::super::EV_WRITE),
                callback: None,
                arg: std::ptr::null_mut(),
            });
            base.io_callbacks.insert(id, callback);
            Some(IoHandle { id })
        }
    }
}

// ---------------------------------------------------------------------------
// Safe deferred callback API
// ---------------------------------------------------------------------------

/// Defer a callback to fire on the next event loop iteration.
///
/// Used for resource cleanup where the callback needs to run *after* the
/// current call stack unwinds — e.g. `server_client_free` must not run
/// while we're still inside a callback that references the client.
///
/// The callback is `FnOnce` — it fires exactly once, then is dropped.
/// Unlike `timer_add`, this returns no handle: the caller cannot cancel
/// the deferred work.
pub fn defer(callback: Box<dyn FnOnce()>) {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return;
    }
    let base = unsafe { &mut *base_ptr };
    let id = base.alloc_id();

    let timer = Timer::immediate();
    let res = base.handle.insert_source(timer, move |_, _, data| {
        data.ready.push(ReadyEvent {
            id,
            fd: -1,
            events: super::super::EV_TIMEOUT,
            callback: None,
            arg: std::ptr::null_mut(),
        });
        TimeoutAction::Drop
    });
    if let Ok(token) = res {
        base.registrations.insert(id, Registration { token });
        base.deferred_callbacks.insert(id, Some(callback));
    }
}

// ---------------------------------------------------------------------------
// Safe signal API
// ---------------------------------------------------------------------------

/// RAII handle for a registered signal.  Dropping it deregisters the signal
/// handler from the event loop automatically.
pub struct SignalHandle {
    id: u64,
}

impl Drop for SignalHandle {
    fn drop(&mut self) {
        let base_ptr = unsafe { GLOBAL_BASE };
        if base_ptr.is_null() {
            return;
        }
        let base = unsafe { &mut *base_ptr };
        if let Some(reg) = base.registrations.remove(&self.id) {
            base.handle.remove(reg.token);
        }
        base.timer_callbacks.remove(&self.id);
        CANCELLED.with(|cell| { cell.borrow_mut().insert(self.id); });
    }
}

/// Register a signal handler that calls `callback` when `signum` is delivered.
///
/// Returns `Some(SignalHandle)` on success.  Drop the handle to deregister.
pub fn signal_register(signum: c_int, callback: Box<dyn Fn()>) -> Option<SignalHandle> {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return None;
    }
    let base = unsafe { &mut *base_ptr };
    let id = base.alloc_id();

    let signal = signal_from_number(signum)?;
    let signals = Signals::new(&[signal]).ok()?;
    let token = base.handle.insert_source(signals, move |_, _, data| {
        data.ready.push(ReadyEvent {
            id,
            fd: signum,
            events: super::super::EV_SIGNAL,
            callback: None,
            arg: std::ptr::null_mut(),
        });
    }).ok()?;

    base.registrations.insert(id, Registration { token });
    base.timer_callbacks.insert(id, callback);
    Some(SignalHandle { id })
}

// ---------------------------------------------------------------------------
// Safe timer API (continued)
// ---------------------------------------------------------------------------

/// Register a one-shot timer that fires after `duration` and calls `callback`.
///
/// Returns `Some(TimerHandle)` on success.  Drop the handle to cancel.
pub fn timer_add(duration: Duration, callback: Box<dyn Fn()>) -> Option<TimerHandle> {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return None;
    }
    let base = unsafe { &mut *base_ptr };
    let id = base.alloc_id();

    let timer = Timer::from_duration(duration);
    let token = base.handle.insert_source(timer, move |_, _, data| {
        data.ready.push(ReadyEvent {
            id,
            fd: -1,
            events: super::super::EV_TIMEOUT,
            callback: None,
            arg: std::ptr::null_mut(),
        });
        TimeoutAction::Drop
    }).ok()?;

    base.registrations.insert(id, Registration { token });
    base.timer_callbacks.insert(id, callback);
    Some(TimerHandle { id })
}

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

/// An event on an fd that epoll can't monitor (regular files, etc.).
/// Treated as always-ready — fires on every event_loop iteration.
struct FallbackEvent {
    id: u64,
    fd: c_int,
    events: c_short,
    callback: Option<unsafe extern "C-unwind" fn(arg1: c_int, arg2: c_short, arg3: *mut c_void)>,
    arg: *mut c_void,
}

/// The calloop-backed event base.
///
/// Owns the calloop `EventLoop` and tracks registrations.
pub(crate) struct EventBase {
    event_loop: EventLoop<'static, LoopData>,
    handle: LoopHandle<'static, LoopData>,
    registrations: HashMap<u64, Registration>,
    /// Events on non-epolable fds (regular files, etc.).  These fire on
    /// every `event_loop` iteration because the fd is always ready.
    fallback_events: HashMap<u64, FallbackEvent>,
    /// Closures for safe timer API (`timer_add`).  Keyed by event id.
    timer_callbacks: HashMap<u64, Box<dyn Fn()>>,
    /// Closures for safe I/O API (`io_register`).  Keyed by event id.
    /// Takes `(fd, fired_events)`.
    io_callbacks: HashMap<u64, Box<dyn Fn(c_int, c_short)>>,
    /// One-shot closures for `defer()`.  Option so dispatch can `.take()`
    /// and move out the FnOnce.  Keyed by event id.
    deferred_callbacks: HashMap<u64, Option<Box<dyn FnOnce()>>>,
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
            fallback_events: HashMap::new(),
            timer_callbacks: HashMap::new(),
            io_callbacks: HashMap::new(),
            deferred_callbacks: HashMap::new(),
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
    //
    // Note: the old EventBase is intentionally leaked (not dropped) because
    // proc_clear_signals already called event_del on events that reference
    // the old base, and dropping it would double-free calloop's internal
    // signal state (signalfd/signal_hook).
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

    // A non-null timeout pointer is always valid, even if {0,0} — that means
    // "fire immediately on next dispatch" (zero-duration timer).  The old code
    // treated {0,0} as "no timeout", which made event_once(..., NULL) a no-op.
    let has_timeout = !timeout.is_null();
    let timeout_duration = if has_timeout {
        Some(timeval_to_duration(unsafe { &*timeout }))
    } else {
        None
    };

    // Remove existing registration if any, then re-register.
    if let Some(reg) = base.registrations.remove(&ev.id) {
        base.handle.remove(reg.token);
    }
    base.fallback_events.remove(&ev.id);

    let id = ev.id;
    let fd = ev.ev_fd;
    let events = ev.ev_events;
    let callback = ev.ev_callback;
    let arg = ev.ev_arg;
    let persist = (events & EV_PERSIST) != 0;

    // --- Signal events ---
    if (events & EV_SIGNAL) != 0 {
        if let Some(signal) = signal_from_number(fd) {
            match Signals::new(&[signal]) {
                Ok(signals) => {
                    match base.handle.insert_source(signals, move |evt, _, data| {
                        data.ready.push(ReadyEvent {
                            id,
                            fd: evt.signal() as c_int,
                            events: EV_SIGNAL,
                            callback,
                            arg,
                        });
                    }) {
                        Ok(token) => {
                            base.registrations.insert(id, Registration { token });
                        }
                        Err(e) => {
                            log_debug!("event_add: signal insert_source failed for signal {fd}: {e}");
                        }
                    }
                }
                Err(e) => {
                    log_debug!("event_add: Signals::new failed for signal {fd}: {e}");
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

        match base.handle.insert_source(generic, move |readiness, _, data| {
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
            Ok(PostAction::Continue)
        }) {
            Ok(token) => {
                base.registrations.insert(id, Registration { token });
            }
            Err(_) => {
                // epoll_ctl(ADD) returns EPERM for regular files and some
                // special fds.  libevent falls back to poll()/select();
                // we emulate that by treating the fd as always-ready and
                // firing the callback on every event_loop iteration.
                base.fallback_events.insert(id, FallbackEvent {
                    id,
                    fd,
                    events: events & (EV_READ | EV_WRITE),
                    callback,
                    arg,
                });
            }
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
        base.fallback_events.remove(&ev.id);
    }

    // Mark this event as cancelled so the dispatch loop skips it if it
    // was already collected as ready in this iteration.
    CANCELLED.with(|cell| { cell.borrow_mut().insert(ev.id); });

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
    /// Event IDs deleted during a dispatch cycle.  Checked before firing each
    /// callback so we skip events whose underlying resource was freed by an
    /// earlier callback in the same iteration (e.g. SIGCHLD destroying a pane
    /// whose read event is also ready).
    static CANCELLED: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
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

    // Drain fallback events (non-epolable fds like regular files).
    // These are always ready, so they fire on every iteration.
    for fb in base.fallback_events.values() {
        data.ready.push(ReadyEvent {
            id: fb.id,
            fd: fb.fd,
            events: fb.events,
            callback: fb.callback,
            arg: fb.arg,
        });
    }

    let has_fallback = !base.fallback_events.is_empty();
    if (flags & EVLOOP_NONBLOCK) != 0 {
        let _ = base.event_loop.dispatch(Some(Duration::ZERO), &mut data);
    } else if has_fallback {
        // Fallback events (non-epolable fds) are always ready, so data.ready
        // is never empty when they exist.  We still need to poll epoll for
        // real I/O and signals, but can't block forever — use a short timeout.
        let _ = base.event_loop.dispatch(Some(Duration::from_millis(1)), &mut data);
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

    // Clear the cancelled set before dispatching.
    CANCELLED.with(|cell| cell.borrow_mut().clear());

    // Dispatch all ready events, skipping any that were cancelled by an
    // earlier callback in this cycle (e.g. SIGCHLD handler destroying a
    // pane whose bufferevent read event is also in the ready list).
    for ready in data.ready {
        let cancelled = CANCELLED.with(|cell| cell.borrow().contains(&ready.id));
        if cancelled {
            continue;
        }
        if let Some(cb) = ready.callback {
            unsafe { cb(ready.fd, ready.events, ready.arg); }
        } else if let Some(cb) = base.io_callbacks.get(&ready.id) {
            cb(ready.fd, ready.events);
        } else if let Some(cb) = base.timer_callbacks.get(&ready.id) {
            cb();
        } else if let Some(slot) = base.deferred_callbacks.get_mut(&ready.id)
            && let Some(cb) = slot.take()
        {
            // One-shot: remove the registration and invoke the FnOnce.
            base.deferred_callbacks.remove(&ready.id);
            base.registrations.remove(&ready.id);
            cb();
        }
    }

    0
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
