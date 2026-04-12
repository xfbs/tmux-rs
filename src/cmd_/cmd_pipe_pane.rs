// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::compat::closefrom;
use crate::libc::{
    _exit, AF_UNIX, O_WRONLY, PF_UNSPEC, SIG_BLOCK, SIG_SETMASK, STDERR_FILENO, STDIN_FILENO,
    STDOUT_FILENO, close, dup2, execl, open, sigfillset, sigprocmask, sigset_t, socketpair,
};
use crate::*;

pub static CMD_PIPE_PANE_ENTRY: cmd_entry = cmd_entry {
    name: "pipe-pane",
    alias: Some("pipep"),

    args: args_parse::new("IOot:", 0, 1, None),
    usage: "[-IOo] [-t target-pane] [shell-command]",

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, cmd_find_flags::empty()),
    source: cmd_entry_flag::zeroed(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_pipe_pane_exec,
};

pub unsafe fn cmd_pipe_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);
        let wp = (*target).wp.and_then(|id| pane_from_id(id)).unwrap_or(null_mut());
        let s = (*target).s.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
        let wl = (*target).wl;
        let wpo = &raw mut (*wp).pipe_offset;
        let mut pipe_fd: [i32; 2] = [0; 2];
        let in_: bool;
        let out: bool;
        let mut set: sigset_t = zeroed(); // TODO uninit
        let mut oldset: sigset_t = zeroed(); // TODO uninit

        // Do nothing if pane is dead.
        if window_pane_exited(wp) {
            cmdq_error!(item, "target pane has exited");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        // Destroy the old pipe.
        let old_fd = (*wp).pipe_fd;
        if (*wp).pipe_fd != -1 {
            (*wp).pipe_read = None;
            (*wp).pipe_write = None;
            close((*wp).pipe_fd);
            (*wp).pipe_fd = -1;

            if window_pane_destroy_ready(&*wp) {
                server_destroy_pane(wp, 1);
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }

        // If no pipe command, that is enough.
        if args_count(args) == 0 || *args_string(args, 0) == b'\0' {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        // With -o, only open the new pipe if there was no previous one. This
        // allows a pipe to be toggled with a single key, for example:
        //
        // 	bind ^p pipep -o 'cat >>~/output'
        if args_has(args, 'o') && old_fd != -1 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        // What do we want to do? Neither -I or -O is -O.
        if args_has(args, 'I') {
            in_ = true;
            out = args_has(args, 'O');
        } else {
            in_ = false;
            out = true;
        }

        // Open the new pipe.
        if socketpair(AF_UNIX, libc::SOCK_STREAM, PF_UNSPEC, pipe_fd.as_mut_ptr()) != 0 {
            cmdq_error!(item, "socketpair error: {}", strerror(errno!()));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        // Expand the command.
        let ft = format_create(
            cmdq_get_client(item),
            item,
            FORMAT_NONE,
            format_flags::empty(),
        );
        format_defaults(ft, tc, NonNull::new(s), NonNull::new(wl), NonNull::new(wp));
        let cmd = format_expand_time(ft, args_string(args, 0));
        format_free(ft);

        // Fork the child.
        sigfillset(&raw mut set);
        sigprocmask(SIG_BLOCK, &raw const set, &raw mut oldset);
        match libc::fork() {
            -1 => {
                sigprocmask(SIG_SETMASK, &raw const oldset, null_mut());
                cmdq_error!(item, "fork error: {}", strerror(errno!()));

                free_(cmd);
                cmd_retval::CMD_RETURN_ERROR
            }
            0 => {
                proc_clear_signals(SERVER_PROC, 1);
                sigprocmask(SIG_SETMASK, &oldset, null_mut());
                proc_unblock_signals();
                close(pipe_fd[0]);

                let null_fd = open(_PATH_DEVNULL, O_WRONLY, 0);
                #[expect(clippy::collapsible_else_if)]
                if out {
                    if dup2(pipe_fd[1], STDIN_FILENO) == -1 {
                        _exit(1);
                    }
                } else {
                    if dup2(null_fd, STDIN_FILENO) == -1 {
                        _exit(1);
                    }
                }
                #[expect(clippy::collapsible_else_if)]
                if in_ {
                    if dup2(pipe_fd[1], STDOUT_FILENO) == -1 {
                        _exit(1);
                    }
                    if pipe_fd[1] != STDOUT_FILENO {
                        close(pipe_fd[1]);
                    }
                } else {
                    if dup2(null_fd, STDOUT_FILENO) == -1 {
                        _exit(1);
                    }
                }
                if dup2(null_fd, STDERR_FILENO) == -1 {
                    _exit(1);
                }
                closefrom(STDERR_FILENO + 1);

                execl(
                    _PATH_BSHELL.cast(),
                    c"sh".as_ptr(),
                    c"-c".as_ptr(),
                    cmd,
                    null_mut::<c_void>(),
                );
                _exit(1)
            }
            _ => {
                // Parent process.
                sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());
                close(pipe_fd[1]);

                (*wp).pipe_fd = pipe_fd[0];
                memcpy__(wpo, &raw mut (*wp).offset);

                setblocking((*wp).pipe_fd, 0);
                let pid = PaneId((*wp).id);
                if out {
                    (*wp).pipe_write = io_register(
                        (*wp).pipe_fd,
                        EV_WRITE,
                        Box::new(move |_fd, _events| unsafe {
                            cmd_pipe_pane_write_fire(pid);
                        }),
                    );
                }
                if in_ {
                    (*wp).pipe_read = io_register(
                        (*wp).pipe_fd,
                        EV_READ,
                        Box::new(move |_fd, _events| unsafe {
                            cmd_pipe_pane_read_fire(pid);
                        }),
                    );
                }

                free_(cmd);
                cmd_retval::CMD_RETURN_NORMAL
            }
        }
    }
}

/// Read callback: reads data from the pipe fd into pipe_input,
/// then writes it to the pane's PTY via bufferevent_write.
unsafe fn cmd_pipe_pane_read_fire(pid: PaneId) {
    unsafe {
        let Some(wp) = pane_from_id(pid) else { return };

        let n = (*wp).pipe_input.read_from_fd((*wp).pipe_fd, 4096);
        if n <= 0 {
            if n < 0
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock
            {
                return;
            }
            // EOF or error on the pipe.
            cmd_pipe_pane_error_fire(wp);
            return;
        }

        let available = (*wp).pipe_input.len();
        log_debug!("%%{} pipe read {}", (*wp).id, available);

        // Forward pipe data to the pane's PTY (still a bufferevent).
        window_pane_write_to_pty(wp, (*wp).pipe_input.as_mut_ptr().cast(), available);
        (*wp).pipe_input.drain(available);

        if window_pane_destroy_ready(&*wp) {
            server_destroy_pane(wp, 1);
        }
    }
}

/// Write callback: drains pipe_output to the pipe fd.
/// When the buffer is empty, drops the write IoHandle.
pub unsafe fn cmd_pipe_pane_write_fire(pid: PaneId) {
    unsafe {
        let Some(wp) = pane_from_id(pid) else { return };

        if (*wp).pipe_output.len() > 0 {
            let n = (*wp).pipe_output.write_to_fd((*wp).pipe_fd);
            if n < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                    return;
                }
                // Write error on the pipe.
                cmd_pipe_pane_error_fire(wp);
                return;
            }
        }

        if (*wp).pipe_output.is_empty() {
            log_debug!("%%{} pipe empty", (*wp).id);
            // Drop the write IoHandle — will be re-armed when new data arrives.
            (*wp).pipe_write = None;

            if window_pane_destroy_ready(&*wp) {
                server_destroy_pane(wp, 1);
            }
        }
    }
}

/// Error/EOF handler for the pipe fd.
unsafe fn cmd_pipe_pane_error_fire(wp: *mut window_pane) {
    unsafe {
        log_debug!("%%{} pipe error", (*wp).id);

        (*wp).pipe_read = None;
        (*wp).pipe_write = None;
        close((*wp).pipe_fd);
        (*wp).pipe_fd = -1;

        if window_pane_destroy_ready(&*wp) {
            server_destroy_pane(wp, 1);
        }
    }
}
