use crate::{xmalloc::xstrndup, *};

unsafe extern "C" {
    // pub unsafe static mut global_options: *mut options = null_mut();
    // pub unsafe static mut global_s_options: *mut options = null_mut();
    // pub unsafe static mut global_w_options: *mut options = null_mut();
    // pub unsafe static mut global_environ: *mut environ = null_mut();

    // pub unsafe static mut start_time: timeval = unsafe { zeroed() };
    // pub unsafe static mut socket_path: *mut c_char = null_mut();
    // pub unsafe static mut ptm_fd: c_int = -1;
    // pub unsafe static mut shell_command: *mut c_char = null_mut();

    // pub unsafe fn checkshell(_: *const c_char) -> c_int;
    // pub unsafe fn setblocking(_: c_int, _: c_int);
    // pub unsafe fn shell_argv0(_: *const c_char, _: c_int) -> *mut c_char;
    // pub unsafe fn get_timer() -> u64;
    // pub unsafe fn sig2name(_: i32) -> *mut c_char;
    // pub unsafe fn find_cwd() -> *mut c_char;
    // pub unsafe fn find_home() -> *mut c_char;
    // pub unsafe fn getversion() -> *mut c_char;

    // TODO move/remove
    fn errx(_: c_int, _: *const c_char, ...);
    fn err(_: c_int, _: *const c_char, ...);

    fn tzset();
}

use compat_rs::{fdforkpty::getptmfd, getprogname::getprogname, optarg, optind};
use libc::{
    __errno_location, CLOCK_MONOTONIC, CLOCK_REALTIME, CODESET, EEXIST, F_GETFL, F_SETFL, LC_CTYPE, LC_TIME,
    O_NONBLOCK, PATH_MAX, S_IRWXO, S_IRWXU, X_OK, access, clock_gettime, fcntl, fprintf, getcwd, getenv, getopt,
    getpwuid, getuid, lstat, mkdir, nl_langinfo, printf, realpath, setlocale, stat, strcasecmp, strcasestr, strchr,
    strcmp, strcspn, strerror, strncmp, strrchr, strstr, timespec,
};

#[unsafe(no_mangle)]
pub static mut global_options: *mut options = null_mut();
#[unsafe(no_mangle)]
pub static mut global_s_options: *mut options = null_mut();
#[unsafe(no_mangle)]
pub static mut global_w_options: *mut options = null_mut();
#[unsafe(no_mangle)]
pub static mut global_environ: *mut environ = null_mut();

#[unsafe(no_mangle)]
pub static mut start_time: timeval = unsafe { zeroed() };
#[unsafe(no_mangle)]
pub static mut socket_path: *const c_char = null_mut();
#[unsafe(no_mangle)]
pub static mut ptm_fd: c_int = -1;
#[unsafe(no_mangle)]
pub static mut shell_command: *mut c_char = null_mut();

