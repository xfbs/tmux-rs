// Copyright (c) 2015 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::compat::{
    getpeereid,
    imsg::{
        imsg_clear, imsg_compose, imsg_flush, imsg_free, imsg_get, imsg_get_fd, imsg_init,
        imsg_read, imsgbuf,
    },
    imsg_buffer::msgbuf_write,
    setproctitle_,
};
use crate::libc::{
    AF_UNIX, EAGAIN, PF_UNSPEC, SA_RESTART, SIG_DFL, SIG_IGN, SIGCHLD, SIGCONT, SIGHUP, SIGINT,
    SIGPIPE, SIGQUIT, SIGTERM, SIGTSTP, SIGTTIN, SIGTTOU, SIGUSR1, SIGUSR2, SIGWINCH, close, gid_t,
    sigaction, sigemptyset, socketpair, uname, utsname,
};
use crate::*;

pub struct tmuxproc {
    pub name: *const u8,
    pub exit: i32,

    pub signalcb: Option<unsafe fn(i32)>,

    pub ev_sigint: Option<SignalHandle>,
    pub ev_sighup: Option<SignalHandle>,
    pub ev_sigchld: Option<SignalHandle>,
    pub ev_sigcont: Option<SignalHandle>,
    pub ev_sigterm: Option<SignalHandle>,
    pub ev_sigusr1: Option<SignalHandle>,
    pub ev_sigusr2: Option<SignalHandle>,
    pub ev_sigwinch: Option<SignalHandle>,

    pub peers: Vec<*mut tmuxpeer>,
}

pub const PEER_BAD: i32 = 0x1;

pub struct tmuxpeer {
    pub parent: *mut tmuxproc,

    pub ibuf: imsgbuf,
    pub event: Option<IoHandle>,
    pub uid: uid_t,

    pub flags: i32,

    pub dispatchcb: Option<unsafe fn(*mut imsg, *mut c_void)>,
    pub arg: *mut c_void,
}

/// Peer I/O callback — called when the peer fd is readable or writable.
pub unsafe fn proc_event_cb_fire(peer: *mut tmuxpeer, events: i16) {
    unsafe {
        let mut imsg: MaybeUninit<imsg> = MaybeUninit::<imsg>::uninit();
        let imsg = imsg.as_mut_ptr();

        if (*peer).flags & PEER_BAD == 0 && events & EV_READ != 0 {
            let mut n = imsg_read(&raw mut (*peer).ibuf);
            if (n == -1 && errno!() != EAGAIN) || n == 0 {
                ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                return;
            }
            loop {
                n = imsg_get(&raw mut (*peer).ibuf, imsg);
                if n == -1 {
                    ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                    return;
                }
                if n == 0 {
                    break;
                }
                let msgtype = msgtype::try_from((*imsg).hdr.type_);
                log_debug!("peer {:p} message {:?}", peer, msgtype);

                if peer_check_version(peer, imsg) != 0 {
                    let fd = imsg_get_fd(imsg);
                    if fd != -1 {
                        close(fd);
                    }
                    imsg_free(imsg);
                    break;
                }

                ((*peer).dispatchcb.unwrap())(imsg, (*peer).arg);
                imsg_free(imsg);
            }
        }

        if events & EV_WRITE != 0
            && msgbuf_write((&raw mut (*peer).ibuf.w).cast()) <= 0
            && errno!() != EAGAIN
        {
            ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
            return;
        }

        if ((*peer).flags & PEER_BAD != 0) && (*peer).ibuf.w.queued == 0 {
            ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
            return;
        }

        proc_update_event(peer);
    }
}


pub unsafe fn peer_check_version(peer: *mut tmuxpeer, imsg: *mut imsg) -> i32 {
    unsafe {
        let version = (*imsg).hdr.peerid & 0xff;
        if (*imsg).hdr.type_ != msgtype::MSG_VERSION as u32 && version != PROTOCOL_VERSION as u32 {
            log_debug!("peer {:p} bad version {}", peer, version);

            proc_send(peer, msgtype::MSG_VERSION, -1, null_mut(), 0);
            (*peer).flags |= PEER_BAD;

            return -1;
        }
        0
    }
}

