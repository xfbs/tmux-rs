use compat_rs::{
    getpeereid,
    imsg::{imsg_clear, imsg_compose, imsg_flush, imsg_free, imsg_get, imsg_get_fd, imsg_init, imsg_read, imsgbuf},
    imsg_buffer::msgbuf_write,
    queue::{tailq_foreach_, tailq_init, tailq_insert_tail, tailq_remove},
    setproctitle,
};
use libc::{
    __errno_location, AF_UNIX, EAGAIN, PF_UNSPEC, SA_RESTART, SIG_DFL, SIG_IGN, SIGCHLD, SIGCONT, SIGHUP, SIGINT,
    SIGPIPE, SIGQUIT, SIGTERM, SIGTSTP, SIGTTIN, SIGTTOU, SIGUSR1, SIGUSR2, SIGWINCH, close, daemon, getpid, gid_t,
    sigaction, sigemptyset, socketpair, uname, utsname,
};

use crate::event_::{signal_add, signal_set};
use crate::{xmalloc::Zeroable, *};

unsafe extern "C" {
    // pub unsafe fn proc_send(_: *mut tmuxpeer, _: msgtype, _: c_int, _: *const c_void, _: usize) -> c_int;
    // pub unsafe fn proc_start(_: *const c_char) -> *mut tmuxproc;
    // pub unsafe fn proc_loop(_: *mut tmuxproc, _: Option<unsafe extern "C" fn() -> c_int>);
    // pub unsafe fn proc_exit(_: *mut tmuxproc);
    // pub unsafe fn proc_set_signals(_: *mut tmuxproc, _: Option<unsafe extern "C" fn(_: c_int)>);
    // pub unsafe fn proc_clear_signals(_: *mut tmuxproc, _: c_int);
    // pub unsafe fn proc_add_peer( _: *mut tmuxproc, _: c_int, _: Option<unsafe extern "C" fn(_: *mut imsg, _: *mut c_void)>, _: *mut c_void,) -> *mut tmuxpeer;
    // pub unsafe fn proc_remove_peer(_: *mut tmuxpeer);
    // pub unsafe fn proc_kill_peer(_: *mut tmuxpeer);
    // pub unsafe fn proc_flush_peer(_: *mut tmuxpeer);
    // pub unsafe fn proc_toggle_log(_: *mut tmuxproc);
    // pub unsafe fn proc_fork_and_daemon(_: *mut c_int) -> pid_t;
    // pub unsafe fn proc_get_peer_uid(_: *mut tmuxpeer) -> uid_t;
}

unsafe impl Zeroable for tmuxproc {}
#[repr(C)]
pub struct tmuxproc {
    pub name: *const c_char,
    pub exit: i32,

    pub signalcb: Option<unsafe extern "C" fn(i32)>,

    pub ev_sigint: event,
    pub ev_sighup: event,
    pub ev_sigchld: event,
    pub ev_sigcont: event,
    pub ev_sigterm: event,
    pub ev_sigusr1: event,
    pub ev_sigusr2: event,
    pub ev_sigwinch: event,

    pub peers: tailq_head<tmuxpeer>,
}

pub const PEER_BAD: i32 = 0x1;

unsafe impl Zeroable for tmuxpeer {}
compat_rs::impl_tailq_entry!(tmuxpeer, entry, tailq_entry<tmuxpeer>);
// #[derive(compat_rs::TailQEntry)]
#[repr(C)]
pub struct tmuxpeer {
    pub parent: *mut tmuxproc,

    pub ibuf: imsgbuf,
    pub event: event,
    pub uid: uid_t,

    pub flags: i32,

    pub dispatchcb: Option<unsafe extern "C" fn(*mut imsg, *mut c_void)>,
    pub arg: *mut c_void,

