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

use libc::{EOF, TIOCGSID, fgetc, ioctl, readlink, tcgetpgrp};

use crate::*;

// this is for osdep-linux.c

pub unsafe extern "C" fn osdep_get_name(fd: i32, tty: *const c_char) -> *mut c_char {
    unsafe {
        let pgrp = tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        let mut path = format_nul!("/proc/{pgrp}/cmdline");
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
            if ch == b'\0' as i32 {
                break;
            }
            buf = xrealloc_(buf, len + 2).as_ptr();
            *buf.add(len) = ch as c_char;
            len += 1;
        }
        if !buf.is_null() {
            *buf.add(len) = b'\0' as c_char;
        }

        fclose(f);
        buf
    }
}

pub unsafe extern "C" fn osdep_get_cwd(fd: i32) -> *const c_char {
    const MAXPATHLEN: usize = libc::PATH_MAX as usize;
    static mut target_buffer: [c_char; MAXPATHLEN + 1] = [0; MAXPATHLEN + 1];
    unsafe {
        let target = &raw mut target_buffer as *mut c_char;

        let pgrp = tcgetpgrp(fd);
        if pgrp == -1 {
            return null_mut();
        }

        let mut path = format_nul!("/proc/{pgrp}/cwd");
        let mut n = libc::readlink(path, target, MAXPATHLEN);
        free_(path);

        let mut sid: pid_t = 0;
        if n == -1 && ioctl(fd, TIOCGSID, &raw mut sid) != -1 {
            path = format_nul!("/proc/{sid}/cwd");
            n = readlink(path, target, MAXPATHLEN);
            free_(path);
        }

        if n > 0 {
            *target.add(n as usize) = b'\0' as c_char;
            return target;
        }
        null_mut()
    }
}

pub unsafe extern "C" fn osdep_event_init() -> *mut event_base {
    unsafe {
        // On Linux, epoll doesn't work on /dev/null (yes, really).
        libc::setenv(c"EVENT_NOEPOLL".as_ptr(), c"1".as_ptr(), 1);

        let base = event_init();
        libc::unsetenv(c"EVENT_NOEPOLL".as_ptr());
        base
    }
}
