// Copyright (c) 2015 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use std::time::Duration;

use crate::*;
use crate::options_::{options_get_number___};

/// Alert option values
#[repr(i32)]
#[derive(Copy, Clone, num_enum::TryFromPrimitive)]
pub(crate) enum alert_option {
    ALERT_NONE,
    ALERT_ANY,
    ALERT_CURRENT,
    ALERT_OTHER,
}

static ALERTS_FIRED: atomic::AtomicI32 = atomic::AtomicI32::new(0);

thread_local! {
    static ALERTS_LIST: RefCell<LinkedList<NonNull<window>>> = const { RefCell::new(LinkedList::new()) };
}

/// Alerts timer callback: fires when silence timeout expires.
unsafe fn alerts_timer_fire(wid: WindowId) {
    unsafe {
        let Some(w) = window_from_id(wid) else { return };
        log_debug!("@{} alerts timer expired", (*w).id);
        alerts_queue(NonNull::new_unchecked(w), window_flag::SILENCE);
    }
}

unsafe fn alerts_callback() {
    unsafe {
        ALERTS_LIST.with_borrow_mut(|alerts_list| {
            while let Some(w) = alerts_list.pop_front() {
                let alerts = alerts_check_all(&*w.as_ptr());

                let w = w.as_ptr();
                log_debug!("@{} alerts check, alerts {:#x}", (*w).id, alerts);

                (*w).alerts_queued = 0;

                (*w).flags &= !WINDOW_ALERTFLAGS;
                window_remove_ref(w, c!("alerts_callback"));
            }
        });
        ALERTS_FIRED.store(0, atomic::Ordering::Release);
    }
}

fn alerts_action_applies(wl: &winlink, name: &str) -> bool {
    unsafe {
        let s = wl.session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
        if s.is_null() {
            return false;
        }
        match alert_option::try_from(options_get_number___::<i32>(&*(*s).options, name)) {
            Ok(alert_option::ALERT_ANY) => true,
            Ok(alert_option::ALERT_CURRENT) => std::ptr::eq(wl, (*s).curw),
            Ok(alert_option::ALERT_OTHER) => !std::ptr::eq(wl, (*s).curw),
            _ => false,
        }
    }
}

fn alerts_check_all(w: &window) -> window_flag {
    alerts_check_bell(w) | alerts_check_activity(w) | alerts_check_silence(w)
}

pub(crate) fn alerts_check_session(s: &session) {
    unsafe {
        for &wl in s.windows.values() {
            let w = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            if !w.is_null() { alerts_check_all(&*w); }
        }
    }
}

fn alerts_enabled(w: &window, flags: window_flag) -> bool {
    unsafe {
        if flags.intersects(window_flag::BELL)
            && options_get_number___::<i64>(&*w.options, "monitor-bell") != 0
        {
            return true;
        }
        if flags.intersects(window_flag::ACTIVITY)
            && options_get_number___::<i64>(&*w.options, "monitor-activity") != 0
        {
            return true;
        }
        if flags.intersects(window_flag::SILENCE)
            && options_get_number___::<i64>(&*w.options, "monitor-silence") != 0
        {
            return true;
        }
    }

    false
}

pub(crate) unsafe fn alerts_reset_all() {
    unsafe {
        for w in windows_iter() {
            alerts_reset(NonNull::new(w).unwrap());
        }
    }
}

unsafe fn alerts_reset(w: NonNull<window>) {
    unsafe {
        let w = w.as_ptr();
        (*w).flags &= !window_flag::SILENCE;

        // Cancel any existing timer.
        (*w).alerts_timer = None;

        let silence: i64 = options_get_number___(&*(*w).options, "monitor-silence");
        log_debug!("@{} alerts timer reset {}", (*w).id, silence);
        if silence != 0 {
            let wid = WindowId((*w).id);
            (*w).alerts_timer = timer_add(
                Duration::from_secs(silence as u64),
                Box::new(move || alerts_timer_fire(wid)),
            );
        }
    }
}