pub unsafe fn proc_update_event(peer: *mut tmuxpeer) {
    unsafe {
        // Drop existing registration and re-register with current interest.
        (*peer).event = None;

        let mut events: i16 = EV_READ;
        if (*peer).ibuf.w.queued > 0 {
            events |= EV_WRITE;
        }
        (*peer).event = io_register(
            (*peer).ibuf.fd,
            events,
            Box::new(move |_fd, fired| unsafe { proc_event_cb_fire(peer, fired) }),
        );
    }
}

pub unsafe fn proc_send(
    peer: *mut tmuxpeer,
    type_: msgtype,
    fd: i32,
    buf: *const c_void,
    len: usize,
) -> i32 {
    unsafe {
        let ibuf = &raw mut (*peer).ibuf;
        let vp = buf;

        if (*peer).flags & PEER_BAD != 0 {
            return -1;
        }
        // log_debug_!("sending message {type_:?} to peer {peer:p} ({len} bytes)");

        let retval = imsg_compose(ibuf, type_ as u32, PROTOCOL_VERSION as u32, -1, fd, vp, len);
        if retval != 1 {
            return -1;
        }
        proc_update_event(peer);
        0
    }
}

pub fn proc_start(name: &CStr) -> *mut tmuxproc {
    unsafe {
        log_open(name);
        let name: *const u8 = name.as_ptr().cast();
        setproctitle_(c"%s (%s)".as_ptr().cast(), name, SOCKET_PATH);

        let mut u = MaybeUninit::<utsname>::uninit();
        if uname(u.as_mut_ptr()) < 0 {
            memset0(u.as_mut_ptr());
        }
        let u = u.as_mut_ptr();

        log_debug!(
            "{} started ({}): version {}, socket {}, protocol {}",
            _s(name),
            std::process::id(),
            getversion(),
            _s(SOCKET_PATH),
            PROTOCOL_VERSION,
        );
        log_debug!(
            "on {} {} {}",
            _s((*u).sysname.as_ptr()),
            _s((*u).release.as_ptr()),
            _s((*u).version.as_ptr()),
        );
        log_debug!(
            "using {} {}",
            _s(event_get_version()),
            _s(event_get_method())
        );
        #[cfg(feature = "utf8proc")]
        {
            log_debug!("using utf8proc {}", _s(utf8proc_version()));
        }

        let tp = xcalloc1::<tmuxproc>();
        tp.name = xstrdup(name).as_ptr();
        std::ptr::write(&raw mut tp.peers, Vec::new());
        std::ptr::write(&raw mut tp.ev_sigint, None);
        std::ptr::write(&raw mut tp.ev_sighup, None);
        std::ptr::write(&raw mut tp.ev_sigchld, None);
        std::ptr::write(&raw mut tp.ev_sigcont, None);
        std::ptr::write(&raw mut tp.ev_sigterm, None);
        std::ptr::write(&raw mut tp.ev_sigusr1, None);
        std::ptr::write(&raw mut tp.ev_sigusr2, None);
        std::ptr::write(&raw mut tp.ev_sigwinch, None);

        tp
    }
}

pub unsafe fn proc_loop(tp: *mut tmuxproc, loopcb: Option<unsafe fn() -> i32>) {
    unsafe {
        log_debug!("{} loop enter", _s((*tp).name));
        match loopcb {
            None => loop {
                event_loop(EVLOOP_ONCE);

                if (*tp).exit != 0 {
                    break;
                }
            },
            Some(loopcb) => loop {
                event_loop(EVLOOP_ONCE);

                if (*tp).exit != 0 {
                    break;
                }

                if loopcb() != 0 {
                    break;
                }
            },
        }
        log_debug!("{} loop exit", _s((*tp).name));
    }
}

pub unsafe fn proc_exit(tp: *mut tmuxproc) {
    unsafe {
        for &peer in &(*tp).peers {
            imsg_flush(&raw mut (*peer).ibuf);
        }
        (*tp).exit = 1;
    }
}

