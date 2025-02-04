use libc::{gettimeofday, isalnum, ispunct, memcpy, strchr, strcmp, strcspn, strlen, strncmp};
use libevent_sys::{event_add, event_initialized};

use super::*;
unsafe extern "C" {
    unsafe fn basename(_: *mut c_char) -> *mut c_char;
}

#[inline]
unsafe fn timerclear(tv: *mut timeval) {
    unsafe {
        (*tv).tv_sec = 0;
        (*tv).tv_usec = 0;
    }
}

#[inline]
unsafe fn timersub(a: *mut timeval, b: *mut timeval, result: *mut timeval) {
    // implemented as a macro by most libc's
    unsafe {
        (*result).tv_sec = (*a).tv_sec - (*b).tv_sec;
        (*result).tv_usec = (*a).tv_usec - (*b).tv_usec;
        if (*result).tv_usec < 0 {
            (*result).tv_sec -= 1;
            (*result).tv_usec += 1000000;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn name_time_callback(_fd: c_int, _events: c_short, arg: *mut c_void) {
    let mut w = arg as *mut window;
    unsafe {
        log_debug(c"@%u name timer expired".as_ptr(), (*w).id);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn name_time_expired(w: *mut window, tv: *mut timeval) -> c_int {
    unsafe {
        let mut offset: timeval = zeroed();
        timersub(tv, &raw mut (*w).name_time, &raw mut offset);

        if offset.tv_sec != 0 || offset.tv_usec > NAME_INTERVAL as i64 {
            0
        } else {
            NAME_INTERVAL - offset.tv_usec as c_int
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe fn check_window_name(w: *mut window) {
    unsafe {
        let mut tv: timeval = zeroed();
        let mut next: timeval = zeroed();
        let mut left = 0;

        if (*w).active.is_null() {
            return;
        }

        if options_get_number((*w).options, c"automatic-rename".as_ptr()) == 0 {
            return;
        }

        if !(*(*w).active).flags & PANE_CHANGED != 0 {
            log_debug(c"@%u active pane not changed".as_ptr(), (*w).id);
            return;
        }
        log_debug(c"@%u active pane changed".as_ptr(), (*w).id);

        gettimeofday(&raw mut tv, null_mut());
        let left = name_time_expired(w, &raw mut tv);
        if left != 0 {
            if event_initialized(&raw mut (*w).name_event) == 0 {
                evtimer_set(&raw mut (*w).name_event, Some(name_time_callback), w as _);
            }
            if evtimer_pending(&raw mut (*w).name_event, null_mut()) == 0 {
                log_debug(c"@%u name timer queued (%d left)".as_ptr(), (*w).id, left);
                timerclear(&raw mut next);
                next.tv_usec = left as i64;
                event_add(
                    &raw mut (*w).name_event,
                    core::mem::transmute::<*const libc::timeval, *const libevent_sys::timeval>(&raw const next),
                );
            } else {
                log_debug(c"@%u name timer already queued (%d left)".as_ptr(), (*w).id, left);
            }
            return;
        }
        memcpy(&raw mut (*w).name_time as _, &raw const tv as _, size_of::<timeval>());
        if event_initialized(&raw mut (*w).name_event) != 0 {
            evtimer_del(&raw mut (*w).name_event);
        }

        (*(*w).active).flags &= !PANE_CHANGED;

        let name = format_window_name(w);
        if strcmp(name, (*w).name) != 0 {
            log_debug(c"@%u new name %s (was %s)".as_ptr(), (*w).id, name, (*w).name);
            window_set_name(w, name);
            server_redraw_window_borders(w);
            server_status_window(w);
        } else {
            log_debug(c"@%u name not changed (still %s)".as_ptr(), (*w).id, (*w).name);
        }

        free(name as _);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn default_window_name(w: *mut window) -> *mut c_char {
    unsafe {
        if (*w).active.is_null() {
            return xstrdup(c"".as_ptr()).cast().as_ptr();
        }

        let cmd = cmd_stringify_argv((*(*w).active).argc, (*(*w).active).argv);
        let s = if !cmd.is_null() && *cmd != b'\0' as _ {
            parse_window_name(cmd)
        } else {
            parse_window_name((*(*w).active).shell)
        };
        free(cmd as _);
        s
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn format_window_name(w: *mut window) -> *const c_char {
    unsafe {
        let ft = format_create(null_mut(), null_mut(), (FORMAT_WINDOW | (*w).id) as i32, 0);
        format_defaults_window(ft, w);
        format_defaults_pane(ft, (*w).active);

        let fmt = options_get_string((*w).options, c"automatic-rename-format".as_ptr());
        let name = format_expand(ft, fmt);

        format_free(ft);
        name
    }
}

#[unsafe(no_mangle)]
pub unsafe fn parse_window_name(in_: *const c_char) -> *mut c_char {
    unsafe {
        let sizeof_exec: usize = 6; // sizeof "exec "
        let copy: *mut c_char = xstrdup(in_).cast().as_ptr();
        let mut name = copy;
        if *name == b'"' as _ {
            name = name.wrapping_add(1);
        }
        *name.add(strcspn(name, c"\"".as_ptr())) = b'\0' as c_char;

        if strncmp(name, c"exec ".as_ptr(), sizeof_exec - 1) == 0 {
            name = name.wrapping_add(sizeof_exec - 1);
        }

        while *name == b' ' as c_char || *name == b'-' as c_char {
            name = name.wrapping_add(1);
        }

        let mut ptr = strchr(name, b' ' as _);
        if !ptr.is_null() {
            *ptr = b'\0' as _;
        }

        if *name != b'\0' as c_char {
            ptr = name.add(strlen(name) - 1);
            while ptr > name && isalnum(*ptr as _) == 0 && ispunct(*ptr as _) == 0 {
                *ptr = b'\0' as c_char;
                *ptr -= 1;
            }
        }

        if *name == b'/' as c_char {
            name = basename(name);
        }
        name = xstrdup(name).cast().as_ptr();
        free(copy as _);
        name
    }
}
