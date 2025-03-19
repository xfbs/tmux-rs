use crate::*;

use compat_rs::closefrom;
use libc::{
    _exit, AF_UNIX, O_WRONLY, PF_UNSPEC, SIG_BLOCK, SIG_SETMASK, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO, close,
    dup2, execl, open, sigfillset, sigprocmask, sigset_t, socketpair,
};

#[unsafe(no_mangle)]
static mut cmd_pipe_pane_entry: cmd_entry = cmd_entry {
    name: c"pipe-pane".as_ptr(),
    alias: c"pipep".as_ptr(),

    args: args_parse::new(c"IOot:", 0, 1, None),
    usage: c"[-IOo] [-t target-pane] [shell-command]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_pipe_pane_exec),
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_pipe_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut tc = cmdq_get_target_client(item);
        let mut wp = (*target).wp;
        let mut s = (*target).s;
        let mut wl = (*target).wl;
        let mut wpo = &raw mut (*wp).pipe_offset;
        // char *cmd;
        // int old_fd, pipe_fd[2], null_fd, in, out;
        let mut old_fd = 0;
        let mut pipe_fd: [i32; 2] = [0; 2];
        let mut in_: i32 = 0;
        let mut out: i32 = 0;
        // struct format_tree *ft;
        // sigset_t set, oldset;
        let mut set: sigset_t = zeroed(); // TODO uninit
        let mut oldset: sigset_t = zeroed(); // TODO uninit

        /* Do nothing if pane is dead. */
        if (window_pane_exited(wp) != 0) {
            cmdq_error(item, c"target pane has exited".as_ptr());
            return cmd_retval::CMD_RETURN_ERROR;
        }

        /* Destroy the old pipe. */
        let old_fd = (*wp).pipe_fd;
        if ((*wp).pipe_fd != -1) {
            bufferevent_free((*wp).pipe_event);
            close((*wp).pipe_fd);
            (*wp).pipe_fd = -1;

            if (window_pane_destroy_ready(wp) != 0) {
                server_destroy_pane(wp, 1);
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }

        /* If no pipe command, that is enough. */
        if (args_count(args) == 0 || *args_string(args, 0) == b'\0' as _) {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        /*
         * With -o, only open the new pipe if there was no previous one. This
         * allows a pipe to be toggled with a single key, for example:
         *
         *	bind ^p pipep -o 'cat >>~/output'
         */
        if (args_has_(args, 'o') && old_fd != -1) {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        /* What do we want to do? Neither -I or -O is -O. */
        if (args_has_(args, 'I')) {
            in_ = 1;
            out = args_has(args, b'O');
        } else {
            in_ = 0;
            out = 1;
        }

        /* Open the new pipe. */
        if (socketpair(AF_UNIX, libc::SOCK_STREAM, PF_UNSPEC, pipe_fd.as_mut_ptr()) != 0) {
            cmdq_error(item, c"socketpair error: %s".as_ptr(), strerror(errno!()));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        /* Expand the command. */
        let ft = format_create(cmdq_get_client(item), item, FORMAT_NONE, 0);
        format_defaults(ft, tc, s, wl, wp);
        let cmd = format_expand_time(ft, args_string(args, 0));
        format_free(ft);

        /* Fork the child. */
        sigfillset(&raw mut set);
        sigprocmask(SIG_BLOCK, &raw const set, &raw mut oldset);
        match (libc::fork()) {
            -1 => {
                sigprocmask(SIG_SETMASK, &raw const oldset, null_mut());
                cmdq_error(item, c"fork error: %s".as_ptr(), strerror(errno!()));

                free_(cmd);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            0 => {
                proc_clear_signals(server_proc, 1);
                sigprocmask(SIG_SETMASK, &oldset, null_mut());
                close(pipe_fd[0]);

                let null_fd = open(_PATH_DEVNULL, O_WRONLY);
                if (out != 0) {
                    if (dup2(pipe_fd[1], STDIN_FILENO) == -1) {
                        _exit(1);
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if (dup2(null_fd, STDIN_FILENO) == -1) {
                        _exit(1);
                    }
                }
                if (in_ != 0) {
                    if (dup2(pipe_fd[1], STDOUT_FILENO) == -1) {
                        _exit(1);
                    }
                    if (pipe_fd[1] != STDOUT_FILENO) {
                        close(pipe_fd[1]);
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if (dup2(null_fd, STDOUT_FILENO) == -1) {
                        _exit(1);
                    }
                }
                if (dup2(null_fd, STDERR_FILENO) == -1) {
                    _exit(1);
                }
                closefrom(STDERR_FILENO + 1);

                execl(_PATH_BSHELL, c"sh".as_ptr(), c"-c".as_ptr(), cmd, null_mut::<c_void>());
                _exit(1)
            }
            _ => {
                /* Parent process. */
                sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
                close(pipe_fd[1]);

                (*wp).pipe_fd = pipe_fd[0];
                memcpy__(wpo, &raw mut (*wp).offset);

                setblocking((*wp).pipe_fd, 0);
                (*wp).pipe_event = bufferevent_new(
                    (*wp).pipe_fd,
                    Some(cmd_pipe_pane_read_callback),
                    Some(cmd_pipe_pane_write_callback),
                    Some(cmd_pipe_pane_error_callback),
                    wp.cast(),
                );
                if ((*wp).pipe_event.is_null()) {
                    fatalx(c"out of memory".as_ptr());
                }
                if (out != 0) {
                    bufferevent_enable((*wp).pipe_event, EV_WRITE as i16);
                }
                if (in_ != 0) {
                    bufferevent_enable((*wp).pipe_event, EV_READ as i16);
                }

                free_(cmd);
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_pipe_pane_read_callback(_bufev: *mut bufferevent, data: *mut c_void) {
    unsafe {
        let wp: *mut window_pane = data as *mut window_pane;
        let mut evb = (*(*wp).pipe_event).input;

        let available = EVBUFFER_LENGTH(evb);
        log_debug(c"%%%u pipe read %zu".as_ptr(), (*wp).id, available);

        bufferevent_write((*wp).event, EVBUFFER_DATA(evb).cast(), available);
        evbuffer_drain(evb, available);

        if (window_pane_destroy_ready(wp) != 0) {
            server_destroy_pane(wp, 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_pipe_pane_write_callback(_bufev: *mut bufferevent, data: *mut c_void) {
    unsafe {
        let wp: *mut window_pane = data as *mut window_pane;

        log_debug(c"%%%u pipe empty".as_ptr(), (*wp).id);

        if (window_pane_destroy_ready(wp) != 0) {
            server_destroy_pane(wp, 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_pipe_pane_error_callback(_bufev: *mut bufferevent, _what: i16, data: *mut c_void) {
    unsafe {
        let wp: *mut window_pane = data as *mut window_pane;

        log_debug(c"%%%u pipe error".as_ptr(), (*wp).id);

        bufferevent_free((*wp).pipe_event);
        close((*wp).pipe_fd);
        (*wp).pipe_fd = -1;

        if (window_pane_destroy_ready(wp) != 0) {
            server_destroy_pane(wp, 1);
        }
    }
}
