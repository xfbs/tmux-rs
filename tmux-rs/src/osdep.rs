use libc::{EOF, TIOCGSID, fgetc, ioctl, readlink, tcgetpgrp};

use crate::*;

// this is for osdep-linux.c

unsafe extern "C" {
    // pub unsafe fn osdep_get_name(_: c_int, _: *mut c_char) -> *mut c_char;
    // pub unsafe fn osdep_get_cwd(_: c_int) -> *mut c_char;
    // pub unsafe fn osdep_event_init() -> *mut event_base;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn osdep_get_name(fd: i32, tty: *const c_char) -> *mut c_char {
    unsafe {
        let mut pgrp = tcgetpgrp(fd);
        if (pgrp == -1) {
            return null_mut();
        }

        let mut path = null_mut();
        xasprintf(&raw mut path, c"/proc/%lld/cmdline".as_ptr(), pgrp as i64);
        let f = fopen(path, c"r".as_ptr());
        if f.is_null() {
            free_(path);
            return null_mut();
        }
        free_(path);

        let mut len = 0;
        let mut buf: *mut c_char = null_mut();

        loop {
            let ch = fgetc(f);
            if ch == EOF {
                break;
            }
            if (ch == b'\0' as i32) {
                break;
            }
            buf = xrealloc_(buf, len + 2).as_ptr();
            *buf.add(len) = ch as c_char;
            len += 1;
        }
        if (!buf.is_null()) {
            *buf.add(len) = b'\0' as c_char;
        }

        fclose(f);
        buf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn osdep_get_cwd(fd: i32) -> *const c_char {
    const MAXPATHLEN: usize = libc::PATH_MAX as usize;
    static mut target_buffer: [c_char; MAXPATHLEN + 1] = [0; MAXPATHLEN + 1];
    unsafe {
        let mut target = &raw mut target_buffer as *mut c_char;

        let mut pgrp = tcgetpgrp(fd);
        if (pgrp == -1) {
            return null_mut();
        }

        let mut path = null_mut();
        xasprintf(&raw mut path, c"/proc/%lld/cwd".as_ptr(), pgrp);
        let mut n = libc::readlink(path, target, MAXPATHLEN);
        free_(path);

        let mut sid: pid_t = 0;
        if (n == -1 && ioctl(fd, TIOCGSID, &raw mut sid) != -1) {
            xasprintf(&raw mut path, c"/proc/%lld/cwd".as_ptr(), sid as i64);
            n = readlink(path, target, MAXPATHLEN);
            free_(path);
        }

        if (n > 0) {
            *target.add(n as usize) = b'\0' as c_char;
            return target;
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn osdep_event_init() -> *mut event_base {
    unsafe {
        // On Linux, epoll doesn't work on /dev/null (yes, really).
        libc::setenv(c"EVENT_NOEPOLL".as_ptr(), c"1".as_ptr(), 1);

        let base = event_init();
        libc::unsetenv(c"EVENT_NOEPOLL".as_ptr());
        base
    }
}
