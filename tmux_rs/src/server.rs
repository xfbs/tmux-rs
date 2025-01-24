use compat_rs::{
    queue::{tailq_empty, tailq_foreach, tailq_foreach_safe, tailq_init},
    tailq_remove,
    tree::{rb_empty, rb_foreach, rb_foreach_safe, rb_init},
};
use libc::{
    __errno_location, AF_UNIX, ECHILD, S_IRGRP, S_IROTH, S_IRUSR, S_IRWXG, S_IRWXO, S_IXGRP, S_IXOTH, S_IXUSR,
    SIG_BLOCK, SIG_SETMASK, SIGCONT, SIGTTIN, SIGTTOU, SOCK_STREAM, WIFEXITED, WIFSIGNALED, WIFSTOPPED, WNOHANG,
    WSTOPSIG, WUNTRACED, bind, chmod, close, fprintf, free, gettimeofday, listen, malloc_trim, sigfillset, sigprocmask,
    sigset_t, socket, strsignal, umask, unlink, waitpid,
};
use libevent_sys::{EV_READ, EV_TIMEOUT, event_add, event_del, event_initialized, event_reinit, event_set};

use super::*;

#[unsafe(no_mangle)]
pub static mut clients: clients = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub static mut server_proc: *mut tmuxproc = null_mut();
pub static mut server_fd: c_int = -1;
pub static mut server_client_flags: u64 = 0;
pub static mut server_exit: c_int = 0;
pub static mut server_ev_accept: event = unsafe { zeroed() };
pub static mut server_ev_tidy: event = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub static mut marked_pane: cmd_find_state = unsafe { zeroed() };

pub static mut message_next: c_uint = 0;

