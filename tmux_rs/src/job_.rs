use super::*;

pub type job_update_cb = Option<unsafe extern "C" fn(*mut job)>;
pub type job_complete_cb = Option<unsafe extern "C" fn(*mut job)>;
pub type job_free_cb = Option<unsafe extern "C" fn(*mut c_void)>;

unsafe extern "C" {
    pub fn job_run(
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
        _: *mut environ,
        _: *mut session,
        _: *const c_char,
        _: job_update_cb,
        _: job_complete_cb,
        _: job_free_cb,
        _: *mut c_void,
        _: c_int,
        _: c_int,
        _: c_int,
    ) -> *mut job;
    pub fn job_free(_: *mut job);
    pub fn job_transfer(_: *mut job, _: *mut pid_t, _: *mut c_char, _: usize) -> c_int;
    pub fn job_resize(_: *mut job, _: c_uint, _: c_uint);
    pub fn job_check_died(_: pid_t, _: c_int);
    pub fn job_get_status(_: *mut job) -> c_int;
    pub fn job_get_data(_: *mut job) -> *mut c_void;
    pub fn job_get_event(_: *mut job) -> *mut bufferevent;
    pub fn job_kill_all();
    pub fn job_still_running() -> c_int;
    pub fn job_print_summary(_: *mut cmdq_item, _: c_int);
}

