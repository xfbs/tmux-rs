use compat_rs::{
    queue::{tailq_first, tailq_foreach},
    strlcat, strtonum,
    tree::{rb_foreach, rb_max, rb_min},
};
use libc::{fnmatch, strchr, strcmp, strncmp};

use crate::*;

unsafe extern "C" {
    // pub fn cmd_find_target(
    //     _: *mut cmd_find_state,
    //     _: *mut cmdq_item,
    //     _: *const c_char,
    //     _: cmd_find_type,
    //     _: c_int,
    // ) -> c_int;
    // pub fn cmd_find_best_client(_: *mut session) -> *mut client;
    // pub fn cmd_find_client(_: *mut cmdq_item, _: *const c_char, _: c_int) -> *mut client;
    // pub fn cmd_find_clear_state(_: *mut cmd_find_state, _: c_int);
    // pub fn cmd_find_empty_state(_: *mut cmd_find_state) -> c_int;
    // pub fn cmd_find_valid_state(_: *mut cmd_find_state) -> c_int;
    // pub fn cmd_find_copy_state(_: *mut cmd_find_state, _: *mut cmd_find_state);
    // pub fn cmd_find_from_session(_: *mut cmd_find_state, _: *mut session, _: c_int);
    // pub fn cmd_find_from_winlink(_: *mut cmd_find_state, _: *mut winlink, _: c_int);
    // pub fn cmd_find_from_session_window(_: *mut cmd_find_state, _: *mut session, _: *mut window, _: c_int) -> c_int;
    // pub fn cmd_find_from_window(_: *mut cmd_find_state, _: *mut window, _: c_int) -> c_int;
    // pub fn cmd_find_from_winlink_pane(_: *mut cmd_find_state, _: *mut winlink, _: *mut window_pane, _: c_int);
    // pub fn cmd_find_from_pane(_: *mut cmd_find_state, _: *mut window_pane, _: c_int) -> c_int;
    // pub fn cmd_find_from_client(_: *mut cmd_find_state, _: *mut client, _: c_int) -> c_int;
    // pub fn cmd_find_from_mouse(_: *mut cmd_find_state, _: *mut mouse_event, _: c_int) -> c_int;
    // pub fn cmd_find_from_nothing(_: *mut cmd_find_state, _: c_int) -> c_int;

    // pub unsafe fn cmd_find_best_winlink_with_window(fs: *mut cmd_find_state) -> i32;
    // pub unsafe fn cmd_find_get_window(fs: *mut cmd_find_state, window: *const c_char, only: i32) -> i32;
    // pub unsafe fn cmd_find_get_window_with_session(fs: *mut cmd_find_state, window: *const c_char) -> i32;
}

#[unsafe(no_mangle)]
static mut cmd_find_session_table: [[*const c_char; 2]; 1] = [[null_mut(), null_mut()]];

#[unsafe(no_mangle)]
static mut cmd_find_window_table: [[*const c_char; 2]; 6] = [
    [c"{start}".as_ptr(), c"^".as_ptr()],
    [c"{last}".as_ptr(), c"!".as_ptr()],
    [c"{end}".as_ptr(), c"$".as_ptr()],
    [c"{next}".as_ptr(), c"+".as_ptr()],
    [c"{previous}".as_ptr(), c"-".as_ptr()],
    [null(), null()],
];