#[unsafe(no_mangle)]
pub extern "C" fn usage() -> ! {
    unsafe {
        fprintf(stderr, c"usage: %s [-2CDlNuVv] [-c shell-command] [-f file] [-L socket-name]\n            [-S socket-path] [-T features] [command [flags]]\n".as_ptr(), getprogname());
        std::process::exit(1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getshell() -> *const c_char {
    unsafe {
        let shell = getenv(c"SHELL".as_ptr());
        if checkshell(shell) != 0 {
            return shell;
        }

        let pw = getpwuid(getuid());
        if !pw.is_null() && checkshell((*pw).pw_shell) != 0 {
            return (*pw).pw_shell;
        }

        _PATH_BSHELL
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn checkshell(shell: *const c_char) -> c_int {
    unsafe {
        if shell.is_null() || *shell != b'/' as c_char {
            return 0;
        }
        if areshell(shell) != 0 {
            return 0;
        }
        if access(shell, X_OK) != 0 {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn areshell(shell: *const c_char) -> c_int {
    unsafe {
        let ptr = strrchr(shell, b'/' as c_int);
        let ptr = if !ptr.is_null() { ptr.wrapping_add(1) } else { shell };
        let mut progname = getprogname();
        if *progname == b'-' as c_char {
            progname = progname.wrapping_add(1);
        }
        if strcmp(ptr, progname) == 0 { 1 } else { 0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn expand_path(path: *const c_char, home: *const c_char) -> *mut c_char {
    unsafe {
        let mut expanded: *mut c_char = null_mut();
        let mut end: *const c_char = null_mut();

        if strncmp(path, c"~/".as_ptr(), 2) == 0 {
            if home.is_null() {
                return null_mut();
            }
            xasprintf(&raw mut expanded, c"%s%s".as_ptr(), home, path.add(1));
            return expanded;
        }

        if *path == b'$' as c_char {
            end = strchr(path, b'/' as i32);
            let name = if end.is_null() {
                xstrdup(path.add(1)).cast().as_ptr()
            } else {
                xstrndup(path.add(1), end.addr() - path.addr() - 1).cast().as_ptr()
            };
            let mut value = environ_find(global_environ, name);
            free_(name);
            if value.is_null() {
                return null_mut();
            }
            if end.is_null() {
                end = c"".as_ptr();
            }
            xasprintf(&raw mut expanded, c"%s%s".as_ptr(), (*value).value, end);
            return (expanded);
        }

        xstrdup(path).cast().as_ptr()
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn expand_paths(s: *const c_char, paths: *mut *mut *mut c_char, n: *mut u32, ignore_errors: i32) {
    unsafe {
        let home = find_home();
        let mut next: *const c_char = null_mut();
        let mut resolved: [c_char; PATH_MAX as usize] = zeroed(); // TODO use unint version
        let mut path = null_mut();

        let func = c"expand_paths".as_ptr();

        *paths = null_mut();
        *n = 0;

        let mut tmp: *mut c_char = xstrdup(s).cast().as_ptr();
        let mut copy = tmp;
        while ({
            next = strsep(&raw mut tmp as _, c":".as_ptr().cast());
            !next.is_null()
        }) {
            let expanded = expand_path(next, home);
            if expanded.is_null() {
                log_debug(c"%s: invalid path: %s".as_ptr(), func, next);
                continue;
            }
            if realpath(expanded, resolved.as_mut_ptr()).is_null() {
                log_debug(
                    c"%s: realpath(\"%s\") failed: %s".as_ptr(),
                    func,
                    expanded,
                    strerror(*__errno_location()),
                );
                if ignore_errors != 0 {
                    free_(expanded);
                    continue;
                }
                path = expanded;
            } else {
                path = xstrdup(resolved.as_ptr()).cast().as_ptr();
                free_(expanded);
            }
            let mut i = 0;
            for j in 0..*n {
                i = j;
                if strcmp(path as _, *(*paths).add(i as usize)) == 0 {
                    break;
                }
            }
            if (i != *n) {
                log_debug(c"%s: duplicate path: %s".as_ptr(), func, path);
                free_(path);
                continue;
            }
            *paths = xreallocarray_::<*mut c_char>(*paths, (*n + 1) as usize).as_ptr();
            *(*paths).add((*n) as usize) = path;
            *n += 1;
        }
        free_(copy);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn make_label(mut label: *const c_char, cause: *mut *mut c_char) -> *const c_char {
    let mut paths: *mut *mut c_char = null_mut();
    let mut path: *mut c_char = null_mut();
    let mut base: *mut c_char = null_mut();
    let mut sb: stat = unsafe { zeroed() }; // TODO use uninit
    let mut n: u32 = 0;

    unsafe {
        'fail: {
            *cause = null_mut();
            if label.is_null() {
                label = c"default".as_ptr();
            }
            let uid = getuid();

            expand_paths(TMUX_SOCK.as_ptr(), &raw mut paths, &raw mut n, 1);
            if n == 0 {
                xasprintf(cause, c"no suitable socket path".as_ptr());
                return null_mut();
            }
            let mut path = *paths; /* can only have one socket! */
            for i in 1..n {
                free_(*paths.add(i as usize));
            }
            free_(paths);

            xasprintf(&raw mut base, c"%s/tmux-%ld".as_ptr(), path, uid as c_long);
            free_(path);
            if mkdir(base, S_IRWXU) != 0 && *__errno_location() != EEXIST {
                xasprintf(
                    cause,
                    c"couldn't create directory %s (%s)".as_ptr(),
                    base,
                    strerror(*__errno_location()),
                );
                break 'fail;
            }
            if lstat(base, &raw mut sb) != 0 {
                xasprintf(
                    cause,
                    c"couldn't read directory %s (%s)".as_ptr(),
                    base,
                    strerror(*__errno_location()),
                );
                break 'fail;
            }
            if !S_ISDIR(sb.st_mode) {
                xasprintf(cause, c"%s is not a directory".as_ptr(), base);
                break 'fail;
            }
            if sb.st_uid != uid || (sb.st_mode & S_IRWXO) != 0 {
                xasprintf(cause, c"directory %s has unsafe permissions".as_ptr(), base);
                break 'fail;
            }
            xasprintf(&raw mut path, c"%s/%s".as_ptr(), base, label);
            free_(base);
            return path;
        }

        // fail:
        free_(base);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn shell_argv0(shell: *const c_char, is_login: c_int) -> *mut c_char {
    unsafe {
        let mut argv0 = null_mut();

        let slash = strrchr(shell, b'/' as _);
        let name = if !slash.is_null() && *slash.add(1) != b'\0' as c_char {
            slash.add(1)
        } else {
            shell
        };

        if is_login != 0 {
            xasprintf(&raw mut argv0, c"-%s".as_ptr(), name);
        } else {
            xasprintf(&raw mut argv0, c"%s".as_ptr(), name);
        }

        argv0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn setblocking(fd: c_int, state: c_int) {
    unsafe {
        let mut mode = fcntl(fd, F_GETFL);

        if mode != -1 {
            if state == 0 {
                mode |= O_NONBLOCK;
            } else {
                mode &= !O_NONBLOCK;
            }
            fcntl(fd, F_SETFL, mode);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_timer() -> u64 {
    unsafe {
        let mut ts: timespec = zeroed();
        //We want a timestamp in milliseconds suitable for time measurement,
        //so prefer the monotonic clock.
        if clock_gettime(CLOCK_MONOTONIC, &raw mut ts) != 0 {
            clock_gettime(CLOCK_REALTIME, &raw mut ts);
        }
        (ts.tv_sec as u64 * 1000) + (ts.tv_nsec as u64 / 1000000)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sig2name(signo: i32) -> *mut c_char {
    static mut s: [c_char; 11] = unsafe { zeroed() };

    unsafe {
        /*
                // TODO
                // #ifdef HAVE_SYS_SIGNAME
                #[cfg(feature = "sys_signame")]
                {
                    if (signo > 0 && signo < NSIG) {
                        return sys_signame[signo];
                    }
                }
        */
        xsnprintf(&raw mut s as _, size_of::<[c_char; 11]>(), c"%d".as_ptr(), signo);
        &raw mut s as _
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_cwd() -> *mut c_char {
    static mut cwd: [c_char; PATH_MAX as usize] = unsafe { zeroed() };
    unsafe {
        let mut resolved1: [c_char; PATH_MAX as usize] = unsafe { zeroed() };
        let mut resolved2: [c_char; PATH_MAX as usize] = unsafe { zeroed() };

        if getcwd(&raw mut cwd as _, size_of::<[c_char; PATH_MAX as usize]>()).is_null() {
            return null_mut();
        }
        let pwd = getenv(c"PWD".as_ptr());
        if pwd.is_null() || *pwd == b'\0' as c_char {
            return &raw mut cwd as _;
        }

        //We want to use PWD so that symbolic links are maintained,
        //but only if it matches the actual working directory.

        if realpath(pwd, &raw mut resolved1 as _).is_null() {
            return &raw mut cwd as _;
        }
        if realpath(&raw mut cwd as _, &raw mut resolved2 as _).is_null() {
            return &raw mut cwd as _;
        }
        if strcmp(&raw mut resolved1 as _, &raw mut resolved2 as _) != 0 {
            return &raw mut cwd as _;
        }
        pwd
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_home() -> *mut c_char {
    static mut home: *mut c_char = null_mut();

    unsafe {
        if !home.is_null() {
            home
        } else {
            home = getenv(c"HOME".as_ptr());
            if home.is_null() || *home == b'\0' as c_char {
                let pw = getpwuid(getuid());
                if !pw.is_null() {
                    home = (*pw).pw_dir;
                } else {
                    home = null_mut();
                }
            }

            home
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn getversion() -> *const c_char {
    // TODO get this from build config somehow
    c"3.5rs".as_ptr()
}

#[unsafe(no_mangle)]
extern "C" fn main(mut argc: i32, mut argv: *mut *mut c_char) {
    unsafe {
        let mut cause: *mut c_char = null_mut();
        let mut path: *const c_char = null_mut();
        let mut label: *mut c_char = null_mut();
        let mut feat: i32 = 0;
        let mut fflag: i32 = 0;
        let mut flags: u64 = 0;

        if setlocale(LC_CTYPE, c"en_US.UTF-8".as_ptr()).is_null() && setlocale(LC_CTYPE, c"C.UTF-8".as_ptr()).is_null()
        {
            if setlocale(LC_CTYPE, c"".as_ptr()).is_null() {
                errx(1, c"invalid LC_ALL, LC_CTYPE or LANG".as_ptr());
            }
            let s = nl_langinfo(CODESET);
            if strcasecmp(s, c"UTF-8".as_ptr()) != 0 && strcasecmp(s, c"UTF8".as_ptr()) != 0 {
                errx(1, c"need UTF-8 locale (LC_CTYPE) but have %s".as_ptr(), s);
            }
        }

        setlocale(LC_TIME, c"".as_ptr());
        tzset();

        if **argv == b'-' as c_char {
            flags = CLIENT_LOGIN;
        }

        global_environ = environ_create().as_ptr();

        let mut var = environ;
        while !(*var).is_null() {
            environ_put(global_environ, *var, 0);
            var = var.add(1);
        }

        let cwd = find_cwd();
        if !cwd.is_null() {
            environ_set(global_environ, c"PWD".as_ptr(), 0, c"%s".as_ptr(), cwd);
        }
        expand_paths(TMUX_CONF.as_ptr(), &raw mut cfg_files, &raw mut cfg_nfiles, 1);

        let mut opt;
        while ({
            opt = getopt(argc, argv, c"2c:CDdf:lL:NqS:T:uUvV".as_ptr());
            opt != -1
        }) {
            match opt as u8 {
                b'2' => tty_add_features(&raw mut feat, c"256".as_ptr(), c":,".as_ptr()),
                b'c' => shell_command = optarg,
                b'D' => flags |= CLIENT_NOFORK,
                b'C' => {
                    if flags & CLIENT_CONTROL != 0 {
                        flags |= CLIENT_CONTROLCONTROL;
                    } else {
                        flags |= CLIENT_CONTROL;
                    }
                }
                b'f' => {
                    if fflag == 0 {
                        fflag = 1;
                        for i in 0..cfg_nfiles {
                            free((*cfg_files.add(i as usize)) as _);
                        }
                        cfg_nfiles = 0;
                    }
                    cfg_files = xreallocarray_::<*mut c_char>(cfg_files, cfg_nfiles as usize + 1).as_ptr();
                    *cfg_files.add(cfg_nfiles as usize) = xstrdup(optarg).cast().as_ptr();
                    cfg_nfiles += 1;
                    cfg_quiet = 0;
                }
                b'V' => {
                    printf(c"tmux %s\n".as_ptr(), getversion());
                    std::process::exit(0);
                }
                b'l' => flags |= CLIENT_LOGIN,
                b'L' => {
                    free(label as _);
                    label = xstrdup(optarg).cast().as_ptr();
                }
                b'N' => flags |= CLIENT_NOSTARTSERVER,
                b'q' => (),
                b'S' => {
                    free(path as _);
                    path = xstrdup(optarg).cast().as_ptr();
                }
                b'T' => tty_add_features(&raw mut feat, optarg, c":,".as_ptr()),
                b'u' => flags |= CLIENT_UTF8,
                b'v' => log_add_level(),
                _ => usage(),
            }
        }
        argc -= optind;
        argv = argv.add(optind as usize);

        if !shell_command.is_null() && argc != 0 {
            usage();
        }
        if flags & CLIENT_NOFORK != 0 && argc != 0 {
            usage();
        }

        ptm_fd = getptmfd();
        if ptm_fd == -1 {
            err(1, c"getptmfd".as_ptr());
        }

        /*
        // TODO no pledge on linux
            if pledge("stdio rpath wpath cpath flock fattr unix getpw sendfd recvfd proc exec tty ps", null_mut()) != 0 {
                err(1, "pledge");
        }
        */

        // tmux is a UTF-8 terminal, so if TMUX is set, assume UTF-8.
        // Otherwise, if the user has set LC_ALL, LC_CTYPE or LANG to contain
        // UTF-8, it is a safe assumption that either they are using a UTF-8
        // terminal, or if not they know that output from UTF-8-capable
        // programs may be wrong.
        if !getenv(c"TMUX".as_ptr()).is_null() {
            flags |= CLIENT_UTF8;
        } else {
            let mut s = getenv(c"LC_ALL".as_ptr()) as *const c_char;
            if s.is_null() || *s == b'\0' as c_char {
                s = getenv(c"LC_CTYPE".as_ptr()) as *const c_char;
            }
            if s.is_null() || *s == b'\0' as c_char {
                s = getenv(c"LANG".as_ptr()) as *const c_char;
            }
            if s.is_null() || *s == b'\0' as c_char {
                s = c"".as_ptr();
            }
            if !strcasestr(s, c"UTF-8".as_ptr()).is_null() || !strcasestr(s, c"UTF8".as_ptr()).is_null() {
                flags |= CLIENT_UTF8;
            }
        }

        global_options = options_create(null_mut());
        global_s_options = options_create(null_mut());
        global_w_options = options_create(null_mut());

        let mut oe: *const options_table_entry = &raw const options_table as _;
        while !(*oe).name.is_null() {
            if (*oe).scope & OPTIONS_TABLE_SERVER != 0 {
                options_default(global_options, oe);
            }
            if (*oe).scope & OPTIONS_TABLE_SESSION != 0 {
                options_default(global_s_options, oe);
            }
            if (*oe).scope & OPTIONS_TABLE_WINDOW != 0 {
                options_default(global_w_options, oe);
            }
            oe = oe.add(1);
        }

        // The default shell comes from SHELL or from the user's passwd entry if available.
        options_set_string(
            global_s_options,
            c"default-shell".as_ptr(),
            0,
            c"%s".as_ptr(),
            getshell(),
        );

        // Override keys to vi if VISUAL or EDITOR are set.
        let mut s = getenv(c"VISUAL".as_ptr());
        if !s.is_null()
            || ({
                s = getenv(c"EDITOR".as_ptr());
                !s.is_null()
            })
        {
            options_set_string(global_options, c"editor".as_ptr(), 0, c"%s".as_ptr(), s);
            if !strrchr(s, b'/' as _).is_null() {
                s = strrchr(s, b'/' as _).add(1);
            }
            let keys = if !strstr(s, c"vi".as_ptr()).is_null() {
                MODEKEY_VI
            } else {
                MODEKEY_EMACS
            };
            options_set_number(global_s_options, c"status-keys".as_ptr(), keys as _);
            options_set_number(global_w_options, c"mode-keys".as_ptr(), keys as _);
        }

        // If socket is specified on the command-line with -S or -L, it is
        // used. Otherwise, $TMUX is checked and if that fails "default" is
        // used.
        if path.is_null() && label.is_null() {
            s = getenv(c"TMUX".as_ptr());
            if !s.is_null() && *s != b'\0' as c_char && *s != b',' as c_char {
                let mut tmp: *mut c_char = xstrdup(s).cast().as_ptr();
                *tmp.add(strcspn(tmp, c",".as_ptr())) = b'\0' as c_char;
                path = tmp;
            }
        }
        if path.is_null() {
            path = make_label(label.cast(), &raw mut cause);
            if path.is_null() {
                if !cause.is_null() {
                    fprintf(stderr, c"%s\n".as_ptr(), cause);
                    free(cause as _);
                }
                std::process::exit(1);
            }
            flags |= CLIENT_DEFAULTSOCKET;
        }
        socket_path = path;
        free(label as _);

        // Pass control to the client.
        std::process::exit(client_main(osdep_event_init(), argc, argv, flags, feat));
    }
}