/*
use crate::*;

use std::mem::MaybeUninit;

use libc::{
    SIG_BLOCK, SOCK_STREAM, TIOCSWINSZ, c_void, chdir, close, dup2, execl, execvp, fork, free, ioctl, kill, killpg,
    memset, pid_t, setenv, sigfillset, sigprocmask, sigset_t, size_t, socketpair, termios, winsize,
};

use libevent_sys::{
    bufferevent, bufferevent_disable, bufferevent_enable, bufferevent_free, bufferevent_get_output, bufferevent_new,
};

use compat_rs::{
    fdforkpty::fdforkpty,
    queue::{ListEntry, list_entry, list_head, list_head_initializer, list_remove},
    strlcpy,
};

#[derive(Eq, PartialEq)]
#[repr(i32)]
enum JobState {
    Running = 0,
    Dead = 1,
    Closed = 2,
}

#[repr(C)]
struct job {
    state: JobState,

    flags: i32,

    cmd: *mut c_char,
    pid: pid_t,
    tty: [c_char; TTY_NAME_MAX as usize],
    status: i32,

    fd: c_int,
    event: *mut bufferevent,

    updatecb: job_update_cb,
    completecb: job_complete_cb,
    freecb: job_free_cb,
    data: *mut c_void,

    entry: list_entry<job>,
}
impl ListEntry<job, ()> for job {
    unsafe fn field(this: *mut Self) -> *mut list_entry<job> {
        unsafe { &raw mut (*this).entry }
    }
}

type joblist = list_head<job>;
static mut all_jobs: joblist = list_head_initializer();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_run2(
    mut cmd: *const c_char,
    mut argc: c_int,
    mut argv: *mut *mut c_char,
    mut e: *mut environ,
    mut s: *mut session,
    mut cwd: *const libc::c_char,
    mut updatecb: job_update_cb,
    mut completecb: job_complete_cb,
    mut freecb: job_free_cb,
    mut data: *mut c_void,
    mut flags: c_int,
    mut sx: c_int,
    mut sy: c_int,
) -> *mut job {
    let func = c"job_run".as_ptr();
    unsafe {
        let env = environ_for_session(s, !cfg_finished);
        if !e.is_null() {
            environ_copy(e, env);
        }

        if !flags & JOB_DEFAULTSHELL {
            shell = _PATH_BSHELL;
        } else {
            if !s.is_null() {
                oo = (*s).options;
            } else {
                oo = global_s_options;
            }
            shell = options_get_String(oo, c"default-shell".as_ptr());
            if !checkshell(shell) {
                shell = _PATH_BSHELL;
            }
        }
        argv0 = shell_argv0(shell, 0);

        sigfillset(&raw mut set);
        sigprocmask(SIG_BLOCK, &raw mut set, &raw mut oldset);

        if flags & JOB_PTY != 0 {
            let mut ws: winsize = zeroed();
            ws.ws_col = sx;
            ws.ws_row = sy;
            pid = fdforkpty(ptm_fd, &raw mut master, tty, null_mut(), &raw mut ws);
        } else {
            if socketpair(AF_UNIX, SOCK_STREAM, PF_UNSPEC, out) != 0 {
                // goto fail;
            }
            pid = fork();
        }
        if cmd.is_null() {
            cmd_log_argv(argc, argv, c"%s:".as_ptr(), func);
            log_debug(
                c"%s: cwd=%s, shell=%s",
                func,
                if cwd.is_null() { c"".as_ptr() } else { cwd },
                shell,
            );
        } else {
            log_debug(
                c"%s: cmd=%s, cwd=%s, shell=%s",
                func,
                cmd,
                if cwd.is_null() { c"".as_ptr() } else { cwd },
                shell,
            );
        }

        match pid {
            -1 => {
                if !flags & JOB_PTY != 0 {
                    close(out[0]);
                    close(out[1]);
                }
                // goto fail;
            }
            0 => {
                proc_clear_signals(server_proc, 1);
                sigprocmask(SIG_SETMASK, &raw mut oldset, null_mut());

                if (cwd.is_null() || chdir(cwd) != 0)
                    && (({
                        home = find_home();
                        home.is_null()
                    }) || chdir(home) != 0)
                    && chdir("/") != 0
                {
                    fatal(c"chdir failed".as_ptr());
                }

                environ_push(env);
                environ_free(env);

                if !flags & JOB_PTY {
                    if dup2(out[1], STDIN_FILENO) == -1 {
                        fatal(c"dup2 failed".as_ptr());
                    }
                    if dup2(out[1], STDOUT_FILENO) == -1 {
                        fatal(c"dup2 failed".as_ptr());
                    }
                    if out[1] != STDIN_FILENO && out[1] != STDOUT_FILENO {
                        close(out[1]);
                    }
                    close(out[0]);

                    nullfd = open(_PATH_DEVNULL, O_RDWR);
                    if (nullfd == -1) {
                        fatal(c"open failed".as_ptr());
                    }
                    if (dup2(nullfd, STDERR_FILENO) == -1) {
                        fatal(c"dup2 failed".as_ptr());
                    }
                    if (nullfd != STDERR_FILENO) {
                        close(nullfd);
                    }
                }
                closefrom(STDERR_FILENO + 1);

                if (cmd != NULL) {
                    setenv("SHELL", shell, 1);
                    execl(shell, argv0, "-c", cmd, NULL);
                    fatal("execl failed");
                } else {
                    argvp = cmd_copy_argv(argc, argv);
                    execvp(argvp[0], argvp);
                    fatal("execvp failed");
                }
            }
        }
    }
}

// TODO retranlate this one
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_run(
    mut cmd: *const libc::c_char,
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
    mut e: *mut environ,
    mut s: *mut session,
    mut cwd: *const libc::c_char,
    mut updatecb: job_update_cb,
    mut completecb: job_complete_cb,
    mut freecb: job_free_cb,
    mut data: *mut libc::c_void,
    mut flags: libc::c_int,
    mut sx: libc::c_int,
    mut sy: libc::c_int,
) -> *mut job {
    unsafe {
        let mut current_block: u64;
        let mut job: *mut job = 0 as *mut job;
        let mut env: *mut environ = 0 as *mut environ;
        let mut pid: pid_t = 0;
        let mut nullfd: libc::c_int = 0;
        let mut out: [libc::c_int; 2] = [0; 2];
        let mut master: libc::c_int = 0;
        let mut home: *const libc::c_char = 0 as *const libc::c_char;
        let mut shell: *const libc::c_char = 0 as *const libc::c_char;
        let mut set: MaybeUninit<sigset_t> = MaybeUninit::zeroed();
        let mut oldset: MaybeUninit<sigset_t> = MaybeUninit::zeroed();
        let mut ws: winsize = winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let mut argvp: *mut *mut libc::c_char = 0 as *mut *mut libc::c_char;
        let mut tty: [libc::c_char; 32] = [0; 32];
        let mut argv0: *mut libc::c_char = 0 as *mut libc::c_char;
        env = environ_for_session(s, (cfg_finished == 0) as libc::c_int);
        if !e.is_null() {
            environ_copy(e, env);
        }
        if !s.is_null() {
            shell = options_get_string((*s).options, c"default-shell".as_ptr());
        } else {
            shell = options_get_string(global_s_options, c"default-shell".as_ptr());
        }
        if checkshell(shell) == 0 {
            shell = c"/bin/sh".as_ptr();
        }
        argv0 = shell_argv0(shell, 0 as libc::c_int);
        sigfillset(set.as_mut_ptr());
        sigprocmask(0 as libc::c_int, set.as_ptr(), oldset.as_mut_ptr());
        if flags & 0x4 as libc::c_int != 0 {
            memset(
                &mut ws as *mut winsize as *mut libc::c_void,
                0 as libc::c_int,
                ::core::mem::size_of::<winsize>(),
            );
            ws.ws_col = sx as libc::c_ushort;
            ws.ws_row = sy as libc::c_ushort;
            pid = fdforkpty(
                ptm_fd,
                &raw mut master,
                tty.as_mut_ptr(),
                std::ptr::null_mut(),
                &raw mut ws,
            );
            current_block = 15652330335145281839;
        } else if socketpair(
            1 as libc::c_int,
            SOCK_STREAM as libc::c_int,
            0 as libc::c_int,
            out.as_mut_ptr(),
        ) != 0 as libc::c_int
        {
            current_block = 4039211401964574181;
        } else {
            pid = fork();
            current_block = 15652330335145281839;
        }
        match current_block {
            15652330335145281839 => {
                if cmd.is_null() {
                    cmd_log_argv(
                        argc,
                        argv,
                        b"%s:\0" as *const u8 as *const libc::c_char,
                        (*::core::mem::transmute::<&[u8; 8], &[libc::c_char; 8]>(b"job_run\0")).as_ptr(),
                    );
                    log_debug(
                        c"%s: cwd=%s, shell=%s".as_ptr(),
                        (*::core::mem::transmute::<&[u8; 8], &[libc::c_char; 8]>(b"job_run\0")).as_ptr(),
                        if cwd.is_null() { c"".as_ptr() } else { cwd },
                        shell,
                    );
                } else {
                    log_debug(
                        b"%s: cmd=%s, cwd=%s, shell=%s\0" as *const u8 as *const libc::c_char,
                        (*::core::mem::transmute::<&[u8; 8], &[libc::c_char; 8]>(b"job_run\0")).as_ptr(),
                        cmd,
                        if cwd.is_null() {
                            b"\0" as *const u8 as *const libc::c_char
                        } else {
                            cwd
                        },
                        shell,
                    );
                }
                match pid {
                    -1 => {
                        if !flags & 0x4 as libc::c_int != 0 {
                            close(out[0 as libc::c_int as usize]);
                            close(out[1 as libc::c_int as usize]);
                        }
                    }
                    0 => {
                        proc_clear_signals(server_proc, 1 as libc::c_int);
                        sigprocmask(2 as libc::c_int, oldset.as_ptr(), 0 as *mut sigset_t);
                        if (cwd.is_null() || chdir(cwd) != 0 as libc::c_int)
                            && {
                                home = find_home();
                                home.is_null() || chdir(home) != 0 as libc::c_int
                            }
                            && chdir(b"/\0" as *const u8 as *const libc::c_char) != 0 as libc::c_int
                        {
                            fatal(b"chdir failed\0" as *const u8 as *const libc::c_char);
                        }
                        environ_push(env);
                        environ_free(env);
                        if !flags & 0x4 as libc::c_int != 0 {
                            if dup2(out[1 as libc::c_int as usize], 0 as libc::c_int) == -(1 as libc::c_int) {
                                fatal(b"dup2 failed\0" as *const u8 as *const libc::c_char);
                            }
                            if dup2(out[1 as libc::c_int as usize], 1 as libc::c_int) == -(1 as libc::c_int) {
                                fatal(b"dup2 failed\0" as *const u8 as *const libc::c_char);
                            }
                            if out[1 as libc::c_int as usize] != 0 as libc::c_int
                                && out[1 as libc::c_int as usize] != 1 as libc::c_int
                            {
                                close(out[1 as libc::c_int as usize]);
                            }
                            close(out[0 as libc::c_int as usize]);
                            nullfd = libc::open(c"/dev/null".as_ptr(), 0o2 as libc::c_int);
                            if nullfd == -(1 as libc::c_int) {
                                fatal(b"open failed\0" as *const u8 as *const libc::c_char);
                            }
                            if libc::dup2(nullfd, 2 as libc::c_int) == -(1 as libc::c_int) {
                                fatal(b"dup2 failed\0" as *const u8 as *const libc::c_char);
                            }
                            if nullfd != 2 as libc::c_int {
                                close(nullfd);
                            }
                        }
                        compat_rs::closefrom(2 as libc::c_int + 1 as libc::c_int);
                        if !cmd.is_null() {
                            setenv(c"SHELL".as_ptr(), shell, 1 as libc::c_int);
                            execl(shell, argv0, c"-c".as_ptr(), cmd, std::ptr::null_mut::<libc::c_char>());
                            fatal(b"execl failed\0" as *const u8 as *const libc::c_char);
                        } else {
                            argvp = cmd_copy_argv(argc, argv);
                            execvp(
                                *argvp.offset(0 as libc::c_int as isize),
                                argvp as *const *const libc::c_char,
                            );
                            fatal(b"execvp failed\0" as *const u8 as *const libc::c_char);
                        }
                    }
                    _ => {
                        sigprocmask(2 as libc::c_int, oldset.as_ptr(), 0 as *mut sigset_t);
                        environ_free(env);
                        free(argv0 as *mut libc::c_void);
                        job = xmalloc(::core::mem::size_of::<job>()).cast().as_ptr();
                        (*job).state = JobState::Running;
                        (*job).flags = flags;
                        if !cmd.is_null() {
                            (*job).cmd = xstrdup(cmd).cast().as_ptr();
                        } else {
                            (*job).cmd = cmd_stringify_argv(argc, argv);
                        }
                        (*job).pid = pid;
                        strlcpy(
                            ((*job).tty).as_mut_ptr(),
                            tty.as_mut_ptr(),
                            ::core::mem::size_of::<[libc::c_char; 32]>(),
                        );
                        (*job).status = 0 as libc::c_int;
                        // (*job).entry.le_next = all_jobs.lh_first;
                        // if !((*job).entry.le_next).is_null() {
                        // (*all_jobs.lh_first).entry.le_prev = &mut (*job).entry.le_next;
                        // }
                        all_jobs.lh_first = job;
                        // (*job).entry.le_prev = &mut all_jobs.lh_first;
                        (*job).updatecb = updatecb;
                        (*job).completecb = completecb;
                        (*job).freecb = freecb;
                        (*job).data = data;
                        if !flags & 0x4 as libc::c_int != 0 {
                            close(out[1 as libc::c_int as usize]);
                            (*job).fd = out[0 as libc::c_int as usize];
                        } else {
                            (*job).fd = master;
                        }
                        setblocking((*job).fd, 0 as libc::c_int);
                        (*job).event = bufferevent_new(
                            (*job).fd,
                            Some(job_read_callback),
                            Some(job_write_callback),
                            Some(job_error_callback),
                            job as *mut libc::c_void,
                        );
                        if ((*job).event).is_null() {
                            fatalx(b"out of memory\0" as *const u8 as *const libc::c_char);
                        }
                        bufferevent_enable((*job).event, (0x2 as libc::c_int | 0x4 as libc::c_int) as libc::c_short);
                        log_debug(
                            c"run job %p: %s, pid %ld".as_ptr(),
                            job,
                            (*job).cmd,
                            (*job).pid as libc::c_long,
                        );
                        return job;
                    }
                }
            }
            _ => {}
        }
        sigprocmask(2, oldset.as_ptr(), std::ptr::null_mut());
        environ_free(env);
        free(argv0 as *mut libc::c_void);
        return 0 as *mut job;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_transfer(
    mut job: *mut job,
    mut pid: *mut pid_t,
    mut tty: *mut c_char,
    mut ttylen: size_t,
) -> c_int {
    unsafe {
        let mut fd = (*job).fd;

        log_debug(c"transfer job %p: %s".as_ptr(), job, (*job).cmd);

        if !pid.is_null() {
            *pid = (*job).pid;
        }
        if !tty.is_null() {
            strlcpy(tty, ((*job).tty).as_mut_ptr(), ttylen);
        }

        list_remove(job);
        free((*job).cmd as _);

        if let Some(freecb) = (*job).freecb
            && !(*job).data.is_null()
        {
            freecb((*job).data);
        }

        if !(*job).event.is_null() {
            bufferevent_free((*job).event);
        }

        free(job as _);
        fd
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_free(mut job: *mut job) {
    unsafe {
        log_debug(c"free job %p: %s".as_ptr(), job, (*job).cmd);
        // if !((*job).entry.le_next).is_null() {
        // (*(*job).entry.le_next).entry.le_prev = (*job).entry.le_prev;
        // }
        // *(*job).entry.le_prev = (*job).entry.le_next;
        free((*job).cmd as *mut libc::c_void);
        if ((*job).freecb).is_some() && !((*job).data).is_null() {
            ((*job).freecb).expect("non-null function pointer")((*job).data);
        }
        if (*job).pid != -(1 as libc::c_int) {
            kill((*job).pid, 15 as libc::c_int);
        }
        if !((*job).event).is_null() {
            bufferevent_free((*job).event);
        }
        if (*job).fd != -(1 as libc::c_int) {
            close((*job).fd);
        }
        free(job as _);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_resize(mut job: *mut job, mut sx: c_uint, mut sy: c_uint) {
    unsafe {
        let mut ws: winsize = zeroed();
        if (*job).fd == -1 || !(*job).flags & JOB_PTY != 0 {
            return;
        }
        log_debug(c"resize job %p: %ux%u".as_ptr(), job, sx, sy);
        ws.ws_col = sx as c_ushort;
        ws.ws_row = sy as c_ushort;

        if ioctl((*job).fd, TIOCSWINSZ, &raw mut ws) == -1 {
            fatal(c"ioctl failed".as_ptr());
        }
    }
}

unsafe extern "C" fn job_read_callback(mut bufev: *mut bufferevent, mut data: *mut libc::c_void) {
    unsafe {
        let mut job: *mut job = data as *mut job;
        if ((*job).updatecb).is_some() {
            ((*job).updatecb).expect("non-null function pointer")(job);
        }
    }
}
unsafe extern "C" fn job_write_callback(mut bufev: *mut bufferevent, mut data: *mut libc::c_void) {
    unsafe {
        let mut job: *mut job = data as *mut job;
        let mut len: size_t = evbuffer_get_length(bufferevent_get_output((*job).event));
        log_debug(
            b"job write %p: %s, pid %ld, output left %zu\0" as *const u8 as *const libc::c_char,
            job,
            (*job).cmd,
            (*job).pid as libc::c_long,
            len,
        );
        const SHUT_WR: libc::c_int = 1;
        if len == 0 && !(*job).flags & 0x2 as libc::c_int != 0 {
            libc::shutdown((*job).fd, SHUT_WR as libc::c_int);
            bufferevent_disable((*job).event, 0x4 as libc::c_int as libc::c_short);
        }
    }
}
unsafe extern "C" fn job_error_callback(
    mut bufev: *mut bufferevent,
    mut events: libc::c_short,
    mut data: *mut libc::c_void,
) {
    unsafe {
        let mut job: *mut job = data as *mut job;
        log_debug(
            b"job error %p: %s, pid %ld\0" as *const u8 as *const libc::c_char,
            job,
            (*job).cmd,
            (*job).pid as libc::c_long,
        );
        if (*job).state == JobState::Dead {
            if ((*job).completecb).is_some() {
                ((*job).completecb).expect("non-null function pointer")(job);
            }
            job_free(job);
        } else {
            bufferevent_disable((*job).event, 0x2 as libc::c_int as libc::c_short);
            (*job).state = JobState::Closed;
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_check_died(mut pid: pid_t, mut status: libc::c_int) {
    unsafe {
        let mut job: *mut job = null_mut();
        job = all_jobs.lh_first;
        while !job.is_null() {
            if pid == (*job).pid {
                break;
            }
            // job = (*job).entry.le_next;
        }
        if job.is_null() {
            return;
        }
        if status & 0xff as libc::c_int == 0x7f as libc::c_int {
            if (status & 0xff00 as libc::c_int) >> 8 as libc::c_int == 21 as libc::c_int
                || (status & 0xff00 as libc::c_int) >> 8 as libc::c_int == 22 as libc::c_int
            {
                return;
            }
            killpg((*job).pid, 18 as libc::c_int);
            return;
        }
        log_debug(
            b"job died %p: %s, pid %ld\0" as *const u8 as *const libc::c_char,
            job,
            (*job).cmd,
            (*job).pid as c_long,
        );
        (*job).status = status;
        if (*job).state == JobState::Closed {
            if ((*job).completecb).is_some() {
                ((*job).completecb).expect("non-null function pointer")(job);
            }
            job_free(job);
        } else {
            (*job).pid = -(1 as libc::c_int);
            (*job).state = JobState::Dead;
        };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_get_status(mut job: *mut job) -> libc::c_int {
    unsafe { (*job).status }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_get_data(mut job: *mut job) -> *mut libc::c_void {
    unsafe { (*job).data }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_get_event(mut job: *mut job) -> *mut bufferevent {
    unsafe { (*job).event }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_kill_all() {
    unsafe {
        let mut job: *mut job = 0 as *mut job;
        job = all_jobs.lh_first;
        while !job.is_null() {
            if (*job).pid != -(1 as libc::c_int) {
                kill((*job).pid, 15 as libc::c_int);
            }
            // job = (*job).entry.le_next;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_still_running() -> libc::c_int {
    unsafe {
        let mut job: *mut job = std::ptr::null_mut();
        job = all_jobs.lh_first;
        while !job.is_null() {
            if !(*job).flags & 0x1 as libc::c_int != 0 && (*job).state == JobState::Running {
                return 1;
            }
            // job = (*job).entry.le_next;
        }

        0
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn job_print_summary(mut item: *mut cmdq_item, mut blank: libc::c_int) {
    unsafe {
        let mut job: *mut job = std::ptr::null_mut();
        let mut n: c_uint = 0;
        job = all_jobs.lh_first;
        while !job.is_null() {
            if blank != 0 {
                cmdq_print(item, c"%s".as_ptr(), c"".as_ptr());
                blank = 0 as libc::c_int;
            }
            cmdq_print(
                item,
                c"Job %u: %s [fd=%d, pid=%ld, status=%d]".as_ptr(),
                n,
                (*job).cmd,
                (*job).fd,
                (*job).pid as libc::c_long,
                (*job).status,
            );
            n = n.wrapping_add(1);
            n;
            // job = (*job).entry.le_next;
        }
    }
}

*/