    // #[entry]
    pub entry: tailq_entry<tmuxpeer>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_event_cb(_fd: i32, events: i16, arg: *mut c_void) {
    unsafe {
        let mut peer = arg as *mut tmuxpeer;
        let mut n = 0isize;
        let mut imsg: MaybeUninit<imsg> = MaybeUninit::<imsg>::uninit();
        let imsg = imsg.as_mut_ptr();

        if ((*peer).flags & PEER_BAD == 0 && events & EV_READ != 0) {
            n = imsg_read(&raw mut (*peer).ibuf);
            if ((n == -1 && errno!() != EAGAIN) || n == 0) {
                ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                return;
            }
            loop {
                n = imsg_get(&raw mut (*peer).ibuf, imsg);
                if (n == -1) {
                    ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                    return;
                }
                if (n == 0) {
                    break;
                }
                log_debug(c"peer %p message %d".as_ptr(), peer, (*imsg).hdr.type_);

                if (peer_check_version(peer, imsg) != 0) {
                    let fd = imsg_get_fd(imsg);
                    if (fd != -1) {
                        close(fd);
                    }
                    imsg_free(imsg);
                    break;
                }

                ((*peer).dispatchcb.unwrap())(imsg, (*peer).arg);
                imsg_free(imsg);
            }
        }

        if (events & EV_WRITE as i16 != 0) {
            if msgbuf_write(&raw mut (*peer).ibuf.w) <= 0 && errno!() != EAGAIN {
                ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
                return;
            }
        }

        if (((*peer).flags & PEER_BAD != 0) && (*peer).ibuf.w.queued == 0) {
            ((*peer).dispatchcb.unwrap())(null_mut(), (*peer).arg);
            return;
        }

        proc_update_event(peer);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_signal_cb(signo: i32, events: i16, arg: *mut c_void) {
    unsafe {
        let mut tp = arg as *mut tmuxproc;

        ((*tp).signalcb.unwrap())(signo);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn peer_check_version(peer: *mut tmuxpeer, imsg: *mut imsg) -> i32 {
    unsafe {
        let version = (*imsg).hdr.peerid & 0xff;
        if ((*imsg).hdr.type_ != msgtype::MSG_VERSION as u32 && version != PROTOCOL_VERSION as u32) {
            log_debug(c"peer %p bad version %d".as_ptr(), peer, version);

            proc_send(peer, msgtype::MSG_VERSION, -1, null_mut(), 0);
            (*peer).flags |= PEER_BAD;

            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_update_event(peer: *mut tmuxpeer) {
    unsafe {
        event_del(&raw mut (*peer).event);

        let mut events: i16 = EV_READ as i16;
        if ((*peer).ibuf.w.queued > 0) {
            events |= EV_WRITE as i16;
        }
        event_set(
            &raw mut (*peer).event,
            (*peer).ibuf.fd,
            events,
            Some(proc_event_cb),
            peer.cast(),
        );

        event_add(&raw mut (*peer).event, null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_send(
    peer: *mut tmuxpeer,
    type_: msgtype,
    fd: i32,
    buf: *const c_void,
    len: usize,
) -> i32 {
    unsafe {
        let ibuf = &raw mut (*peer).ibuf;
        let vp = buf;

        if ((*peer).flags & PEER_BAD != 0) {
            return -1;
        }
        log_debug(c"sending message %d to peer %p (%zu bytes)".as_ptr(), type_, peer, len);

        let retval = imsg_compose(ibuf, type_ as u32, PROTOCOL_VERSION as u32, -1, fd, vp, len);
        if (retval != 1) {
            return -1;
        }
        proc_update_event(peer);
        0
    }
}

pub unsafe fn proc_start(name: &CStr) -> *mut tmuxproc {
    unsafe {
        let name = name.as_ptr();
        log_open(name);
        setproctitle(c"%s (%s)".as_ptr(), name, socket_path);

        let mut u = MaybeUninit::<utsname>::uninit();
        if uname(u.as_mut_ptr()) < 0 {
            memset0(u.as_mut_ptr());
        }
        let u = u.as_mut_ptr();

        log_debug(
            c"%s started (%ld): version %s, socket %s, protocol %d".as_ptr(),
            name,
            getpid() as c_long,
            getversion(),
            socket_path,
            PROTOCOL_VERSION,
        );
        log_debug(
            c"on %s %s %s".as_ptr(),
            (*u).sysname.as_ptr(),
            (*u).release.as_ptr(),
            (*u).version.as_ptr(),
        );
        log_debug(
            c"using libevent %s %s".as_ptr(),
            event_get_version(),
            event_get_method(),
        );
        #[cfg(feature = "utf8proc")]
        {
            log_debug(c"using utf8proc %s".as_ptr(), utf8proc_version());
        }
        #[cfg(feature = "ncurses")]
        {
            log_debug(
                c"using ncurses %s %06u".as_ptr(),
                NCURSES_VERSION,
                NCURSES_VERSION_PATCH,
            );
        }

        let tp = xcalloc1::<tmuxproc>();
        tp.name = xstrdup(name).as_ptr();
        tailq_init(&raw mut tp.peers);

        tp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_loop(tp: *mut tmuxproc, loopcb: Option<unsafe extern "C" fn() -> i32>) {
    unsafe {
        log_debug(c"%s loop enter".as_ptr(), (*tp).name);
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
        log_debug(c"%s loop exit".as_ptr(), (*tp).name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_exit(tp: *mut tmuxproc) {
    unsafe {
        for peer in tailq_foreach_(&raw mut (*tp).peers).map(NonNull::as_ptr) {
            imsg_flush(&raw mut (*peer).ibuf);
        }
        (*tp).exit = 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_set_signals(tp: *mut tmuxproc, signalcb: Option<unsafe extern "C" fn(i32)>) {
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

        signal_set(&raw mut (*tp).ev_sigint, SIGINT, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigint, null_mut());
        signal_set(&raw mut (*tp).ev_sighup, SIGHUP, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sighup, null_mut());
        signal_set(&raw mut (*tp).ev_sigchld, SIGCHLD, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigchld, null_mut());
        signal_set(&raw mut (*tp).ev_sigcont, SIGCONT, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigcont, null_mut());
        signal_set(&raw mut (*tp).ev_sigterm, SIGTERM, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigterm, null_mut());
        signal_set(&raw mut (*tp).ev_sigusr1, SIGUSR1, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigusr1, null_mut());
        signal_set(&raw mut (*tp).ev_sigusr2, SIGUSR2, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigusr2, null_mut());
        signal_set(&raw mut (*tp).ev_sigwinch, SIGWINCH, Some(proc_signal_cb), tp.cast());
        signal_add(&raw mut (*tp).ev_sigwinch, null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_clear_signals(tp: *mut tmuxproc, defaults: i32) {
    unsafe {
        let mut sa: sigaction = zeroed();

        sigemptyset(&raw mut sa.sa_mask);
        sa.sa_flags = SA_RESTART;
        sa.sa_sigaction = SIG_DFL;

        sigaction(SIGPIPE, &raw mut sa, null_mut());
        sigaction(SIGTSTP, &raw mut sa, null_mut());

        event_del(&raw mut (*tp).ev_sigint);
        event_del(&raw mut (*tp).ev_sighup);
        event_del(&raw mut (*tp).ev_sigchld);
        event_del(&raw mut (*tp).ev_sigcont);
        event_del(&raw mut (*tp).ev_sigterm);
        event_del(&raw mut (*tp).ev_sigusr1);
        event_del(&raw mut (*tp).ev_sigusr2);
        event_del(&raw mut (*tp).ev_sigwinch);

        if defaults != 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_add_peer(
    tp: *mut tmuxproc,
    fd: i32,
    dispatchcb: Option<unsafe extern "C" fn(*mut imsg, *mut c_void)>,
    arg: *mut c_void,
) -> *mut tmuxpeer {
    unsafe {
        let mut gid: gid_t = 0;
        let mut peer = xcalloc1::<tmuxpeer>();
        peer.parent = tp;

        peer.dispatchcb = dispatchcb;
        peer.arg = arg;

        let peer_uid = &raw mut peer.uid;
        let peer = peer as *mut tmuxpeer;

        imsg_init(&raw mut (*peer).ibuf, fd);
        event_set(
            &raw mut (*peer).event,
            fd,
            EV_READ,
            Some(proc_event_cb),
            peer.cast(), // TODO could be ub if this and function below both write
        );

        if getpeereid(fd, peer_uid, &raw mut gid) != 0 {
            (*peer).uid = -1i32 as uid_t;
        }

        log_debug(c"add peer %p: %d (%p)".as_ptr(), peer, fd, arg);
        tailq_insert_tail(&raw mut (*tp).peers, peer);

        proc_update_event(peer);
        peer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_remove_peer(peer: *mut tmuxpeer) {
    unsafe {
        tailq_remove(&raw mut (*(*peer).parent).peers, peer);
        log_debug(c"remove peer %p".as_ptr(), peer);

        event_del(&raw mut (*peer).event);
        imsg_clear(&raw mut (*peer).ibuf);

        close((*peer).ibuf.fd);
        free_(peer);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_kill_peer(peer: *mut tmuxpeer) {
    unsafe {
        (*peer).flags |= PEER_BAD;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_flush_peer(peer: *mut tmuxpeer) {
    unsafe {
        imsg_flush(&raw mut (*peer).ibuf);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_toggle_log(tp: *mut tmuxproc) {
    unsafe {
        log_toggle((*tp).name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_fork_and_daemon(fd: *mut i32) -> pid_t {
    unsafe {
        let mut pair: [c_int; 2] = [0; 2];

        if socketpair(AF_UNIX, libc::SOCK_STREAM, PF_UNSPEC, &raw mut pair as *mut i32) != 0 {
            fatal(c"socketpair failed".as_ptr());
        }

        match libc::fork() {
            -1 => {
                fatal(c"fork failed".as_ptr());
            }
            0 => {
                close(pair[0]);
                *fd = pair[1];
                if (daemon(1, 0) != 0) {
                    fatal(c"daemon failed".as_ptr());
                }
                return 0;
            }
            pid => {
                close(pair[1]);
                *fd = pair[0];
                return pid;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn proc_get_peer_uid(peer: *const tmuxpeer) -> uid_t { unsafe { (*peer).uid } }