pub unsafe fn proc_set_signals(tp: *mut tmuxproc, signalcb: Option<unsafe fn(i32)>) {
    unsafe {
        let mut sa: sigaction = zeroed();

        (*tp).signalcb = signalcb;

        sigemptyset(&raw mut sa.sa_mask);
        sa.sa_flags = SA_RESTART;
        sa.sa_sigaction = SIG_IGN;

        sigaction(SIGPIPE, &sa, null_mut());
        sigaction(SIGTSTP, &sa, null_mut());
        sigaction(SIGTTIN, &sa, null_mut());
        sigaction(SIGTTOU, &sa, null_mut());
        sigaction(SIGQUIT, &sa, null_mut());

        let cb = signalcb.unwrap();
        (*tp).ev_sigint = signal_register(SIGINT, Box::new(move || cb(SIGINT)));
        (*tp).ev_sighup = signal_register(SIGHUP, Box::new(move || cb(SIGHUP)));
        (*tp).ev_sigchld = signal_register(SIGCHLD, Box::new(move || cb(SIGCHLD)));
        (*tp).ev_sigcont = signal_register(SIGCONT, Box::new(move || cb(SIGCONT)));
        (*tp).ev_sigterm = signal_register(SIGTERM, Box::new(move || cb(SIGTERM)));
        (*tp).ev_sigusr1 = signal_register(SIGUSR1, Box::new(move || cb(SIGUSR1)));
        (*tp).ev_sigusr2 = signal_register(SIGUSR2, Box::new(move || cb(SIGUSR2)));
        (*tp).ev_sigwinch = signal_register(SIGWINCH, Box::new(move || cb(SIGWINCH)));
    }
}

/// Clear signal event registrations and optionally reset signal dispositions.
///
/// When `defaults` is 0, this is called in the server process itself (e.g. after
/// the initial fork in `server_start`) and we must go through `event_del` to
/// properly unregister calloop sources from this process's event loop.
///
/// When `defaults` is non-zero, this is called in a **forked child** that is
/// about to `exec()` (spawn_pane, job_run, pipe-pane, etc.).  In that case we
/// must NOT call `event_del`, because calloop's unregister path issues
/// `epoll_ctl(DEL)` which operates on the **shared kernel epoll instance**,
/// removing the parent server's signal registrations.  The child's copies of
/// the signalfd/epoll file descriptors will be closed by `closefrom()` or
/// `exec()`'s CLOEXEC without affecting the parent.
pub unsafe fn proc_clear_signals(tp: *mut tmuxproc, defaults: i32) {
    unsafe {
        let mut sa: sigaction = zeroed();

        sigemptyset(&raw mut sa.sa_mask);
        sa.sa_flags = SA_RESTART;
        sa.sa_sigaction = SIG_DFL;

        sigaction(SIGPIPE, &raw mut sa, null_mut());
        sigaction(SIGTSTP, &raw mut sa, null_mut());

        if defaults == 0 {
            // Server process: properly unregister from our own event loop.
            (*tp).ev_sigint = None;
            (*tp).ev_sighup = None;
            (*tp).ev_sigchld = None;
            (*tp).ev_sigcont = None;
            (*tp).ev_sigterm = None;
            (*tp).ev_sigusr1 = None;
            (*tp).ev_sigusr2 = None;
            (*tp).ev_sigwinch = None;
        } else {
            // Forked child: reset signal dispositions to defaults.
            // Do NOT drop the SignalHandles — their Drop impl would call
            // epoll_ctl(DEL) on the shared kernel epoll, corrupting the
            // parent's event loop.  Forget them instead; the child will
            // exec() shortly and the leaked memory is harmless.
            std::mem::forget((*tp).ev_sigint.take());
            std::mem::forget((*tp).ev_sighup.take());
            std::mem::forget((*tp).ev_sigchld.take());
            std::mem::forget((*tp).ev_sigcont.take());
            std::mem::forget((*tp).ev_sigterm.take());
            std::mem::forget((*tp).ev_sigusr1.take());
            std::mem::forget((*tp).ev_sigusr2.take());
            std::mem::forget((*tp).ev_sigwinch.take());

            sigaction(SIGINT, &sa, null_mut());
            sigaction(SIGQUIT, &sa, null_mut());
            sigaction(SIGHUP, &sa, null_mut());
            sigaction(SIGCHLD, &sa, null_mut());
            sigaction(SIGCONT, &sa, null_mut());
            sigaction(SIGTERM, &sa, null_mut());
            sigaction(SIGUSR1, &sa, null_mut());
            sigaction(SIGUSR2, &sa, null_mut());
            sigaction(SIGWINCH, &sa, null_mut());
        }
    }
}

