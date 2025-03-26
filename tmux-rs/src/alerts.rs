#![allow(dead_code)]
use core::ffi::{c_char, c_int, c_short, c_void};

use super::*;

use compat_rs::{
    queue::{tailq_foreach, tailq_head, tailq_remove},
    tree::rb_foreach,
};

static mut alerts_fired: i32 = 0;

static mut alerts_list: tailq_head<window> = const {
    tailq_head {
        tqh_first: null_mut(),
        tqh_last: unsafe { &raw mut alerts_list.tqh_first },
    }
};

unsafe extern "C" fn alerts_timer(_fd: i32, _events: i16, arg: *mut c_void) {
    let w = arg as *mut window;

    unsafe {
        log_debug!("@{} alerts timer expired", (*w).id);
        alerts_queue(NonNull::new_unchecked(w), window_flag::SILENCE);
    }
}

pub unsafe extern "C" fn alerts_callback(_fd: c_int, _events: c_short, arg: *mut c_void) {
    unsafe {
        for w in tailq_foreach::<_, crate::discr_alerts_entry>(&raw mut alerts_list).map(NonNull::as_ptr) {
            unsafe {
                let alerts = alerts_check_all(w);

                log_debug!("@{} alerts check, alerts {:#x}", (*w).id, alerts);

                (*w).alerts_queued = 0;
                tailq_remove::<_, crate::discr_alerts_entry>(&raw mut alerts_list, w);

                (*w).flags &= !WINDOW_ALERTFLAGS;
                window_remove_ref(w, c"alerts_callback".as_ptr());
            }
        }
        alerts_fired = 0;
    }
}

pub unsafe fn alerts_action_applies(wl: *mut winlink, name: *const c_char) -> c_int {
    unsafe {
        let action: i32 = options_get_number((*(*wl).session).options, name) as i32;
        match action {
            ALERT_ANY => 1,
            ALERT_CURRENT => (wl == (*(*wl).session).curw) as i32,
            ALERT_OTHER => (wl != (*(*wl).session).curw) as i32,
            _ => 0,
        }
    }
}