pub(crate) unsafe fn alerts_queue(w: NonNull<window>, flags: window_flag) {
    unsafe {
        alerts_reset(w);
        let w = w.as_ptr();

        if ((*w).flags & flags) != flags {
            (*w).flags |= flags;
            log_debug!("@{} alerts flags added {:#x}", (*w).id, flags);
        }

        if alerts_enabled(&*w, flags) {
            if (*w).alerts_queued == 0 {
                (*w).alerts_queued = 1;
                ALERTS_LIST.with_borrow_mut(|alerts_list| {
                    alerts_list.push_back(NonNull::new_unchecked(w));
                });
                window_add_ref(w, c!("alerts_queue"));
            }

            if ALERTS_FIRED.load(atomic::Ordering::Acquire) == 0 {
                log_debug!("alerts check queued (by @{})", (*w).id);
                defer(Box::new(|| alerts_callback()));
                ALERTS_FIRED.store(1, atomic::Ordering::Release);
            }
        }
    }
}

fn alerts_check_bell(w: &window) -> window_flag {
    unsafe {
        if !w.flags.intersects(window_flag::BELL) {
            return window_flag::empty();
        }
        if options_get_number___::<i64>(&*w.options, "monitor-bell") == 0 {
            return window_flag::empty();
        }

        for &wl in &w.winlinks {
            let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
            if !s.is_null() { (*s).flags &= !SESSION_ALERTED; }
        }

        for &wl in &w.winlinks {
            // Bells are allowed even if there is an existing bell (so do
            // not check WINLINK_BELL).
            let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
            if s.is_null() { continue; }
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_BELL;
                server_status_session(s);
            }
            if !alerts_action_applies(&*wl, "bell-action") {
                continue;
            }
            notify_winlink(c"alert-bell", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(&*wl, "Bell", "visual-bell");
        }
    }
    window_flag::BELL
}

fn alerts_check_activity(w: &window) -> window_flag {
    unsafe {
        if !w.flags.intersects(window_flag::ACTIVITY) {
            return window_flag::empty();
        }
        if options_get_number___::<i64>(&*w.options, "monitor-activity") == 0 {
            return window_flag::empty();
        }

        for &wl in &w.winlinks {
            let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
            if !s.is_null() { (*s).flags &= !SESSION_ALERTED; }
        }

        for &wl in &w.winlinks {
            let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
            if s.is_null() { continue; }
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_ACTIVITY;
                server_status_session(s);
            }
            if !alerts_action_applies(&*wl, "activity-action") {
                continue;
            }
            notify_winlink(c"alert-activity", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(&*wl, "Activity", "visual-activity");
        }
    }
    window_flag::ACTIVITY
}

fn alerts_check_silence(w: &window) -> window_flag {
    unsafe {
        if !w.flags.intersects(window_flag::SILENCE) {
            return window_flag::empty();
        }
        if options_get_number___::<i64>(&*w.options, "monitor-silence") == 0 {
            return window_flag::empty();
        }

        for &wl in &w.winlinks {
            let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
            if !s.is_null() { (*s).flags &= !SESSION_ALERTED; }
        }

        for &wl in &w.winlinks {
            if (*wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
                continue;
            }
            let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
            if s.is_null() { continue; }
            if (*s).curw != wl || (*s).attached == 0 {
                (*wl).flags |= winlink_flags::WINLINK_SILENCE;
                server_status_session(s);
            }
            if !alerts_action_applies(&*wl, "silence-action") {
                continue;
            }
            notify_winlink(c"alert-silence", wl);

            if (*s).flags & SESSION_ALERTED != 0 {
                continue;
            }
            (*s).flags |= SESSION_ALERTED;

            alerts_set_message(&*wl, "Silence", "visual-silence");
        }
    }

    window_flag::SILENCE
}

fn alerts_set_message(wl: &winlink, type_: &str, option: &str) {
    unsafe {
        let s = wl.session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
        if s.is_null() { return; }
        let visual =
            visual_option::try_from(options_get_number___::<i32>(&*(*s).options, option));

        for c in clients_iter() {
            if client_get_session(c) != s || (*c).flags.intersects(client_flag::CONTROL) {
                continue;
            }

            if matches!(
                visual,
                Ok(visual_option::VISUAL_OFF) | Ok(visual_option::VISUAL_BOTH)
            ) {
                tty_putcode(&raw mut (*c).tty, tty_code_code::TTYC_BEL);
            }
            if matches!(visual, Ok(visual_option::VISUAL_OFF)) {
                continue;
            }
            if std::ptr::eq((*client_get_session(c)).curw, wl) {
                status_message_set!(c, -1, 1, false, "{type_} in current window",);
            } else {
                status_message_set!(c, -1, 1, false, "{type_} in window {}", wl.idx);
            }
        }
    }
}
