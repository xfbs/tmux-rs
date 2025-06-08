use ::libc::{gettimeofday, isalnum, ispunct, memcpy, strchr, strcmp, strcspn, strlen, strncmp};

use crate::event_::{event_add, event_initialized};
use crate::*;

unsafe extern "C" {
    unsafe fn basename(_: *mut c_char) -> *mut c_char;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn name_time_callback(_fd: c_int, _events: c_short, arg: *mut c_void) {
    let mut w = arg as *mut window;
    unsafe {
        log_debug!("@{} timer expired", (*w).id);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn name_time_expired(w: *mut window, tv: *mut timeval) -> c_int {
    unsafe {
        let mut offset: MaybeUninit<timeval> = MaybeUninit::<timeval>::uninit();

        timersub(tv, &raw mut (*w).name_time, offset.as_mut_ptr());
        let offset = offset.assume_init_ref();

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

        if (*w).active.is_null() {
            return;
        }

        if options_get_number((*w).options, c"automatic-rename".as_ptr()) == 0 {
            return;
        }

        if !(*(*w).active)
            .flags
            .intersects(window_pane_flags::PANE_CHANGED)
        {
            log_debug!("@{} pane not changed", (*w).id);
            return;
        }
        log_debug!("@{} pane changed", (*w).id);

        gettimeofday(&raw mut tv, null_mut());
        let left = name_time_expired(w, &raw mut tv);
        if left != 0 {
            if !event_initialized(&raw mut (*w).name_event) {
                evtimer_set(&raw mut (*w).name_event, Some(name_time_callback), w as _);
            }
            if evtimer_pending(&raw mut (*w).name_event, null_mut()) == 0 {
                log_debug!("@{} timer queued ({})", (*w).id, left);
                timerclear(&raw mut next);
                next.tv_usec = left as i64;
                event_add(&raw mut (*w).name_event, &raw const next);
            } else {
                log_debug!("@{} timer already queued ({})", (*w).id, left);
            }
            return;
        }
        memcpy(
            &raw mut (*w).name_time as _,
            &raw const tv as _,
            size_of::<timeval>(),
        );
        if event_initialized(&raw mut (*w).name_event).as_bool() {
            evtimer_del(&raw mut (*w).name_event);
        }

        (*(*w).active).flags &= !window_pane_flags::PANE_CHANGED;

        let name = format_window_name(w);
        if strcmp(name, (*w).name) != 0 {
            log_debug!("@{} name {} (was {})", (*w).id, _s(name), _s((*w).name));
            window_set_name(w, name);
            server_redraw_window_borders(w);
            server_status_window(w);
        } else {
            log_debug!("@{} not changed (still {})", (*w).id, _s((*w).name));
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

unsafe extern "C" fn format_window_name(w: *mut window) -> *const c_char {
    unsafe {
        let ft = format_create(
            null_mut(),
            null_mut(),
            (FORMAT_WINDOW | (*w).id) as i32,
            format_flags::empty(),
        );
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