pub unsafe fn alerts_check_all(w: *mut window) -> window_flag {
    unsafe {
        let mut alerts = alerts_check_bell(w);
        alerts |= alerts_check_activity(w);
        alerts |= alerts_check_silence(w);
        alerts
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_check_session(s: *mut session) {
    unsafe {
        for wl in rb_foreach(&raw mut (*s).windows) {
            alerts_check_all((*wl.as_ptr()).window);
        }
    }
}

pub unsafe fn alerts_enabled(w: *mut window, flags: window_flag) -> c_int {
    unsafe {
        if flags.intersects(window_flag::BELL) {
            if options_get_number((*w).options, c"monitor-bell".as_ptr()) != 0 {
                return 1;
            }
        }
        if flags.intersects(window_flag::ACTIVITY) {
            if options_get_number((*w).options, c"monitor-activity".as_ptr()) != 0 {
                return 1;
            }
        }
        if flags.intersects(window_flag::SILENCE) {
            if options_get_number((*w).options, c"monitor-silence".as_ptr()) != 0 {
                return 1;
            }
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_reset_all() {
    unsafe {
        for w in rb_foreach(&raw mut windows) {
            alerts_reset(w);
        }
    }
}

#[unsafe(no_mangle)]
unsafe fn alerts_reset(w: NonNull<window>) {
    let w = w.as_ptr();
    unsafe {
        if event_initialized(&raw const (*w).alerts_timer) == 0 {
            evtimer_set(&raw mut (*w).alerts_timer, Some(alerts_timer), w as _);
        }

        (*w).flags &= !window_flag::SILENCE;
        event_del(&raw mut (*w).alerts_timer);

        let mut tv = timeval {
            tv_sec: options_get_number((*w).options, c"monitor-silence".as_ptr()),
            tv_usec: 0,
        };

        log_debug!("@{} alerts timer reset {}", (*w).id, tv.tv_sec);
        if tv.tv_sec != 0 {
            event_add(&raw mut (*w).alerts_timer, &raw mut tv);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_queue(w: NonNull<window>, flags: window_flag) {
    unsafe {
        alerts_reset(w);
        let w = w.as_ptr();

        if ((*w).flags & flags) != flags {
            (*w).flags |= flags;
            log_debug!("@{} alerts flags added {:#x}", (*w).id, flags);
        }

        if alerts_enabled(w, flags) != 0 {
            if (*w).alerts_queued == 0 {
                (*w).alerts_queued = 1;
                compat_rs::queue::tailq_insert_tail::<_, discr_alerts_entry>(&raw mut alerts_list, w);
                window_add_ref(w, c"alerts_queue".as_ptr());
            }

            if alerts_fired == 0 {
                log_debug!("alerts check queued (by @{})", (*w).id);
                event_once(-1, EV_TIMEOUT as i16, Some(alerts_callback), null_mut(), null_mut());
                alerts_fired = 1;
            }
        }
    }
}

unsafe fn alerts_check_bell(w: *mut window) -> window_flag {
    unsafe {
        if !(*w).flags.intersects(window_flag::BELL) {
            return window_flag::empty();
        }
        if options_get_number((*w).options, c"monitor-bell".as_ptr()) == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks) {
            (*(*wl.as_ptr()).session).flags &= !SESSION_ALERTED;
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            /*
             * Bells are allowed even if there is an existing bell (so do
             * not check WINLINK_BELL).
             */
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= WINLINK_BELL;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"bell-action".as_ptr()) == 0 {
                continue;
            }
            notify_winlink(c"alert-bell".as_ptr(), wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Bell".as_ptr(), c"visual-bell".as_ptr());
        }
    }
    window_flag::BELL
}

unsafe fn alerts_check_activity(w: *mut window) -> window_flag {
    unsafe {
        if !(*w).flags.intersects(window_flag::ACTIVITY) {
            return window_flag::empty();
        }
        if options_get_number((*w).options, c"monitor-activity".as_ptr()) == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            (*(*wl).session).flags &= !SESSION_ALERTED;
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= WINLINK_ACTIVITY;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"activity-action".as_ptr()) == 0 {
                continue;
            }
            notify_winlink(c"alert-activity".as_ptr(), wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Activity".as_ptr(), c"visual-activity".as_ptr());
        }
    }
    window_flag::ACTIVITY
}

unsafe fn alerts_check_silence(w: *mut window) -> window_flag {
    unsafe {
        if !(*w).flags.intersects(window_flag::SILENCE) {
            return window_flag::empty();
        }
        if options_get_number((*w).options, c"monitor-silence".as_ptr()) == 0 {
            return window_flag::empty();
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            (*(*wl).session).flags &= !SESSION_ALERTED;
        }

        for wl in tailq_foreach::<_, crate::discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            if (*wl).flags & WINLINK_SILENCE != 0 {
                continue;
            }
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= WINLINK_SILENCE;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"silence-action".as_ptr()) == 0 {
                continue;
            }
            notify_winlink(c"alert-silence".as_ptr(), wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Silence".as_ptr(), c"visual-silence".as_ptr());
        }
    }

    window_flag::SILENCE
}

unsafe fn alerts_set_message(wl: *mut winlink, type_: *const c_char, option: *const c_char) {
    unsafe {
        let visual: i32 = options_get_number((*(*wl).session).options, option) as i32;

        for c in compat_rs::queue::tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if (*c).session != (*wl).session || (*c).flags.intersects(client_flag::CONTROL) {
                continue;
            }

            if visual == VISUAL_OFF || visual == VISUAL_BOTH {
                tty_putcode(&raw mut (*c).tty, tty_code_code::TTYC_BEL);
            }
            if visual == VISUAL_OFF {
                continue;
            }
            if (*(*c).session).curw == wl {
                status_message_set(c, -1, 1, 0, c"%s in current window".as_ptr(), type_);
            } else {
                status_message_set(c, -1, 1, 0, c"%s in window %d".as_ptr(), type_, (*wl).idx);
            }
        }
    }
}