#[unsafe(no_mangle)]
static mut cmd_find_pane_table: [[*const c_char; 2]; 16] = [
    [c"{last}".as_ptr(), c"!".as_ptr()],
    [c"{next}".as_ptr(), c"+".as_ptr()],
    [c"{previous}".as_ptr(), c"-".as_ptr()],
    [c"{top}".as_ptr(), c"top".as_ptr()],
    [c"{bottom}".as_ptr(), c"bottom".as_ptr()],
    [c"{left}".as_ptr(), c"left".as_ptr()],
    [c"{right}".as_ptr(), c"right".as_ptr()],
    [c"{top-left}".as_ptr(), c"top-left".as_ptr()],
    [c"{top-right}".as_ptr(), c"top-right".as_ptr()],
    [c"{bottom-left}".as_ptr(), c"bottom-left".as_ptr()],
    [c"{bottom-right}".as_ptr(), c"bottom-right".as_ptr()],
    [c"{up-of}".as_ptr(), c"{up-of}".as_ptr()],
    [c"{down-of}".as_ptr(), c"{down-of}".as_ptr()],
    [c"{left-of}".as_ptr(), c"{left-of}".as_ptr()],
    [c"{right-of}.as_ptr()".as_ptr(), c"{right-of}".as_ptr()],
    [null(), null()],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_inside_pane(c: *mut client) -> *mut window_pane {
    let __func__ = c"cmd_find_inside_pane".as_ptr();
    unsafe {
        if (c.is_null()) {
            return null_mut();
        }

        let mut wp: *mut window_pane = null_mut();
        rb_foreach(&raw mut all_window_panes, |wp_| {
            wp = wp_;
            if ((*wp).fd != -1 && strcmp((*wp).tty.as_ptr(), (*c).ttyname) == 0) {
                return ControlFlow::<(), ()>::Break(());
            }
            ControlFlow::<(), ()>::Continue(())
        });

        if (wp.is_null()) {
            let envent = environ_find((*c).environ, c"TMUX_PANE".as_ptr());
            if (!envent.is_null()) {
                wp = window_pane_find_by_id_str(transmute_ptr((*envent).value));
            }
        }
        if (!wp.is_null()) {
            log_debug(c"%s: got pane %%%u (%s)".as_ptr(), __func__, (*wp).id, (*wp).tty);
        }
        wp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_client_better(c: *mut client, than: *mut client) -> i32 {
    if (than.is_null()) {
        return 1;
    }
    unsafe { timer::new(&raw const (*c).activity_time).cmp(&timer::new(&raw const (*than).activity_time)) as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_best_client(mut s: *mut session) -> *mut client {
    unsafe {
        if ((*s).attached == 0) {
            s = null_mut();
        }

        let mut c = null_mut();
        tailq_foreach(&raw mut clients, |c_loop| {
            if ((*c_loop).session.is_null()) {
                return ControlFlow::<(), ()>::Continue(());
            }
            if (!s.is_null() && (*c_loop).session != s) {
                return ControlFlow::<(), ()>::Continue(());
            }
            if (cmd_find_client_better(c_loop, c) != 0) {
                c = c_loop;
            }
            ControlFlow::<(), ()>::Continue(())
        });

        c
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_session_better(s: *mut session, than: *mut session, flags: i32) -> i32 {
    if (than.is_null()) {
        return 1;
    }

    unsafe {
        if (flags & CMD_FIND_PREFER_UNATTACHED != 0) {
            let attached = ((*than).attached != 0);
            if (attached && (*s).attached == 0) {
                return 1;
            } else if (!attached && (*s).attached != 0) {
                return 0;
            }
        }
        (timer::new(&raw const (*s).activity_time) > timer::new(&raw const (*than).activity_time)) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_best_session(slist: *mut *mut session, ssize: u32, flags: i32) -> *mut session {
    unsafe {
        log_debug(c"%s: %u sessions to try".as_ptr(), "cmd_find_best_session", ssize);

        let mut s = null_mut();
        if (!slist.is_null()) {
            for i in 0..ssize {
                if (cmd_find_session_better(*slist.add(i as usize), s, flags) != 0) {
                    s = *slist.add(i as usize);
                }
            }
        } else {
            rb_foreach(&raw mut sessions, |s_loop| {
                if (cmd_find_session_better(s_loop, s, flags) != 0) {
                    s = s_loop;
                }
                ControlFlow::<(), ()>::Continue(())
            });
        }

        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_best_session_with_window(fs: *mut cmd_find_state) -> i32 {
    let __func__ = c"cmd_find_best_session_with_window".as_ptr();
    unsafe {
        let mut slist: *mut *mut session = null_mut();
        log_debug(c"%s: window is @%u".as_ptr(), __func__, (*(*fs).w).id);

        'fail: {
            let mut ssize: u32 = 0;
            rb_foreach(&raw mut sessions, |s| {
                if (session_has(s, (*fs).w) == 0) {
                    return ControlFlow::<(), ()>::Continue(());
                }
                slist = xreallocarray_(slist, ssize as usize + 1).as_ptr();
                *slist.add(ssize as usize) = s;
                ssize += 1;
                ControlFlow::<(), ()>::Continue(())
            });
            if (ssize == 0) {
                break 'fail;
            }
            (*fs).s = cmd_find_best_session(slist, ssize, (*fs).flags);
            if ((*fs).s.is_null()) {
                break 'fail;
            }
            free_(slist);
            return cmd_find_best_winlink_with_window(fs);
        }

        // fail:
        free_(slist);
        return -1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_best_winlink_with_window(fs: *mut cmd_find_state) -> i32 {
    let __func__ = c"cmd_find_best_winlink_with_window".as_ptr();
    unsafe {
        log_debug(c"%s: window is @%u".as_ptr(), __func__, (*(*fs).w).id);

        let mut wl = null_mut();
        if (!(*(*fs).s).curw.is_null() && (*(*(*fs).s).curw).window == (*fs).w) {
            wl = (*(*fs).s).curw;
        } else {
            rb_foreach(&raw mut (*(*fs).s).windows, |wl_loop| {
                if ((*wl_loop).window == (*fs).w) {
                    wl = wl_loop;
                    return ControlFlow::<(), ()>::Break(());
                }
                ControlFlow::<(), ()>::Continue(())
            });
        }
        if (wl.is_null()) {
            return -1;
        }
        (*fs).wl = wl;
        (*fs).idx = (*(*fs).wl).idx;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_map_table(table: *const [*const c_char; 2], s: *const c_char) -> *const c_char {
    unsafe {
        let mut i = 0;
        while !(*table.add(i))[0].is_null() {
            if unsafe { strcmp(s, (*table.add(i))[0]) == 0 } {
                return (*table.add(i))[1];
            }
            i += 1;
        }
        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_get_session(fs: *mut cmd_find_state, session: *const c_char) -> i32 {
    let __func__ = c"cmd_find_get_session".as_ptr();
    unsafe {
        log_debug(c"%s: %s".as_ptr(), __func__, session);

        if (*session == b'$' as _) {
            (*fs).s = session_find_by_id_str(session);
            if ((*fs).s.is_null()) {
                return -1;
            }
            return 0;
        }

        (*fs).s = session_find(session.cast_mut()); // TODO this is invalid casting away const
        if (!(*fs).s.is_null()) {
            return 0;
        }

        let c = cmd_find_client(null_mut(), session, 1);
        if (!c.is_null() && !(*c).session.is_null()) {
            (*fs).s = (*c).session;
            return 0;
        }

        if ((*fs).flags & CMD_FIND_EXACT_SESSION != 0) {
            return -1;
        }

        let mut s: *mut session = null_mut();
        if rb_foreach(&raw mut sessions, |s_loop| {
            if (strncmp(session, (*s_loop).name, strlen(session)) == 0) {
                if (!s.is_null()) {
                    return ControlFlow::<(), ()>::Break(());
                }
                s = s_loop;
            }
            ControlFlow::<(), ()>::Continue(())
        })
        .is_some()
        {
            return -1;
        }
        if (!s.is_null()) {
            (*fs).s = s;
            return 0;
        }

        s = null_mut();
        if rb_foreach(&raw mut sessions, |s_loop| {
            if (fnmatch(session, (*s_loop).name, 0) == 0) {
                if (!s.is_null()) {
                    return ControlFlow::<(), ()>::Break(());
                }
                s = s_loop;
            }
            ControlFlow::<(), ()>::Continue(())
        })
        .is_some()
        {
            return -1;
        }
        if (!s.is_null()) {
            (*fs).s = s;
            return 0;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_get_window(fs: *mut cmd_find_state, window: *const c_char, only: i32) -> i32 {
    let __func__ = c"cmd_find_get_window".as_ptr();
    unsafe {
        log_debug(c"%s: %s".as_ptr(), __func__, window);

        if (*window == b'@' as c_char) {
            (*fs).w = window_find_by_id_str(window);
            if ((*fs).w.is_null()) {
                return -1;
            }
            return cmd_find_best_session_with_window(fs);
        }

        (*fs).s = (*(*fs).current).s;

        if (cmd_find_get_window_with_session(fs, window) == 0) {
            return 0;
        }

        if (only == 0 && cmd_find_get_session(fs, window) == 0) {
            (*fs).wl = (*(*fs).s).curw;
            (*fs).w = (*(*fs).wl).window;
            if (!(*fs).flags & CMD_FIND_WINDOW_INDEX != 0) {
                (*fs).idx = (*(*fs).wl).idx;
            }
            return 0;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_get_window_with_session(fs: *mut cmd_find_state, window: *const c_char) -> i32 {
    let __func__ = c"cmd_find_get_window_with_session".as_ptr();
    unsafe {
        let mut errstr: *const c_char = null();
        let mut idx = 0i32;
        let mut n = 0i32;
        let mut exact = 0i32;
        let mut s = null_mut();

        log_debug(c"%s: %s".as_ptr(), __func__, window);
        exact = ((*fs).flags & CMD_FIND_EXACT_WINDOW);

        (*fs).wl = (*(*fs).s).curw;
        (*fs).w = (*(*fs).wl).window;

        if (*window == b'@' as _) {
            (*fs).w = window_find_by_id_str(window);
            if ((*fs).w.is_null() || session_has((*fs).s, (*fs).w) == 0) {
                return -1;
            }
            return cmd_find_best_winlink_with_window(fs);
        }

        if (exact == 0 && (*window == b'+' as _ || *window == b'-' as _)) {
            if (*window.add(1) != b'\0' as _) {
                n = strtonum(window.add(1), 1, i32::MAX as i64, null_mut()) as i32;
            } else {
                n = 1;
            }
            s = (*fs).s;
            if ((*fs).flags & CMD_FIND_WINDOW_INDEX != 0) {
                if (*window == b'+' as _) {
                    if (i32::MAX - (*(*s).curw).idx < n) {
                        return -1;
                    }
                    (*fs).idx = (*(*s).curw).idx + n;
                } else {
                    if (n > (*(*s).curw).idx) {
                        return -1;
                    }
                    (*fs).idx = (*(*s).curw).idx - n;
                }
                return 0;
            }
            if (*window == b'+' as _) {
                (*fs).wl = winlink_next_by_number((*s).curw, s, n);
            } else {
                (*fs).wl = winlink_previous_by_number((*s).curw, s, n);
            }
            if (!(*fs).wl.is_null()) {
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            }
        }

        if (exact == 0) {
            if (strcmp(window, c"!".as_ptr()) == 0) {
                (*fs).wl = tailq_first(&raw mut (*(*fs).s).lastw);
                if ((*fs).wl.is_null()) {
                    return -1;
                }
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            } else if (strcmp(window, c"^".as_ptr()) == 0) {
                (*fs).wl = rb_min(&raw mut (*(*fs).s).windows);
                if ((*fs).wl.is_null()) {
                    return -1;
                }
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            } else if (strcmp(window, c"$".as_ptr()) == 0) {
                (*fs).wl = rb_max(&raw mut (*(*fs).s).windows);
                if ((*fs).wl.is_null()) {
                    return -1;
                }
                (*fs).idx = (*(*fs).wl).idx;
                (*fs).w = (*(*fs).wl).window;
                return 0;
            }
        }

        if (*window != b'+' as _ && *window != b'-' as _) {
            idx = strtonum(window, 0, i32::MAX as i64, &raw mut errstr) as i32;
            if (errstr.is_null()) {
                (*fs).wl = winlink_find_by_index(&raw mut (*(*fs).s).windows, idx);
                if (!(*fs).wl.is_null()) {
                    (*fs).idx = (*(*fs).wl).idx;
                    (*fs).w = (*(*fs).wl).window;
                    return 0;
                }
                if ((*fs).flags & CMD_FIND_WINDOW_INDEX != 0) {
                    (*fs).idx = idx;
                    return 0;
                }
            }
        }

        (*fs).wl = null_mut();
        if rb_foreach(&raw mut (*(*fs).s).windows, |wl| {
            if (strcmp(window, (*(*wl).window).name) == 0) {
                if (!(*fs).wl.is_null()) {
                    return ControlFlow::<(), ()>::Break(());
                }
                (*fs).wl = wl;
            }
            ControlFlow::<(), ()>::Continue(())
        })
        .is_some()
        {
            return -1;
        }
        if (!(*fs).wl.is_null()) {
            (*fs).idx = (*(*fs).wl).idx;
            (*fs).w = (*(*fs).wl).window;
            return 0;
        }

        if (exact != 0) {
            return -1;
        }

        (*fs).wl = null_mut();
        if rb_foreach(&raw mut (*(*fs).s).windows, |wl| {
            if (strncmp(window, (*(*wl).window).name, strlen(window)) == 0) {
                if (!(*fs).wl.is_null()) {
                    return ControlFlow::<(), ()>::Break(());
                }
                (*fs).wl = wl;
            }
            ControlFlow::<(), ()>::Continue(())
        })
        .is_some()
        {
            return -1;
        };
        if (!(*fs).wl.is_null()) {
            (*fs).idx = (*(*fs).wl).idx;
            (*fs).w = (*(*fs).wl).window;
            return 0;
        }

        (*fs).wl = null_mut();
        if rb_foreach(&raw mut (*(*fs).s).windows, |wl| {
            if (fnmatch(window, (*(*wl).window).name, 0) == 0) {
                if (!(*fs).wl.is_null()) {
                    return ControlFlow::<(), ()>::Break(());
                }
                (*fs).wl = wl;
            }
            ControlFlow::<(), ()>::Continue(())
        })
        .is_some()
        {
            return -1;
        }
        if (!(*fs).wl.is_null()) {
            (*fs).idx = (*(*fs).wl).idx;
            (*fs).w = (*(*fs).wl).window;
            return 0;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_get_pane(fs: *mut cmd_find_state, pane: *const c_char, only: i32) -> i32 {
    let __func__ = c"cmd_find_get_pane".as_ptr();
    unsafe {
        log_debug(c"%s: %s".as_ptr(), __func__, pane);

        if (*pane == b'%' as _) {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            (*fs).w = (*(*fs).wp).window;
            return cmd_find_best_session_with_window(fs);
        }

        (*fs).s = (*(*fs).current).s;
        (*fs).wl = (*(*fs).current).wl;
        (*fs).idx = (*(*fs).current).idx;
        (*fs).w = (*(*fs).current).w;

        if (cmd_find_get_pane_with_window(fs, pane) == 0) {
            return 0;
        }

        if (only == 0 && cmd_find_get_window(fs, pane, 0) == 0) {
            (*fs).wp = (*(*fs).w).active;
            return 0;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_get_pane_with_session(fs: *mut cmd_find_state, pane: *const c_char) -> i32 {
    let __func__ = c"cmd_find_get_pane_with_session".as_ptr();
    unsafe {
        log_debug(c"%s: %s".as_ptr(), __func__, pane);

        if (*pane == b'%' as _) {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            (*fs).w = (*(*fs).wp).window;
            return cmd_find_best_winlink_with_window(fs);
        }

        (*fs).wl = (*(*fs).s).curw;
        (*fs).idx = (*(*fs).wl).idx;
        (*fs).w = (*(*fs).wl).window;

        cmd_find_get_pane_with_window(fs, pane)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_get_pane_with_window(fs: *mut cmd_find_state, pane: *const c_char) -> i32 {
    let __func__ = c"cmd_find_get_pane_with_window".as_ptr();
    unsafe {
        let mut n = 0u32;
        let mut errstr: *const c_char = null();

        log_debug(c"%s: %s".as_ptr(), __func__, pane);

        if (*pane == b'%' as _) {
            (*fs).wp = window_pane_find_by_id_str(pane);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            if ((*(*fs).wp).window != (*fs).w) {
                return -1;
            }
            return 0;
        }

        if (strcmp(pane, c"!".as_ptr()) == 0) {
            (*fs).wp = tailq_first(&raw mut (*(*fs).w).last_panes);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            return 0;
        } else if (strcmp(pane, c"{up-of}".as_ptr()) == 0) {
            (*fs).wp = window_pane_find_up((*(*fs).w).active);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            return 0;
        } else if (strcmp(pane, c"{down-of}".as_ptr()) == 0) {
            (*fs).wp = window_pane_find_down((*(*fs).w).active);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            return 0;
        } else if (strcmp(pane, c"{left-of}".as_ptr()) == 0) {
            (*fs).wp = window_pane_find_left((*(*fs).w).active);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            return 0;
        } else if (strcmp(pane, c"{right-of}".as_ptr()) == 0) {
            (*fs).wp = window_pane_find_right((*(*fs).w).active);
            if ((*fs).wp.is_null()) {
                return -1;
            }
            return 0;
        }

        if (*pane == b'+' as _ || *pane == b'-' as _) {
            if (*pane.add(1) != b'\0' as _) {
                n = strtonum(pane.add(1), 1, i32::MAX as i64, null_mut()) as u32;
            } else {
                n = 1;
            }
            let wp = (*(*fs).w).active;
            if (*pane == b'+' as _) {
                (*fs).wp = window_pane_next_by_number((*fs).w, wp, n);
            } else {
                (*fs).wp = window_pane_previous_by_number((*fs).w, wp, n);
            }
            if (!(*fs).wp.is_null()) {
                return 0;
            }
        }

        let idx = strtonum(pane, 0, i32::MAX as i64, &raw mut errstr) as i32;
        if (errstr.is_null()) {
            (*fs).wp = window_pane_at_index((*fs).w, idx as u32);
            if (!(*fs).wp.is_null()) {
                return 0;
            }
        }

        (*fs).wp = window_find_string((*fs).w, pane);
        if (!(*fs).wp.is_null()) {
            return 0;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_clear_state(fs: *mut cmd_find_state, flags: i32) {
    unsafe {
        memset0(fs);

        (*fs).flags = flags;

        (*fs).idx = -1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_empty_state(fs: *mut cmd_find_state) -> i32 {
    unsafe { ((*fs).s.is_null() && (*fs).wl.is_null() && (*fs).w.is_null() && (*fs).wp.is_null()) as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_valid_state(fs: *mut cmd_find_state) -> boolint {
    unsafe {
        if ((*fs).s.is_null() || (*fs).wl.is_null() || (*fs).w.is_null() || (*fs).wp.is_null()) {
            return boolint::false_();
        }

        if !session_alive((*fs).s) {
            return boolint::false_();
        }

        let mut wl = null_mut();
        rb_foreach(&raw mut (*(*fs).s).windows, |wl_| {
            wl = wl_;
            if ((*wl).window == (*fs).w && wl == (*fs).wl) {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        });

        if (wl.is_null()) {
            return boolint::false_();
        }

        if ((*fs).w != (*(*fs).wl).window) {
            return boolint::false_();
        }

        window_has_pane((*fs).w, (*fs).wp)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_copy_state(dst: *mut cmd_find_state, src: *mut cmd_find_state) {
    unsafe {
        (*dst).s = (*src).s;
        (*dst).wl = (*src).wl;
        (*dst).idx = (*src).idx;
        (*dst).w = (*src).w;
        (*dst).wp = (*src).wp;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_log_state(prefix: *const c_char, fs: *const cmd_find_state) {
    unsafe {
        if (!(*fs).s.is_null()) {
            log_debug(c"%s: s=$%u %s".as_ptr(), prefix, (*(*fs).s).id, (*(*fs).s).name);
        } else {
            log_debug(c"%s: s=none".as_ptr(), prefix);
        }
        if (!(*fs).wl.is_null()) {
            log_debug(
                c"%s: wl=%u %d w=@%u %s".as_ptr(),
                prefix,
                (*(*fs).wl).idx,
                ((*(*fs).wl).window == (*fs).w) as i32,
                (*(*fs).w).id,
                (*(*fs).w).name,
            );
        } else {
            log_debug(c"%s: wl=none".as_ptr(), prefix);
        }
        if (!(*fs).wp.is_null()) {
            log_debug(c"%s: wp=%%%u".as_ptr(), prefix, (*(*fs).wp).id);
        } else {
            log_debug(c"%s: wp=none".as_ptr(), prefix);
        }
        if ((*fs).idx != -1) {
            log_debug(c"%s: idx=%d".as_ptr(), prefix, (*fs).idx);
        } else {
            log_debug(c"%s: idx=none".as_ptr(), prefix);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_session(fs: *mut cmd_find_state, s: *mut session, flags: i32) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = s;
        (*fs).wl = (*(*fs).s).curw;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c"cmd_find_from_session".as_ptr(), fs);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_winlink(fs: *mut cmd_find_state, wl: *mut winlink, flags: i32) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = (*wl).session;
        (*fs).wl = wl;
        (*fs).w = (*wl).window;
        (*fs).wp = (*(*wl).window).active;

        cmd_find_log_state(c"cmd_find_from_winlink".as_ptr(), fs);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_session_window(
    fs: *mut cmd_find_state,
    s: *mut session,
    w: *mut window,
    flags: i32,
) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = s;
        (*fs).w = w;
        if (cmd_find_best_winlink_with_window(fs) != 0) {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c"cmd_find_from_session_window".as_ptr(), fs);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_window(fs: *mut cmd_find_state, w: *mut window, flags: i32) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).w = w;
        if (cmd_find_best_session_with_window(fs) != 0) {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        if (cmd_find_best_winlink_with_window(fs) != 0) {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c"cmd_find_from_window".as_ptr(), fs);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_winlink_pane(
    fs: *mut cmd_find_state,
    wl: *mut winlink,
    wp: *mut window_pane,
    flags: i32,
) {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = (*wl).session;
        (*fs).wl = wl;
        (*fs).idx = (*(*fs).wl).idx;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = wp;

        cmd_find_log_state(c"cmd_find_from_winlink_pane".as_ptr(), fs);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_pane(fs: *mut cmd_find_state, wp: *mut window_pane, flags: i32) -> i32 {
    unsafe {
        if (cmd_find_from_window(fs, (*wp).window, flags) != 0) {
            return -1;
        }
        (*fs).wp = wp;

        cmd_find_log_state(c"cmd_find_from_pane".as_ptr(), fs);
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_nothing(fs: *mut cmd_find_state, flags: i32) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        (*fs).s = cmd_find_best_session(null_mut(), 0, flags);
        if ((*fs).s.is_null()) {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).wl = (*(*fs).s).curw;
        (*fs).idx = (*(*fs).wl).idx;
        (*fs).w = (*(*fs).wl).window;
        (*fs).wp = (*(*fs).w).active;

        cmd_find_log_state(c"cmd_find_from_nothing".as_ptr(), fs);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_mouse(fs: *mut cmd_find_state, m: *mut mouse_event, flags: i32) -> i32 {
    unsafe {
        cmd_find_clear_state(fs, flags);

        if (*m).valid == 0 {
            return -1;
        }

        (*fs).wp = transmute_ptr(cmd_mouse_pane(m, &raw mut (*fs).s, &raw mut (*fs).wl));
        if ((*fs).wp.is_null()) {
            cmd_find_clear_state(fs, flags);
            return -1;
        }
        (*fs).w = (*(*fs).wl).window;

        cmd_find_log_state(c"cmd_find_from_mouse".as_ptr(), fs);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_from_client(fs: *mut cmd_find_state, c: *mut client, flags: i32) -> i32 {
    let __func__ = c"cmd_find_from_client".as_ptr();
    unsafe {
        // struct window_pane *wp;

        'unknown_pane: {
            if (c.is_null()) {
                return cmd_find_from_nothing(fs, flags);
            }

            if (!(*c).session.is_null()) {
                cmd_find_clear_state(fs, flags);

                (*fs).wp = server_client_get_pane(c);
                if ((*fs).wp.is_null()) {
                    cmd_find_from_session(fs, (*c).session, flags);
                    return 0;
                }
                (*fs).s = (*c).session;
                (*fs).wl = (*(*fs).s).curw;
                (*fs).w = (*(*fs).wl).window;

                cmd_find_log_state(__func__, fs);
                return 0;
            }
            cmd_find_clear_state(fs, flags);

            let wp = cmd_find_inside_pane(c);
            if (wp.is_null()) {
                break 'unknown_pane;
            }

            (*fs).w = (*wp).window;
            if (cmd_find_best_session_with_window(fs) != 0) {
                break 'unknown_pane;
            }
            (*fs).wl = (*(*fs).s).curw;
            (*fs).w = (*(*fs).wl).window;
            (*fs).wp = (*(*fs).w).active;

            cmd_find_log_state(__func__, fs);
            return 0;
        }
        // unknown_pane:
        return cmd_find_from_nothing(fs, flags);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_target(
    fs: *mut cmd_find_state,
    item: *mut cmdq_item,
    target: *const c_char,
    type_: cmd_find_type,
    mut flags: i32,
) -> i32 {
    let __func__ = c"cmd_find_target".as_ptr();
    unsafe {
        let mut m: *mut mouse_event = null_mut();
        let mut current: cmd_find_state = zeroed();

        let mut colon: *mut c_char = null_mut();
        let mut period: *mut c_char = null_mut();
        let mut copy: *mut c_char = null_mut();
        let sizeof_tmp = 256;
        let mut tmp: [c_char; 256] = [0; 256];

        let mut session: *const c_char = null();
        let mut window: *const c_char = null();
        let mut pane: *const c_char = null();
        let mut s: *const c_char = null();

        let mut window_only = 0;
        let mut pane_only = 0;

        'error: {
            'no_pane: {
                'no_window: {
                    'no_session: {
                        'found: {
                            'current: {
                                if (flags & CMD_FIND_CANFAIL != 0) {
                                    flags |= CMD_FIND_QUIET;
                                }

                                s = match type_ {
                                    cmd_find_type::CMD_FIND_PANE => c"pane".as_ptr(),
                                    cmd_find_type::CMD_FIND_WINDOW => c"window".as_ptr(),
                                    cmd_find_type::CMD_FIND_SESSION => c"session".as_ptr(),
                                };

                                tmp[0] = b'\0' as c_char;
                                if (flags & CMD_FIND_PREFER_UNATTACHED != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"PREFER_UNATTACHED,".as_ptr(), sizeof_tmp);
                                }
                                if (flags & CMD_FIND_QUIET != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"QUIET,".as_ptr(), sizeof_tmp);
                                }
                                if (flags & CMD_FIND_WINDOW_INDEX != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"WINDOW_INDEX,".as_ptr(), sizeof_tmp);
                                }
                                if (flags & CMD_FIND_DEFAULT_MARKED != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"DEFAULT_MARKED,".as_ptr(), sizeof_tmp);
                                }
                                if (flags & CMD_FIND_EXACT_SESSION != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"EXACT_SESSION,".as_ptr(), sizeof_tmp);
                                }
                                if (flags & CMD_FIND_EXACT_WINDOW != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"EXACT_WINDOW,".as_ptr(), sizeof_tmp);
                                }
                                if (flags & CMD_FIND_CANFAIL != 0) {
                                    strlcat(tmp.as_mut_ptr(), c"CANFAIL,".as_ptr(), sizeof_tmp);
                                }
                                if (tmp[0] != b'\0' as c_char) {
                                    tmp[strlen(tmp.as_mut_ptr()) - 1] = b'\0' as c_char;
                                } else {
                                    strlcat(tmp.as_mut_ptr(), c"NONE".as_ptr(), sizeof_tmp);
                                }
                                log_debug(
                                    c"%s: target %s, type %s, item %p, flags %s".as_ptr(),
                                    __func__,
                                    if target.is_null() { c"none".as_ptr() } else { target },
                                    s,
                                    item,
                                    tmp,
                                );

                                cmd_find_clear_state(fs, flags);

                                if server_check_marked().as_bool() && (flags & CMD_FIND_DEFAULT_MARKED != 0) {
                                    (*fs).current = &raw mut marked_pane;
                                    log_debug(c"%s: current is marked pane".as_ptr(), __func__);
                                } else if cmd_find_valid_state(cmdq_get_current(item)).as_bool() {
                                    (*fs).current = cmdq_get_current(item);
                                    log_debug(c"%s: current is from queue".as_ptr(), __func__);
                                } else if (cmd_find_from_client(&raw mut current, cmdq_get_client(item), flags) == 0) {
                                    (*fs).current = &raw mut current;
                                    log_debug(c"%s: current is from client".as_ptr(), __func__);
                                } else {
                                    if (!flags & CMD_FIND_QUIET != 0) {
                                        cmdq_error(item, c"no current target".as_ptr());
                                    }
                                    break 'error;
                                }
                                if !cmd_find_valid_state((*fs).current) {
                                    fatalx(c"invalid current find state".as_ptr());
                                }

                                /* An empty or NULL target is the current. */
                                if (target.is_null() || *target == b'\0' as _) {
                                    break 'current;
                                }

                                /* Mouse target is a plain = or {mouse}. */
                                if strcmp(target, c"=".as_ptr()) == 0 || strcmp(target, c"{mouse}".as_ptr()) == 0 {
                                    m = &raw mut (*cmdq_get_event(item)).m;
                                    match (type_) {
                                        cmd_find_type::CMD_FIND_PANE => {
                                            (*fs).wp =
                                                transmute_ptr(cmd_mouse_pane(m, &raw mut (*fs).s, &raw mut (*fs).wl));
                                            if (!(*fs).wp.is_null()) {
                                                (*fs).w = (*(*fs).wl).window;
                                            } else {
                                                /* FALLTHROUGH; copied from below */
                                                (*fs).wl = transmute_ptr(cmd_mouse_window(m, &raw mut (*fs).s));
                                                if ((*fs).wl.is_null() && !(*fs).s.is_null()) {
                                                    (*fs).wl = (*(*fs).s).curw;
                                                }
                                                if (!(*fs).wl.is_null()) {
                                                    (*fs).w = (*(*fs).wl).window;
                                                    (*fs).wp = (*(*fs).w).active;
                                                }
                                            }
                                        }
                                        cmd_find_type::CMD_FIND_WINDOW | cmd_find_type::CMD_FIND_SESSION => {
                                            (*fs).wl = transmute_ptr(cmd_mouse_window(m, &raw mut (*fs).s));
                                            if ((*fs).wl.is_null() && !(*fs).s.is_null()) {
                                                (*fs).wl = (*(*fs).s).curw;
                                            }
                                            if (!(*fs).wl.is_null()) {
                                                (*fs).w = (*(*fs).wl).window;
                                                (*fs).wp = (*(*fs).w).active;
                                            }
                                        }
                                    }
                                    if ((*fs).wp.is_null()) {
                                        if (!flags & CMD_FIND_QUIET != 0) {
                                            cmdq_error(item, c"no mouse target".as_ptr());
                                        }
                                        break 'error;
                                    }
                                    break 'found;
                                }

                                if (strcmp(target, c"~".as_ptr()) == 0 || strcmp(target, c"{marked}".as_ptr()) == 0) {
                                    if !server_check_marked() {
                                        if (!flags & CMD_FIND_QUIET != 0) {
                                            cmdq_error(item, c"no marked target".as_ptr());
                                        }
                                        break 'error;
                                    }
                                    cmd_find_copy_state(fs, &raw mut marked_pane);
                                    break 'found;
                                }

                                copy = xstrdup(target).as_ptr();
                                colon = strchr(copy, b':' as _);
                                if (!colon.is_null()) {
                                    *colon = b'\0' as _;
                                    colon = colon.add(1);
                                }
                                if (colon.is_null()) {
                                    period = strchr(copy, b'.' as _);
                                } else {
                                    period = strchr(colon, b'.' as _);
                                }
                                if (!period.is_null()) {
                                    *period = b'\0' as _;
                                    period = period.add(1);
                                }

                                session = null_mut();
                                window = null_mut();
                                pane = null_mut();
                                if (!colon.is_null() && !period.is_null()) {
                                    session = copy;
                                    window = colon;
                                    window_only = 1;
                                    pane = period;
                                    pane_only = 1;
                                } else if (!colon.is_null() && period.is_null()) {
                                    session = copy;
                                    window = colon;
                                    window_only = 1;
                                } else if (colon.is_null() && !period.is_null()) {
                                    window = copy;
                                    pane = period;
                                    pane_only = 1;
                                } else {
                                    if (*copy == b'$' as _) {
                                        session = copy;
                                    } else if (*copy == b'@' as _) {
                                        window = copy;
                                    } else if (*copy == b'%' as _) {
                                        pane = copy;
                                    } else {
                                        match type_ {
                                            cmd_find_type::CMD_FIND_SESSION => session = copy,
                                            cmd_find_type::CMD_FIND_WINDOW => window = copy,
                                            cmd_find_type::CMD_FIND_PANE => pane = copy,
                                        }
                                    }
                                }

                                if (!session.is_null() && *session == b'=' as _) {
                                    session = session.add(1);
                                    (*fs).flags |= CMD_FIND_EXACT_SESSION;
                                }
                                if (!window.is_null() && *window == b'=' as _) {
                                    window = window.add(1);
                                    (*fs).flags |= CMD_FIND_EXACT_WINDOW;
                                }

                                if (!session.is_null() && *session == b'\0' as _) {
                                    session = null_mut();
                                }
                                if (!window.is_null() && *window == b'\0' as _) {
                                    window = null_mut();
                                }
                                if (!pane.is_null() && *pane == b'\0' as _) {
                                    pane = null_mut();
                                }

                                if (!session.is_null()) {
                                    session =
                                        cmd_find_map_table(&raw const cmd_find_session_table as *const _, session);
                                }
                                if (!window.is_null()) {
                                    window = cmd_find_map_table(&raw const cmd_find_window_table as *const _, window);
                                }
                                if (!pane.is_null()) {
                                    pane = cmd_find_map_table(&raw const cmd_find_pane_table as *const _, pane);
                                }

                                if (!session.is_null() || !window.is_null() || !pane.is_null()) {
                                    log_debug(
                                        c"%s: target %s is %s%s%s%s%s%s".as_ptr(),
                                        __func__,
                                        target,
                                        if session.is_null() {
                                            c"".as_ptr()
                                        } else {
                                            c"session ".as_ptr()
                                        },
                                        if session.is_null() { c"".as_ptr() } else { session },
                                        if window.is_null() {
                                            c"".as_ptr()
                                        } else {
                                            c"window ".as_ptr()
                                        },
                                        if window.is_null() { c"".as_ptr() } else { window },
                                        if pane.is_null() {
                                            c"".as_ptr()
                                        } else {
                                            c"pane ".as_ptr()
                                        },
                                        if pane.is_null() { c"".as_ptr() } else { pane },
                                    );
                                }

                                if (!pane.is_null() && (flags & CMD_FIND_WINDOW_INDEX != 0)) {
                                    if (!flags & CMD_FIND_QUIET != 0) {
                                        cmdq_error(item, c"can't specify pane here".as_ptr());
                                    }
                                    break 'error;
                                }

                                if (!session.is_null()) {
                                    if (cmd_find_get_session(fs, session) != 0) {
                                        break 'no_session;
                                    }

                                    if (window.is_null() && pane.is_null()) {
                                        (*fs).wl = (*(*fs).s).curw;
                                        (*fs).idx = -1;
                                        (*fs).w = (*(*fs).wl).window;
                                        (*fs).wp = (*(*fs).w).active;
                                        break 'found;
                                    }

                                    if (!window.is_null() && pane.is_null()) {
                                        if (cmd_find_get_window_with_session(fs, window) != 0) {
                                            break 'no_window;
                                        }
                                        if (!(*fs).wl.is_null()) {
                                            (*fs).wp = (*(*(*fs).wl).window).active;
                                        }
                                        break 'found;
                                    }

                                    if (window.is_null() && !pane.is_null()) {
                                        if (cmd_find_get_pane_with_session(fs, pane) != 0) {
                                            break 'no_pane;
                                        }
                                        break 'found;
                                    }

                                    if (cmd_find_get_window_with_session(fs, window) != 0) {
                                        break 'no_window;
                                    }
                                    if (cmd_find_get_pane_with_window(fs, pane) != 0) {
                                        break 'no_pane;
                                    }
                                    break 'found;
                                }

                                if (!window.is_null() && !pane.is_null()) {
                                    if (cmd_find_get_window(fs, window, window_only) != 0) {
                                        break 'no_window;
                                    }
                                    if (cmd_find_get_pane_with_window(fs, pane) != 0) {
                                        break 'no_pane;
                                    }
                                    break 'found;
                                }

                                if (!window.is_null() && pane.is_null()) {
                                    if (cmd_find_get_window(fs, window, window_only) != 0) {
                                        break 'no_window;
                                    }
                                    if (!(*fs).wl.is_null()) {
                                        (*fs).wp = (*(*(*fs).wl).window).active;
                                    }
                                    break 'found;
                                }

                                if (window.is_null() && !pane.is_null()) {
                                    if (cmd_find_get_pane(fs, pane, pane_only) != 0) {
                                        break 'no_pane;
                                    }
                                    break 'found;
                                }

                                //
                            }
                            // current:
                            cmd_find_copy_state(fs, (*fs).current);
                            if (flags & CMD_FIND_WINDOW_INDEX != 0) {
                                (*fs).idx = -1;
                            }
                            break 'found;
                        }
                        // found:
                        (*fs).current = null_mut();
                        cmd_find_log_state(__func__, fs);

                        free_(copy);
                        return 0;
                    }
                    // no_session:
                    if (!flags & CMD_FIND_QUIET != 0) {
                        cmdq_error(item, c"can't find session: %s".as_ptr(), session);
                    }
                    break 'error;
                }
                // no_window:
                if (!flags & CMD_FIND_QUIET != 0) {
                    cmdq_error(item, c"can't find window: %s".as_ptr(), window);
                }
                break 'error;
            }
            // no_pane:
            if (!flags & CMD_FIND_QUIET != 0) {
                cmdq_error(item, c"can't find pane: %s".as_ptr(), pane);
            }
            break 'error;
        }

        // error:
        (*fs).current = null_mut();
        log_debug(c"%s: error".as_ptr(), __func__);

        free_(copy);
        if (flags & CMD_FIND_CANFAIL != 0) {
            return 0;
        }
        return -1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_current_client(item: *mut cmdq_item, quiet: i32) -> *mut client {
    let __func__ = c"cmd_find_current_client".as_ptr();
    unsafe {
        let mut c: *mut client = null_mut();
        let mut found: *mut client = null_mut();
        let mut s = null_mut();
        let mut wp = null_mut();
        let mut fs: cmd_find_state = zeroed();

        if (!item.is_null()) {
            c = cmdq_get_client(item);
        }
        if (!c.is_null() && !(*c).session.is_null()) {
            return c;
        }

        found = null_mut();
        if !c.is_null()
            && ({
                wp = cmd_find_inside_pane(c);
                !wp.is_null()
            })
        {
            cmd_find_clear_state(&raw mut fs, CMD_FIND_QUIET);
            fs.w = (*wp).window;
            if (cmd_find_best_session_with_window(&raw mut fs) == 0) {
                found = cmd_find_best_client(fs.s);
            }
        } else {
            s = cmd_find_best_session(null_mut(), 0, CMD_FIND_QUIET);
            if (!s.is_null()) {
                found = cmd_find_best_client(s);
            }
        }
        if found.is_null() && !item.is_null() && quiet == 0 {
            cmdq_error(item, c"no current client".as_ptr());
        }
        log_debug(c"%s: no target, return %p".as_ptr(), __func__, found);
        found
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find_client(item: *mut cmdq_item, target: *const c_char, quiet: i32) -> *mut client {
    let __func__ = c"cmd_find_client".as_ptr();
    unsafe {
        // struct client *c;
        // char *copy;
        // size_t size;

        /* A NULL argument means the current client. */
        if (target.is_null()) {
            return cmd_find_current_client(item, quiet);
        }
        let copy = xstrdup(target).as_ptr();

        /* Trim a single trailing colon if any. */
        let size = strlen(copy);
        if (size != 0 && *copy.add(size - 1) == b':' as _) {
            *copy.add(size - 1) = b'\0' as _;
        }

        let mut c = null_mut();
        /* Check name and path of each client. */
        tailq_foreach(&raw mut clients, |c_| {
            c = c_;
            if ((*c).session.is_null()) {
                return ControlFlow::Continue(());
            }
            if (strcmp(copy, (*c).name) == 0) {
                return ControlFlow::Break(());
            }

            if (*(*c).ttyname == b'\0' as _) {
                return ControlFlow::Continue(());
            }
            if (strcmp(copy, (*c).ttyname) == 0) {
                return ControlFlow::Break(());
            }
            if (strncmp((*c).ttyname, _PATH_DEV, SIZEOF_PATH_DEV - 1) != 0) {
                return ControlFlow::Continue(());
            }
            if (strcmp(copy, (*c).ttyname.add(SIZEOF_PATH_DEV - 1)) == 0) {
                return ControlFlow::Break(());
            }

            ControlFlow::Continue(())
        });

        if (c.is_null() && quiet == 0) {
            cmdq_error(item, c"can't find client: %s".as_ptr(), copy);
        }

        free_(copy);
        log_debug(c"%s: target %s, return %p".as_ptr(), __func__, target, c);
        c
    }
}
