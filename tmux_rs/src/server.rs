use compat_rs::{queue::tailq_init, tree::rb_init};
use libc::{
    __errno_location, AF_UNIX, S_IRWXG, S_IRWXO, S_IXGRP, S_IXUSR, SIG_BLOCK, SIG_SETMASK, SOCK_STREAM, bind, close,
    fprintf, free, gettimeofday, listen, malloc_trim, sigfillset, sigprocmask, sigset_t, socket, strsignal, umask,
    unlink,
};
use libevent_sys::{event_add, event_reinit};

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
            }
            unlink(sa.sun_path.as_ptr());

            let fd = socket(AF_UNIX, SOCK_STREAM, 0);
            if fd == -1 {
                // goto fail;
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

        #[expect(clippy::collapsible_if)]
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

unsafe extern "C" fn server_loop() -> i32 {
    // TODO
    0
}

unsafe extern "C" fn server_send_exit() {
    // TODO
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_update_socket() {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_accept(_timeout: c_int) {
    todo!()
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

pub unsafe extern "C" fn server_child_signal() {
    //
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_message(_fmt: *const c_char, _args: ...) {
    todo!()
}