#[unsafe(no_mangle)]
pub static mut message_log: message_list = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub static mut current_time: time_t = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_set_marked(s: *mut session, wl: *mut winlink, wp: *mut window_pane) {
    unsafe {
        cmd_find_clear_state(&raw mut marked_pane, 0);
        marked_pane.s = s;
        marked_pane.wl = wl;
        marked_pane.w = (*wl).window;
        marked_pane.wp = wp;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_clear_marked() {
    unsafe {
        cmd_find_clear_state(&raw mut marked_pane, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_is_marked(s: *mut session, wl: *mut winlink, wp: *mut window_pane) -> c_int {
    if s.is_null() || wl.is_null() || wp.is_null() {
        return 0;
    }

    unsafe {
        if marked_pane.s != s || marked_pane.wl != wl {
            return 0;
        }
        if marked_pane.wp != wp {
            return 0;
        }
        server_check_marked()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_check_marked() -> c_int {
    unsafe { cmd_find_valid_state(&raw mut marked_pane) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_create_socket(flags: u64, cause: *mut *mut c_char) -> c_int {
    unsafe {
        #[allow(clippy::never_loop)]
        'fail: loop {
            let mut sa: libc::sockaddr_un = zeroed();
            sa.sun_family = libc::AF_UNIX as _;
            let size = compat_rs::strlcpy(sa.sun_path.as_mut_ptr(), socket_path, size_of_val(&sa.sun_path));
            if size >= size_of_val(&sa.sun_path) {
                *__errno_location() = libc::ENAMETOOLONG;
                // goto fail;
                break 'fail;
            }
            unlink(sa.sun_path.as_ptr());

            let fd = socket(AF_UNIX, SOCK_STREAM, 0);
            if fd == -1 {
                // goto fail;
                break 'fail;
            }

            let mask = if flags & CLIENT_DEFAULTSOCKET != 0 {
                umask(S_IXUSR | S_IXGRP | S_IRWXO)
            } else {
                umask(S_IXUSR | S_IRWXG | S_IRWXO)
            };

            let saved_errno: c_int;
            if bind(fd, &raw const sa as _, size_of::<libc::sockaddr_un>() as _) == -1 {
                saved_errno = *__errno_location();
                close(fd);
                *__errno_location() = saved_errno;
                break 'fail;
            }
            umask(mask);

            if listen(fd, 128) == -1 {
                saved_errno = *__errno_location();
                close(fd);
                *__errno_location() = saved_errno;
                break 'fail;
            }
            setblocking(fd, 0);

            return fd;
        }

        // fail:
        if !cause.is_null() {
            xmalloc::xasprintf(
                cause,
                c"error creating %s (%s)".as_ptr(),
                socket_path,
                libc::strerror(*__errno_location()),
            );
        }
        -1
    }
}

/// Tidy up every hour.
unsafe extern "C" fn server_tidy_event(_fd: i32, _events: i16, _data: *mut c_void) {
    let tv = libevent_sys::timeval {
        tv_sec: 3600,
        tv_usec: 0,
    };
    unsafe {
        let t = get_timer();

        format_tidy_jobs();

        malloc_trim(0);

        log_debug(
            c"%s: took %llu milliseconds".as_ptr(),
            c"server_tidy_event".as_ptr(),
            get_timer() - t,
        );
        event_add(&raw mut server_ev_tidy, &raw const tv);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_start(
    client: *mut tmuxproc,
    flags: u64,
    base: *mut event_base,
    lockfd: c_int,
    lockfile: *mut c_char,
) -> c_int {
    unsafe {
        let mut fd = 0;
        let mut set: sigset_t = zeroed();
        let mut oldset: sigset_t = zeroed();

        let mut c: *mut client = null_mut();
        let mut cause: *mut c_char = null_mut();
        let tv: timeval = timeval {
            tv_sec: 3600,
            tv_usec: 0,
        };

        sigfillset(&raw mut set);
        sigprocmask(SIG_BLOCK, &set, &raw mut oldset);

        if !flags & CLIENT_NOFORK != 0 {
            if proc_fork_and_daemon(&raw mut fd) != 0 {
                sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
                return fd;
            }
        }
        proc_clear_signals(client, 0);
        server_client_flags = flags;

        if event_reinit(base) != 0 {
            fatalx(c"event_reinit failed".as_ptr());
        }
        server_proc = proc_start(c"server".as_ptr());

        proc_set_signals(server_proc, Some(server_signal));
        sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());

        if log_get_level() > 1 {
            tty_create_log();
        }

        // TODO pledge

        input_key_build();
        rb_init(&raw mut windows);
        rb_init(&raw mut all_window_panes);
        tailq_init(&raw mut clients);
        rb_init(&raw mut sessions);
        key_bindings_init();
        tailq_init(&raw mut message_log);
        gettimeofday(&raw mut start_time, null_mut());

        if cfg!(feature = "systemd") {
            server_fd = compat_rs::systemd::systemd_create_socket(flags as i32, &raw mut cause);
        } else {
            server_fd = server_create_socket(flags, &raw mut cause);
        }
        if server_fd != -1 {
            server_update_socket();
        }
        if !flags & CLIENT_NOFORK != 0 {
            c = server_client_create(fd);
        } else {
            options_set_number(global_options, c"exit-empty".as_ptr(), 0);
        }

        if lockfd >= 0 {
            unlink(lockfile);
            free(lockfile as _);
            close(lockfd);
        }

        if !cause.is_null() {
            if !c.is_null() {
                (*c).exit_message = cause;
                (*c).flags |= CLIENT_EXIT;
            } else {
                fprintf(stderr, c"%s\n".as_ptr(), cause);
                std::process::exit(1);
            }
        }

        evtimer_set(&raw mut server_ev_tidy, Some(server_tidy_event), null_mut());
        evtimer_add(&raw mut server_ev_tidy, &raw const tv);

        server_acl_init();

        server_add_accept(0);
        proc_loop(server_proc, Some(server_loop));

        job_kill_all();
        status_prompt_save_history();

        std::process::exit(0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_loop() -> i32 {
    unsafe {
        current_time = libc::time(null_mut());

        loop {
            let mut items = cmdq_next(null_mut());
            tailq_foreach(&raw mut clients, |c| {
                if (*c).flags & CLIENT_IDENTIFIED != 0 {
                    items += cmdq_next(c);
                }
                ControlFlow::Continue::<(), ()>(())
            });

            if items == 0 {
                break;
            }
        }

        server_client_loop();

        if options_get_number(global_options, c"exit-empty".as_ptr()) == 0 && server_exit == 0 {
            return 0;
        }

        if options_get_number(global_options, c"exit-unattached".as_ptr()) == 0 {
            if !rb_empty(&raw mut sessions) {
                return 0;
            }
        }

        if tailq_foreach(&raw mut clients, |c| {
            if !(*c).session.is_null() {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        })
        .is_break()
        {
            return 0;
        }

        /*
         * No attached clients therefore want to exit - flush any waiting
         * clients but don't actually exit until they've gone.
         */
        cmd_wait_for_flush();
        if !tailq_empty(&raw mut clients) {
            return 0;
        }

        if job_still_running() != 0 {
            return 0;
        }

        1
    }
}

unsafe extern "C" fn server_send_exit() {
    unsafe {
        cmd_wait_for_flush();

        tailq_foreach_safe(&raw mut clients, |c| {
            if (*c).flags & CLIENT_SUSPENDED != 0 {
                server_client_lost(c);
            } else {
                (*c).flags |= CLIENT_EXIT;
                (*c).exit_type = exit_type::CLIENT_EXIT_SHUTDOWN;
            }
            (*c).session = null_mut();
            ControlFlow::Continue::<(), ()>(())
        });

        compat_rs::tree::rb_foreach_safe(&raw mut sessions, |s| {
            session_destroy(s, 1, c"server_send_exit".as_ptr());
            ControlFlow::Continue::<(), ()>(())
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_update_socket() {
    static mut last: c_int = -1;
    unsafe {
        let mut sb: libc::stat = zeroed(); // TODO remove unecessary init
        let mut n = 0;

        rb_foreach(&raw mut sessions, |s| {
            if (*s).attached != 0 {
                n += 1;
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });

        if n != last {
            last = n;

            if libc::stat(socket_path, &raw mut sb) != 0 {
                return;
            }
            let mut mode = sb.st_mode & compat_rs::ACCESSPERMS;
            if n != 0 {
                if mode & S_IRUSR != 0 {
                    mode |= S_IXUSR;
                }
                if mode & S_IRGRP != 0 {
                    mode |= S_IXGRP;
                }
                if mode & S_IROTH != 0 {
                    mode |= S_IXOTH;
                }
            } else {
                mode &= !(S_IXUSR | S_IXGRP | S_IXOTH);
            }
            chmod(socket_path, mode);
        }
    }
}

unsafe extern "C" fn server_accept(fd: i32, events: i16, _data: *mut c_void) {
    unsafe {
        let mut sa: libc::sockaddr_storage = zeroed(); // TODO remove this init
        let mut slen: libc::socklen_t = size_of::<libc::sockaddr_storage>() as libc::socklen_t;

        server_add_accept(0);
        if events & EV_READ as i16 == 0 {
            return;
        }

        let newfd = libc::accept(fd, &raw mut sa as _, &raw mut slen);
        if newfd == -1 {
            match *__errno_location() {
                libc::EAGAIN | libc::EINTR | libc::ECONNABORTED => {
                    return;
                }
                libc::ENFILE | libc::EMFILE => {
                    /* Delete and don't try again for 1 second. */
                    server_add_accept(1);
                    return;
                }
                _ => {
                    fatal(c"accept failed".as_ptr());
                }
            }
        }

        if server_exit != 0 {
            close(newfd);
            return;
        }
        let c = server_client_create(newfd);
        if server_acl_join(c) == 0 {
            (*c).exit_message = xmalloc::xstrdup(c"access not allowed".as_ptr()).cast().as_ptr();
            (*c).flags |= CLIENT_EXIT;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_accept(timeout: c_int) {
    unsafe {
        let mut tv = libevent_sys::timeval {
            tv_sec: timeout as i64,
            tv_usec: 0,
        };

        if server_fd == -1 {
            return;
        }

        if event_initialized(&raw mut server_ev_accept) != 0 {
            event_del(&raw mut server_ev_accept);
        }

        if timeout == 0 {
            event_set(
                &raw mut server_ev_accept,
                server_fd,
                EV_READ as i16,
                Some(server_accept),
                null_mut(),
            );
            event_add(&raw mut server_ev_accept, null_mut());
        } else {
            event_set(
                &raw mut server_ev_accept,
                server_fd,
                EV_TIMEOUT as i16,
                Some(server_accept),
                null_mut(),
            );
            event_add(&raw mut server_ev_accept, &raw mut tv);
        }
    }
}

// Signal handler.
unsafe extern "C" fn server_signal(sig: i32) {
    unsafe {
        log_debug(c"%s: %s".as_ptr(), c"server_signal".as_ptr(), strsignal(sig));
        match sig {
            libc::SIGINT | libc::SIGTERM => {
                server_exit = 1;
                server_send_exit();
            }
            libc::SIGCHLD => {
                server_child_signal();
            }
            libc::SIGUSR1 => {
                libevent_sys::event_del(&raw mut server_ev_accept);
                let fd = server_create_socket(server_client_flags, null_mut());
                if fd != -1 {
                    close(server_fd);
                    server_fd = fd;
                    server_update_socket();
                }
                server_add_accept(0);
            }
            libc::SIGUSR2 => {
                proc_toggle_log(server_proc);
            }
            _ => {
                // nop
            }
        }
    }
}

// handle SIGCHLD
unsafe extern "C" fn server_child_signal() {
    let mut status = 0i32;
    unsafe {
        loop {
            let pid: pid_t = waitpid(compat_rs::WAIT_ANY, &raw mut status, WNOHANG | WUNTRACED);
            match pid {
                -1 => {
                    if *__errno_location() == ECHILD {
                        return;
                    }
                    fatal(c"waitpid failed".as_ptr());
                }
                0 => return,
                _ => {
                    if WIFSTOPPED(status) {
                        server_child_stopped(pid, status);
                    } else if WIFEXITED(status) || WIFSIGNALED(status) {
                        server_child_exited(pid, status);
                    }
                }
            }
        }
    }
}

unsafe extern "C" fn server_child_exited(pid: pid_t, status: i32) {
    unsafe {
        rb_foreach_safe(&raw mut windows, |w| {
            tailq_foreach(&raw mut (*w).panes, |wp| {
                if (*wp).pid == pid {
                    (*wp).status = status;
                    (*wp).flags |= PANE_STATUSREADY;

                    log_debug(c"%%%u exited".as_ptr(), (*wp).id);
                    (*wp).flags |= PANE_EXITED;

                    if window_pane_destroy_ready(wp) != 0 {
                        server_destroy_pane(wp, 1);
                    }
                    return ControlFlow::Break(());
                }
                ControlFlow::Continue(())
            });
            ControlFlow::Continue::<(), ()>(())
        });
        job_check_died(pid, status);
    }
}
unsafe extern "C" fn server_child_stopped(pid: pid_t, status: i32) {
    unsafe {
        if WSTOPSIG(status) == SIGTTIN || WSTOPSIG(status) == SIGTTOU {
            return;
        }

        rb_foreach(&raw mut windows, |w| {
            tailq_foreach(&raw mut (*w).panes, |wp| {
                if (*wp).pid == pid {
                    if libc::killpg(pid, SIGCONT) != 0 {
                        libc::kill(pid, SIGCONT);
                    }
                }
                ControlFlow::Continue::<(), ()>(())
            });
            ControlFlow::Continue::<(), ()>(())
        });
        job_check_died(pid, status);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_message(fmt: *const c_char, mut args: ...) {
    unsafe {
        let mut s: *mut c_char = null_mut();

        let mut ap: VaListImpl = args.clone();
        xmalloc::xvasprintf(&raw mut s, fmt, ap.as_va_list());

        log_debug(c"message: %s".as_ptr(), s);

        let msg: *mut message_entry = xmalloc::xcalloc(1, size_of::<message_entry>()).cast().as_ptr();
        gettimeofday(&raw mut (*msg).msg_time, null_mut());
        (*msg).msg_num = message_next + 1;
        message_next += 1;
        (*msg).msg = s;

        compat_rs::queue::tailq_insert_tail!(&raw mut message_log, msg, entry);

        let limit = options_get_number(global_options, c"message-limit".as_ptr()) as u32;
        tailq_foreach_safe(&raw mut message_log, |msg| {
            if (*msg).msg_num + limit >= message_next {
                return ControlFlow::Continue::<(), ()>(());
            }
            free((*msg).msg as _);
            tailq_remove!(&raw mut message_log, msg, entry);
            free(msg as _);
            ControlFlow::Continue::<(), ()>(())
        });
    }
}
