use crate::*;
use compat_rs::queue::tailq_foreach;
unsafe extern "C" {
    // pub unsafe fn control_notify_pane_mode_changed(_: c_int);
    // pub unsafe fn control_notify_window_layout_changed(_: *mut window);
    // pub unsafe fn control_notify_window_pane_changed(_: *mut window);
    // pub unsafe fn control_notify_window_unlinked(_: *mut session, _: *mut window);
    // pub unsafe fn control_notify_window_linked(_: *mut session, _: *mut window);
    // pub unsafe fn control_notify_window_renamed(_: *mut window);
    // pub unsafe fn control_notify_client_session_changed(_: *mut client);
    // pub unsafe fn control_notify_client_detached(_: *mut client);
    // pub unsafe fn control_notify_session_renamed(_: *mut session);
    // pub unsafe fn control_notify_session_created(_: *mut session);
    // pub unsafe fn control_notify_session_closed(_: *mut session);
    // pub unsafe fn control_notify_session_window_changed(_: *mut session);
    // pub unsafe fn control_notify_paste_buffer_changed(_: *const c_char);
    // pub unsafe fn control_notify_paste_buffer_deleted(_: *const c_char);
}

macro_rules! CONTROL_SHOULD_NOTIFY_CLIENT {
    ($c:expr) => {
        !$c.is_null() && (*$c).flags.intersects(client_flag::CONTROL)
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_pane_mode_changed(pane: c_int) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write(c, c"%%pane-mode-changed %%%u".as_ptr(), pane);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_window_layout_changed(w: *mut window) {
    let template =
        c"%layout-change #{window_id} #{window_layout} #{window_visible_layout} #{window_raw_flags}".as_ptr();

    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let s = (*c).session;

                if winlink_find_by_window_id(&raw mut (*s).windows, (*w).id).is_null() {
                    continue;
                }

                // When the last pane in a window is closed it won't have a
                // layout root and we don't need to inform the client about the
                // layout change because the whole window will go away soon.
                if (*w).layout_root.is_null() {
                    continue;
                }

                if let Some(wl) = winlink_find_by_window(&raw mut (*s).windows, w) {
                    let cp = format_single(null_mut(), template, c, null_mut(), wl.as_ptr(), null_mut());
                    control_write(c, c"%s".as_ptr(), cp);
                    free_(cp);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_window_pane_changed(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if (!CONTROL_SHOULD_NOTIFY_CLIENT!(c)) {
                    continue;
                }

                control_write(
                    c,
                    c"%%window-pane-changed @%u %%%u".as_ptr(),
                    (*w).id,
                    (*(*w).active).id,
                );
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_window_unlinked(s: *mut session, w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let cs = (*c).session;

                if !winlink_find_by_window_id(&raw mut (*cs).windows, (*w).id).is_null() {
                    control_write(c, c"%%window-close @%u".as_ptr(), (*w).id);
                } else {
                    control_write(c, c"%%unlinked-window-close @%u".as_ptr(), (*w).id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_window_linked(s: *mut session, w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let cs = (*c).session;

                if !winlink_find_by_window_id(&raw mut (*cs).windows, (*w).id).is_null() {
                    control_write(c, c"%%window-add @%u".as_ptr(), (*w).id);
                } else {
                    control_write(c, c"%%unlinked-window-add @%u".as_ptr(), (*w).id);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_window_renamed(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }
                let cs = (*c).session;

                if !winlink_find_by_window_id(&raw mut (*cs).windows, (*w).id).is_null() {
                    control_write(c, c"%%window-renamed @%u %s".as_ptr(), (*w).id, (*w).name);
                } else {
                    control_write(c, c"%%unlinked-window-renamed @%u %s".as_ptr(), (*w).id, (*w).name);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_client_session_changed(cc: *mut client) {
    unsafe {
        if (*cc).session.is_null() {
            return;
        }
        let s = (*cc).session;

        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) || (*c).session.is_null() {
                    continue;
                }

                if cc == c {
                    control_write(c, c"%%session-changed $%u %s".as_ptr(), (*s).id, (*s).name);
                } else {
                    control_write(
                        c,
                        c"%%client-session-changed %s $%u %s".as_ptr(),
                        (*cc).name,
                        (*s).id,
                        (*s).name,
                    );
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_client_detached(cc: *mut client) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    control_write(c, c"%%client-detached %s".as_ptr(), (*cc).name);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_session_renamed(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write(c, c"%%session-renamed $%u %s".as_ptr(), (*s).id, (*s).name);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_session_created(_s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if (!CONTROL_SHOULD_NOTIFY_CLIENT!(c)) {
                    continue;
                }

                control_write(c, c"%%sessions-changed".as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_session_closed(_s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write(c, c"%%sessions-changed".as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_session_window_changed(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write(
                    c,
                    c"%%session-window-changed $%u @%u".as_ptr(),
                    (*s).id,
                    (*(*(*s).curw).window).id,
                );
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_paste_buffer_changed(name: *const c_char) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write(c, c"%%paste-buffer-changed %s".as_ptr(), name);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn control_notify_paste_buffer_deleted(name: *const c_char) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            {
                if !CONTROL_SHOULD_NOTIFY_CLIENT!(c) {
                    continue;
                }

                control_write(c, c"%%paste-buffer-deleted %s".as_ptr(), name);
            }
        }
    }
}
