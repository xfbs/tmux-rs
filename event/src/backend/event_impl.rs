//! calloop-backed event loop implementation.
//!
//! Provides safe RAII wrappers (`TimerHandle`, `SignalHandle`, `IoHandle`,
//! `defer`) and the core `event_init` / `event_loop` dispatch.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{c_int, c_short};
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
        base.io_callbacks.remove(&self.id);
        base.fallback_io.remove(&self.id);
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
            });
        }
        Ok(PostAction::Continue)
    }) {
        Ok(token) => {
            base.registrations.insert(id, Registration { token });
            base.io_callbacks.insert(id, callback);
            Some(IoHandle { id })
        }
        Err(_) => {
            // Non-epolable fd (regular file, etc.) — treat as always-ready.
            // The callback fires on every event_loop iteration.
            base.fallback_io.insert(id, (fd, events & (super::super::EV_READ | super::super::EV_WRITE)));
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
    let res = base.handle.insert_source(timer, move |_, (), data| {
        data.ready.push(ReadyEvent {
            id,
            fd: -1,
            events: super::super::EV_TIMEOUT,
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
    /// On non-Linux platforms, holds the signal-hook registration ID so we
    /// can unregister the signal handler on drop.
    #[cfg(not(target_os = "linux"))]
    sig_id: signal_hook::SigId,
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
        #[cfg(not(target_os = "linux"))]
        signal_hook::low_level::unregister(self.sig_id);
        CANCELLED.with(|cell| { cell.borrow_mut().insert(self.id); });
    }
}

/// Register a signal handler that calls `callback` when `signum` is delivered.
///
/// Returns `Some(SignalHandle)` on success.  Drop the handle to deregister.
///
/// On Linux this uses `signalfd(2)` via calloop's built-in `Signals` source.
/// On other platforms (macOS, OpenBSD) it uses a self-pipe with `signal-hook`.
#[cfg(target_os = "linux")]
pub fn signal_register(signum: c_int, callback: Box<dyn Fn()>) -> Option<SignalHandle> {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return None;
    }
    let base = unsafe { &mut *base_ptr };
    let id = base.alloc_id();

    let signal = signal_from_number(signum)?;
    let signals = Signals::new(&[signal]).ok()?;
    let token = base.handle.insert_source(signals, move |_, (), data| {
        data.ready.push(ReadyEvent {
            id,
            fd: signum,
            events: super::super::EV_SIGNAL,
        });
    }).ok()?;

    base.registrations.insert(id, Registration { token });
    base.timer_callbacks.insert(id, callback);
    Some(SignalHandle { id })
}

/// Self-pipe signal registration for non-Linux platforms.
#[cfg(not(target_os = "linux"))]
pub fn signal_register(signum: c_int, callback: Box<dyn Fn()>) -> Option<SignalHandle> {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return None;
    }
    let base = unsafe { &mut *base_ptr };
    let id = base.alloc_id();

    // Create a non-blocking pipe.
    let (read_fd, write_fd) = pipe_nonblock().ok()?;

    // Register signal-hook to write a byte on signal delivery.
    let sig_id = signal_hook::low_level::pipe::register(signum, write_fd).ok()?;

    // Register the read end with calloop.
    let generic = Generic::new(read_fd, Interest::READ, Mode::Level);
    let token = base.handle.insert_source(generic, move |_, fd, data| {
        // Drain the pipe so it doesn't keep firing.
        let mut buf = [0u8; 64];
        loop {
            match rustix::io::read(&mut *fd, &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
        data.ready.push(ReadyEvent {
            id,
            fd: signum,
            events: super::super::EV_SIGNAL,
        });
        Ok(PostAction::Continue)
    }).ok()?;

    base.registrations.insert(id, Registration { token });
    base.timer_callbacks.insert(id, callback);
    Some(SignalHandle { id, sig_id })
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
    let token = base.handle.insert_source(timer, move |_, (), data| {
        data.ready.push(ReadyEvent {
            id,
            fd: -1,
            events: super::super::EV_TIMEOUT,
        });
        TimeoutAction::Drop
    }).ok()?;

    base.registrations.insert(id, Registration { token });
    base.timer_callbacks.insert(id, callback);
    Some(TimerHandle { id })
}

use calloop::generic::Generic;
#[cfg(target_os = "linux")]
use calloop::signals::{Signal, Signals};
use calloop::timer::{TimeoutAction, Timer};
use calloop::{EventLoop, Interest, LoopHandle, Mode, PostAction, RegistrationToken};

use super::event_base;
use super::super::{
    EV_READ, EV_SIGNAL, EV_TIMEOUT, EV_WRITE, EVLOOP_NONBLOCK,
    event_log_cb,
};
use tmux_log::log_debug;

/// A ready event notification — queued when a calloop source fires.
struct ReadyEvent {
    id: u64,
    fd: c_int,
    events: c_short,
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
    /// Non-epolable fds (regular files, etc.) that are always ready.
    /// Maps event id → (fd, events).  The actual callback is in
    /// `io_callbacks`; these fire on every `event_loop` iteration.
    fallback_io: HashMap<u64, (c_int, c_short)>,
    /// Closures for safe timer API (`timer_add`).  Keyed by event id.
    timer_callbacks: HashMap<u64, Box<dyn Fn()>>,
    /// Closures for safe I/O API (`io_register`).  Keyed by event id.
    /// Takes `(fd, fired_events)`.
    io_callbacks: HashMap<u64, Box<dyn Fn(c_int, c_short)>>,
    /// One-shot closures for `defer()`.  Option so dispatch can `.take()`
    /// and move out the `FnOnce`.  Keyed by event id.
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
            fallback_io: HashMap::new(),
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

#[cfg(target_os = "linux")]
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

/// Create a non-blocking pipe, returning `(read, write)` as `OwnedFd`s.
#[cfg(not(target_os = "linux"))]
fn pipe_nonblock() -> io::Result<(OwnedFd, OwnedFd)> {
    use std::os::fd::AsRawFd;
    let (read_fd, write_fd) = rustix::pipe::pipe()
        .map_err(|e| io::Error::from_raw_os_error(e.raw_os_error()))?;
    // Set non-blocking and close-on-exec on both ends.
    for fd in [read_fd.as_raw_fd(), write_fd.as_raw_fd()] {
        unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
    }
    Ok((read_fd, write_fd))
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
    // After fork: create a fresh event loop.  The old one's epoll fd is
    // an inherited copy and won't work correctly in the child.
    //
    // The old EventBase is intentionally leaked (not dropped) — dropping
    // it would double-free calloop's internal signal state.
    match EventBase::new() {
        Ok(base) => {
            let ptr = Box::into_raw(Box::new(base));
            unsafe { GLOBAL_BASE = ptr; }
            0
        }
        Err(_) => -1,
    }
}

thread_local! {
    /// Event IDs deleted during a dispatch cycle.  Checked before firing each
    /// callback so we skip events whose underlying resource was freed by an
    /// earlier callback in the same iteration (e.g. SIGCHLD destroying a pane
    /// whose read event is also ready).
    static CANCELLED: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
}

pub unsafe fn event_loop(flags: c_int) -> c_int {
    let base_ptr = unsafe { GLOBAL_BASE };
    if base_ptr.is_null() {
        return -1;
    }
    let base = unsafe { &mut *base_ptr };

    let mut data = LoopData { ready: Vec::new() };

    // Drain fallback I/O events (non-epolable fds like regular files).
    // These are always ready, so they fire on every iteration.
    for (&id, &(fd, events)) in &base.fallback_io {
        data.ready.push(ReadyEvent { id, fd, events });
    }

    let has_fallback = !base.fallback_io.is_empty();
    if (flags & EVLOOP_NONBLOCK) != 0 {
        let _ = base.event_loop.dispatch(Some(Duration::ZERO), &mut data);
    } else if has_fallback {
        // Fallback I/O events are always ready, so data.ready is never empty
        // when they exist.  We still need to poll epoll for real I/O and
        // signals, but can't block forever — use a short timeout.
        let _ = base.event_loop.dispatch(Some(Duration::from_millis(1)), &mut data);
    } else {
        // EVLOOP_ONCE: block until at least one event fires.
        let _ = base.event_loop.dispatch(None, &mut data);
    }

    // Clear the cancelled set before dispatching.
    CANCELLED.with(|cell| cell.borrow_mut().clear());

    // Dispatch all ready events, skipping any that were cancelled by an
    // earlier callback in this cycle (e.g. SIGCHLD handler destroying a
    // pane whose read event is also in the ready list).
    for ready in data.ready {
        let cancelled = CANCELLED.with(|cell| cell.borrow().contains(&ready.id));
        if cancelled {
            continue;
        }
        if let Some(cb) = base.io_callbacks.get(&ready.id) {
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