/// Unblock the signals that calloop's signalfd keeps masked.
///
/// The event backend blocks signals (INT, HUP, CHLD, CONT, TERM, USR1,
/// USR2, WINCH) so they are delivered to the signalfd rather than to
/// signal handlers.  Forked children inherit this mask, and the callers'
/// `sigprocmask(SIG_SETMASK, &oldset)` restores it — but `oldset` was
/// captured while the signals were blocked, so it keeps them blocked.
///
/// Call this in forked children **after** restoring `oldset` to ensure the
/// child process can receive signals normally before `exec()`.
pub unsafe fn proc_unblock_signals() {
    unsafe {
        let mut unblock: sigset_t = zeroed();
        sigemptyset(&raw mut unblock);
        sigaddset(&raw mut unblock, SIGINT);
        sigaddset(&raw mut unblock, SIGHUP);
        sigaddset(&raw mut unblock, SIGCHLD);
        sigaddset(&raw mut unblock, SIGCONT);
        sigaddset(&raw mut unblock, SIGTERM);
        sigaddset(&raw mut unblock, SIGUSR1);
        sigaddset(&raw mut unblock, SIGUSR2);
        sigaddset(&raw mut unblock, SIGWINCH);
        sigprocmask(SIG_UNBLOCK, &raw mut unblock, null_mut());
    }
}

pub unsafe fn proc_add_peer(
    tp: *mut tmuxproc,
    fd: i32,
    dispatchcb: Option<unsafe fn(*mut imsg, *mut c_void)>,
    arg: *mut c_void,
) -> *mut tmuxpeer {
    unsafe {
        let mut gid: gid_t = 0;
        let peer = xcalloc1::<tmuxpeer>() as *mut tmuxpeer;
        (*peer).parent = tp;

        (*peer).dispatchcb = dispatchcb;
        (*peer).arg = arg;

        imsg_init(&raw mut (*peer).ibuf, fd);
        std::ptr::write(&raw mut (*peer).event, None);

        if getpeereid(fd, &raw mut (*peer).uid, &raw mut gid) != 0 {
            (*peer).uid = -1i32 as uid_t;
        }

        log_debug!("add peer {:p}: {} ({:p})", peer, fd, arg);
        (*tp).peers.push(peer);

        proc_update_event(peer);
        peer
    }
}

pub unsafe fn proc_remove_peer(peer: *mut tmuxpeer) {
    unsafe {
        (*(*peer).parent).peers.retain(|&p| p != peer);
        log_debug!("remove peer {:p}", peer);

        std::ptr::drop_in_place(&raw mut (*peer).event);
        imsg_clear(&raw mut (*peer).ibuf);

        close((*peer).ibuf.fd);
        // Drop Vec fields before freeing the struct
        std::ptr::drop_in_place(&raw mut (*peer).ibuf.fds);
        std::ptr::drop_in_place(&raw mut (*peer).ibuf.w.bufs);
        free_(peer);
    }
}

pub unsafe fn proc_kill_peer(peer: *mut tmuxpeer) {
    unsafe {
        (*peer).flags |= PEER_BAD;
    }
}

pub unsafe fn proc_flush_peer(peer: *mut tmuxpeer) {
    unsafe {
        imsg_flush(&raw mut (*peer).ibuf);
    }
}

pub unsafe fn proc_toggle_log(tp: *mut tmuxproc) {
    unsafe {
        log_toggle(CStr::from_ptr((*tp).name.cast()));
    }
}

#[cfg_attr(target_os = "macos", expect(deprecated))]
/// On success, the PID of the child process is returned in the parent, and 0 is returned in the child.
pub unsafe fn proc_fork_and_daemon(fd: *mut i32) -> pid_t {
    unsafe {
        let mut pair: [c_int; 2] = [0; 2];

        if socketpair(
            AF_UNIX,
            libc::SOCK_STREAM,
            PF_UNSPEC,
            &raw mut pair as *mut i32,
        ) != 0
        {
            fatal("socketpair failed");
        }

        match libc::fork() {
            -1 => fatal("fork failed"),
            0 => {
                close(pair[0]);
                *fd = pair[1];
                if libc::daemon(1, 0) != 0 {
                    fatal("daemon failed");
                }
                0
            }
            pid => {
                close(pair[1]);
                *fd = pair[0];
                pid
            }
        }
    }
}

pub unsafe fn proc_get_peer_uid(peer: *const tmuxpeer) -> uid_t {
    unsafe { (*peer).uid }
}
