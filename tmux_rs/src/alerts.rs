#![allow(dead_code)]
use core::ffi::{c_char, c_int, c_short, c_void};

use super::*;

use compat_rs::{
    queue::{tailq_foreach, tailq_foreach_safe, tailq_head, tailq_remove},
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
        log_debug(c"@%u alerts timer expired".as_ptr(), (*w).id);
        alerts_queue(w, WINDOW_SILENCE);
    }
}

pub unsafe extern "C" fn alerts_callback(_fd: c_int, _events: c_short, arg: *mut c_void) {
    unsafe {
        tailq_foreach_safe::<_, _, _, crate::discr_alerts_entry>(&raw mut alerts_list, |w| {
            unsafe {
                let alerts = alerts_check_all(w);

                log_debug(c"@%u alerts check, alerts %#x".as_ptr(), (*w).id, alerts);

                (*w).alerts_queued = 0;
                tailq_remove::<_, crate::discr_alerts_entry>(&raw mut alerts_list, w);

                (*w).flags &= !WINDOW_ALERTFLAGS;
                window_remove_ref(w, c"alerts_callback".as_ptr());
            }

            ControlFlow::Continue::<(), ()>(())
        });
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

pub unsafe fn alerts_check_all(w: *mut window) -> c_int {
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
        rb_foreach(&raw mut (*s).windows, |wl| {
            alerts_check_all((*wl).window);
            ControlFlow::Continue::<(), ()>(())
        });
    }
}

pub unsafe fn alerts_enabled(w: *mut window, flags: c_int) -> c_int {
    unsafe {
        if flags & WINDOW_BELL != 0 {
            if options_get_number((*w).options, c"monitor-bell".as_ptr()) != 0 {
                return 1;
            }
        }
        if flags & WINDOW_ACTIVITY != 0 {
            if options_get_number((*w).options, c"monitor-activity".as_ptr()) != 0 {
                return 1;
            }
        }
        if flags & WINDOW_SILENCE != 0 {
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
        rb_foreach(&raw mut windows, |w| {
            alerts_reset(w);
            ControlFlow::Continue::<(), ()>(())
        });
    }
}

#[unsafe(no_mangle)]
unsafe fn alerts_reset(w: *mut window) {
    unsafe {
        if event_initialized(&raw const (*w).alerts_timer) == 0 {
            evtimer_set(&raw mut (*w).alerts_timer, Some(alerts_timer), w as _);
        }

        (*w).flags &= !WINDOW_SILENCE;
        event_del(&raw mut (*w).alerts_timer);

        let mut tv = timeval {
            tv_sec: options_get_number((*w).options, c"monitor-silence".as_ptr()),
            tv_usec: 0,
        };

        log_debug(c"@%u alerts timer reset %u".as_ptr(), (*w).id, tv.tv_sec as u32);
        if tv.tv_sec != 0 {
            event_add(&raw mut (*w).alerts_timer, &raw mut tv);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_queue(w: *mut window, flags: c_int) {
    unsafe {
        alerts_reset(w);

        if ((*w).flags & flags) != flags {
            (*w).flags |= flags;
            log_debug(c"@%u alerts flags added %#x".as_ptr(), (*w).id, flags);
        }

        if alerts_enabled(w, flags) != 0 {
            if (*w).alerts_queued == 0 {
                (*w).alerts_queued = 1;
                compat_rs::queue::tailq_insert_tail::<_, discr_alerts_entry>(&raw mut alerts_list, w);
                window_add_ref(w, c"alerts_queue".as_ptr());
            }

            if alerts_fired == 0 {
                log_debug(c"alerts check queued (by @%u)".as_ptr(), (*w).id);
                event_once(-1, EV_TIMEOUT as i16, Some(alerts_callback), null_mut(), null_mut());
                alerts_fired = 1;
            }
        }
    }
}

unsafe fn alerts_check_bell(w: *mut window) -> c_int {
    unsafe {
        if !(*w).flags & WINDOW_BELL != 0 {
            return 0;
        }
        if options_get_number((*w).options, c"monitor-bell".as_ptr()) == 0 {
            return 0;
        }

        tailq_foreach::<_, _, _, crate::discr_wentry>(&raw mut (*w).winlinks, |wl| {
            (*(*wl).session).flags &= !SESSION_ALERTED;
            ControlFlow::<(), ()>::Continue(())
        });

        tailq_foreach::<_, _, _, crate::discr_wentry>(&raw mut (*w).winlinks, |wl| {
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
                return ControlFlow::<(), ()>::Continue(());
            }
            notify_winlink(c"alert-bell".as_ptr(), wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                return ControlFlow::<(), ()>::Continue(());
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Bell".as_ptr(), c"visual-bell".as_ptr());
            ControlFlow::<(), ()>::Continue(())
        });
    }
    WINDOW_BELL
}

unsafe fn alerts_check_activity(w: *mut window) -> c_int {
    unsafe {
        if !(*w).flags & WINDOW_ACTIVITY != 0 {
            return 0;
        }
        if options_get_number((*w).options, c"monitor-activity".as_ptr()) == 0 {
            return 0;
        }

        tailq_foreach::<_, _, _, crate::discr_wentry>(&raw mut (*w).winlinks, |wl| {
            (*(*wl).session).flags &= !SESSION_ALERTED;
            ControlFlow::<(), ()>::Continue(())
        });

        tailq_foreach::<_, _, _, crate::discr_wentry>(&raw mut (*w).winlinks, |wl| {
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= WINLINK_ACTIVITY;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"activity-action".as_ptr()) == 0 {
                return ControlFlow::<(), ()>::Continue(());
            }
            notify_winlink(c"alert-activity".as_ptr(), wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                return ControlFlow::<(), ()>::Continue(());
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Activity".as_ptr(), c"visual-activity".as_ptr());
            ControlFlow::<(), ()>::Continue(())
        });
    }
    WINDOW_ACTIVITY
}

unsafe fn alerts_check_silence(w: *mut window) -> c_int {
    unsafe {
        if !(*w).flags & WINDOW_SILENCE != 0 {
            return 0;
        }
        if options_get_number((*w).options, c"monitor-silence".as_ptr()) == 0 {
            return 0;
        }

        tailq_foreach::<_, _, _, crate::discr_wentry>(&raw mut (*w).winlinks, |wl| {
            (*(*wl).session).flags &= !SESSION_ALERTED;
            ControlFlow::Continue::<(), ()>(())
        });

        tailq_foreach::<_, _, _, crate::discr_wentry>(&raw mut (*w).winlinks, |wl| {
            if (*wl).flags & WINLINK_SILENCE != 0 {
                return ControlFlow::Continue::<(), ()>(());
            }
            let s = (*wl).session;
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= WINLINK_SILENCE;
                server_status_session(s);
            }
            if alerts_action_applies(wl, c"silence-action".as_ptr()) == 0 {
                return ControlFlow::Continue::<(), ()>(());
            }
            notify_winlink(c"alert-silence".as_ptr(), wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                return ControlFlow::Continue::<(), ()>(());
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(wl, c"Silence".as_ptr(), c"visual-silence".as_ptr());
            ControlFlow::Continue::<(), ()>(())
        });
    }

    WINDOW_SILENCE
}

unsafe fn alerts_set_message(wl: *mut winlink, type_: *const c_char, option: *const c_char) {
    unsafe {
        let visual: i32 = options_get_number((*(*wl).session).options, option) as i32;

        tailq_foreach(&raw mut clients, |c| {
            if (*c).session != (*wl).session || (*c).flags & CLIENT_CONTROL != 0 {
                return ControlFlow::Continue::<(), ()>(());
            }

            if visual == VISUAL_OFF || visual == VISUAL_BOTH {
                tty_putcode(&raw mut (*c).tty, tty_code_code::TTYC_BEL);
            }
            if visual == VISUAL_OFF {
                return ControlFlow::Continue(());
            }
            if (*(*c).session).curw == wl {
                status_message_set(c, -1, 1, 0, c"%s in current window".as_ptr(), type_);
            } else {
                status_message_set(c, -1, 1, 0, c"%s in window %d".as_ptr(), type_, (*wl).idx);
            }
            ControlFlow::Continue(())
        });
    }
}
