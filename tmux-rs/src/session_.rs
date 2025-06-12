// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::*;

use crate::compat::{
    RB_GENERATE, RB_GENERATE_STATIC, VIS_CSTYLE, VIS_NL, VIS_OCTAL, VIS_TAB,
    queue::{tailq_empty, tailq_foreach, tailq_init, tailq_insert_tail, tailq_remove},
    strtonum,
    tree::{
        rb_empty, rb_find, rb_foreach, rb_init, rb_initializer, rb_insert, rb_max, rb_min, rb_next,
        rb_prev, rb_remove, rb_root,
    },
};

RB_GENERATE!(sessions, session, entry, session_cmp);
RB_GENERATE!(session_groups, session_group, entry, session_group_cmp);

#[unsafe(no_mangle)]
pub static mut sessions: sessions = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub static mut next_session_id: u32 = 0;

#[unsafe(no_mangle)]
pub static mut session_groups: session_groups = rb_initializer();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_cmp(s1: *const session, s2: *const session) -> i32 {
    unsafe { libc::strcmp((*s1).name, (*s2).name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_cmp(
    s1: *const session_group,
    s2: *const session_group,
) -> i32 {
    unsafe { libc::strcmp((*s1).name, (*s2).name) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_alive(s: *mut session) -> boolint {
    unsafe {
        for s_loop in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            if s_loop == s {
                return boolint::TRUE;
            }
        }
    }

    boolint::FALSE
}

/// Find session by name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_find(name: *mut c_char) -> *mut session {
    let mut s = MaybeUninit::<session>::uninit();
    let s = s.as_mut_ptr();

    unsafe {
        (*s).name = name;
        rb_find(&raw mut sessions, s)
    }
}

/// Find session by id parsed from a string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_find_by_id_str(s: *const c_char) -> *mut session {
    unsafe {
        if *s != b'$' as c_char {
            return null_mut();
        }

        let mut errstr: *const c_char = null();
        let id = strtonum(s.add(1), 0, u32::MAX as i64, &raw mut errstr) as u32;
        if !errstr.is_null() {
            return null_mut();
        }
        transmute_ptr(session_find_by_id(id))
    }
}

/// Find session by id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_find_by_id(id: u32) -> Option<NonNull<session>> {
    unsafe { rb_foreach(&raw mut sessions).find(|s| (*s.as_ptr()).id == id) }
}

/// Create a new session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_create(
    prefix: *const c_char,
    name: *const c_char,
    cwd: *const c_char,
    env: *mut environ,
    oo: *mut options,
    tio: *mut termios,
) -> *mut session {
    unsafe {
        let s = xcalloc1::<session>();
        s.references = 1;
        s.flags = 0;

        s.cwd = xstrdup(cwd).as_ptr();

        tailq_init(&raw mut s.lastw);
        rb_init(&raw mut s.windows);

        s.environ = env;
        s.options = oo;

        status_update_cache(s);

        s.tio = null_mut();
        if (!tio.is_null()) {
            s.tio = xmalloc_::<termios>().as_ptr();
            memcpy__(s.tio, tio);
        }

        if (!name.is_null()) {
            s.name = xstrdup(name).as_ptr();
            s.id = next_session_id;
            next_session_id += 1;
        } else {
            loop {
                s.id = next_session_id;
                next_session_id += 1;
                free_(s.name);
                if (!prefix.is_null()) {
                    xasprintf(&raw mut s.name, c"%s-%u".as_ptr(), prefix, s.id);
                } else {
                    xasprintf(&raw mut s.name, c"%u".as_ptr(), s.id);
                }

                if rb_find(&raw mut sessions, s).is_null() {
                    break;
                }
            }
        }
        rb_insert(&raw mut sessions, s);

        log_debug!("new session {} ${}", _s(s.name), s.id);

        if libc::gettimeofday(&raw mut s.creation_time, null_mut()) != 0 {
            fatal(c"gettimeofday failed".as_ptr());
        }
        session_update_activity(s, &raw mut s.creation_time);

        s
    }
}

/// Add a reference to a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_add_ref(s: *mut session, from: *const c_char) {
    let __func__ = "session_add_ref";
    unsafe {
        (*s).references += 1;
        log_debug!(
            "{}: {} {}, now {}",
            __func__,
            _s((*s).name),
            _s(from),
            (*s).references
        );
    }
}

/// Remove a reference from a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_remove_ref(s: *mut session, from: *const c_char) {
    let __func__ = "session_remove_ref";
    unsafe {
        (*s).references -= 1;
        log_debug!(
            "{}: {} {}, now {}",
            __func__,
            _s((*s).name),
            _s(from),
            (*s).references
        );

        if (*s).references == 0 {
            event_once(-1, EV_TIMEOUT, Some(session_free), s.cast(), null_mut());
        }
    }
}

/// Free session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_free(_fd: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let mut s = arg as *mut session;

        log_debug!(
            "session {} freed ({} references)",
            _s((*s).name),
            (*s).references
        );

        if ((*s).references == 0) {
            environ_free((*s).environ);
            options_free((*s).options);
            free_((*s).name);
            free_(s);
        }
    }
}

/// Destroy a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_destroy(s: *mut session, notify: i32, from: *const c_char) {
    let __func__ = c"session_destroy".as_ptr();
    unsafe {
        log_debug!("session {} destroyed ({})", _s((*s).name), _s(from));

        if (*s).curw.is_null() {
            return;
        }
        (*s).curw = null_mut();

        rb_remove(&raw mut sessions, s);
        if notify != 0 {
            notify_session(c"session-closed".as_ptr(), s);
        }

        free_((*s).tio);

        if event_initialized(&raw mut (*s).lock_timer).as_bool() {
            event_del(&raw mut (*s).lock_timer);
        }

        session_group_remove(s);

        while !tailq_empty(&raw mut (*s).lastw) {
            winlink_stack_remove(&raw mut (*s).lastw, tailq_first(&raw mut (*s).lastw));
        }
        while (!rb_empty(&raw mut (*s).windows)) {
            let wl = rb_root(&raw mut (*s).windows);
            notify_session_window(c"window-unlinked".as_ptr(), s, (*wl).window);
            winlink_remove(&raw mut (*s).windows, wl);
        }

        free_((*s).cwd);

        session_remove_ref(s, __func__);
    }
}

/// Sanitize session name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_check_name(name: *const c_char) -> *mut c_char {
    unsafe {
        let mut new_name = null_mut();
        if *name == b'\0' as c_char {
            return null_mut();
        }
        let copy = xstrdup(name).as_ptr();
        let mut cp = copy;
        while *cp != b'\0' as c_char {
            if *cp == b':' as c_char || *cp == b'.' as c_char {
                *cp = b'_' as c_char;
            }
            cp = cp.add(1);
        }
        utf8_stravis(
            &raw mut new_name,
            copy,
            VIS_OCTAL | VIS_CSTYLE | VIS_TAB | VIS_NL,
        );
        free_(copy);
        new_name
    }
}

/// Lock session if it has timed out.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_lock_timer(fd: i32, events: i16, arg: *mut c_void) {
    unsafe {
        let s = arg as *mut session;

        if (*s).attached == 0 {
            return;
        }

        log_debug!(
            "session {} locked, activity time {}",
            _s((*s).name),
            (*s).activity_time.tv_sec,
        );

        server_lock_session(s);
        recalculate_sizes();
    }
}

/// Update activity time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_update_activity(s: *mut session, from: *mut timeval) {
    unsafe {
        let mut last = &raw mut (*s).last_activity_time;

        memcpy__(last, &raw mut (*s).activity_time);
        if (from.is_null()) {
            libc::gettimeofday(&raw mut (*s).activity_time, null_mut());
        } else {
            memcpy__(&raw mut (*s).activity_time, from);
        }

        log_debug!(
            "session ${} {} activity {}.{:06} (last {}.{:06})",
            (*s).id,
            _s((*s).name),
            (*s).activity_time.tv_sec,
            (*s).activity_time.tv_usec as i32,
            (*last).tv_sec,
            (*last).tv_usec as i32,
        );

        if evtimer_initialized(&raw mut (*s).lock_timer).as_bool() {
            evtimer_del(&raw mut (*s).lock_timer);
        } else {
            evtimer_set(&raw mut (*s).lock_timer, Some(session_lock_timer), s.cast());
        }

        let mut tv = MaybeUninit::<timeval>::uninit();
        let tv = tv.as_mut_ptr();
        if ((*s).attached != 0) {
            timerclear(tv);
            (*tv).tv_sec = options_get_number((*s).options, c"lock-after-time".as_ptr());
            if (*tv).tv_sec != 0 {
                evtimer_add(&raw mut (*s).lock_timer, tv);
            }
        }
    }
}

/// Find the next usable session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_next_session(s: *mut session) -> *mut session {
    unsafe {
        if rb_empty(&raw mut sessions) || !session_alive(s) {
            return null_mut();
        }

        let mut s2 = rb_next(s);
        if s2.is_null() {
            s2 = rb_min(&raw mut sessions);
        }
        if s2 == s {
            return null_mut();
        }

        s2
    }
}

/// Find the previous usable session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_previous_session(s: *mut session) -> *mut session {
    unsafe {
        if rb_empty(&raw mut sessions) || !session_alive(s) {
            return null_mut();
        }

        let mut s2 = rb_prev(s);
        if s2.is_null() {
            s2 = rb_max(&raw mut sessions);
        }
        if s2 == s {
            return null_mut();
        }
        s2
    }
}

/// Attach a window to a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_attach(
    s: *mut session,
    w: *mut window,
    idx: i32,
    cause: *mut *mut c_char,
) -> *mut winlink {
    unsafe {
        let mut wl = winlink_add(&raw mut (*s).windows, idx);

        if wl.is_null() {
            xasprintf(cause, c"index in use: %d".as_ptr(), idx);
            return null_mut();
        }
        (*wl).session = s;
        winlink_set_window(wl, w);
        notify_session_window(c"window-linked".as_ptr(), s, w);

        session_group_synchronize_from(s);
        wl
    }
}

/// Detach a window from a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_detach(s: *mut session, wl: *mut winlink) -> i32 {
    unsafe {
        if (*s).curw == wl && session_last(s) != 0 && session_previous(s, 0) != 0 {
            session_next(s, 0);
        }

        (*wl).flags &= !WINLINK_ALERTFLAGS;
        notify_session_window(c"window-unlinked".as_ptr(), s, (*wl).window);
        winlink_stack_remove(&raw mut (*s).lastw, wl);
        winlink_remove(&raw mut (*s).windows, wl);

        session_group_synchronize_from(s);

        if rb_empty(&raw mut (*s).windows) {
            return 1;
        }
        0
    }
}

/// Return if session has window.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_has(s: *mut session, w: *mut window) -> i32 {
    unsafe {
        tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks)
            .any(|wl| (*wl.as_ptr()).session == s) as i32
    }
}

/*
 * Return 1 if a window is linked outside this session (not including session
 * groups). The window must be in this session!
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_is_linked(s: *mut session, w: *mut window) -> i32 {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            return ((*w).references != session_group_count(sg)) as i32;
        }
        ((*w).references != 1) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_next_alert(mut wl: *mut winlink) -> *mut winlink {
    unsafe {
        while (!wl.is_null()) {
            if (*wl).flags & WINLINK_ALERTFLAGS != 0 {
                break;
            }
            wl = winlink_next(wl);
        }
    }
    wl
}

/* Move session to next window. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_next(s: *mut session, alert: i32) -> i32 {
    // struct winlink *wl;
    unsafe {
        if (*s).curw.is_null() {
            return -1;
        }

        let mut wl = winlink_next((*s).curw);
        if alert != 0 {
            wl = session_next_alert(wl);
        }
        if (wl.is_null()) {
            wl = rb_min(&raw mut (*s).windows);
            if alert != 0
                && ({
                    (wl = session_next_alert(wl));
                    wl.is_null()
                })
            {
                return -1;
            }
        }
        session_set_current(s, wl)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_previous_alert(mut wl: *mut winlink) -> *mut winlink {
    unsafe {
        while (!wl.is_null()) {
            if (*wl).flags & WINLINK_ALERTFLAGS != 0 {
                break;
            }
            wl = winlink_previous(wl);
        }
        wl
    }
}

/* Move session to previous window. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_previous(s: *mut session, alert: i32) -> i32 {
    unsafe {
        if (*s).curw.is_null() {
            return -1;
        }

        let mut wl = winlink_previous((*s).curw);
        if alert != 0 {
            wl = session_previous_alert(wl);
        }
        if (wl.is_null()) {
            wl = rb_max(&raw mut (*s).windows);
            if alert != 0
                && ({
                    (wl = session_previous_alert(wl));
                    wl.is_null()
                })
            {
                return -1;
            }
        }
        session_set_current(s, wl)
    }
}

/* Move session to specific window. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_select(s: *mut session, idx: i32) -> i32 {
    unsafe {
        let mut wl = winlink_find_by_index(&raw mut (*s).windows, idx);
        session_set_current(s, wl)
    }
}

/* Move session to last used window. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_last(s: *mut session) -> i32 {
    unsafe {
        let mut wl = tailq_first(&raw mut (*s).lastw);
        if wl.is_null() {
            return -1;
        }
        if wl == (*s).curw {
            return 1;
        }

        session_set_current(s, wl)
    }
}

/// Set current winlink to wl.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_set_current(s: *mut session, wl: *mut winlink) -> i32 {
    unsafe {
        let mut old: *mut winlink = (*s).curw;

        if wl.is_null() {
            return -1;
        }
        if wl == (*s).curw {
            return 1;
        }

        winlink_stack_remove(&raw mut (*s).lastw, wl);
        winlink_stack_push(&raw mut (*s).lastw, (*s).curw);
        (*s).curw = wl;
        if (options_get_number(global_options, c"focus-events".as_ptr()) != 0) {
            if !old.is_null() {
                window_update_focus((*old).window);
            }
            window_update_focus((*wl).window);
        }
        winlink_clear_flags(wl);
        window_update_activity(NonNull::new_unchecked((*wl).window));
        tty_update_window_offset((*wl).window);
        notify_session(c"session-window-changed".as_ptr(), s);
        0
    }
}

/* Find the session group containing a session. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_contains(target: *mut session) -> *mut session_group {
    unsafe {
        for sg in rb_foreach(&raw mut session_groups) {
            for s in tailq_foreach(&raw mut (*sg.as_ptr()).sessions) {
                if s.as_ptr() == target {
                    return sg.as_ptr();
                }
            }
        }

        null_mut()
    }
}

/* Find session group by name. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_find(name: *const c_char) -> *mut session_group {
    unsafe {
        let mut sg = MaybeUninit::<session_group>::uninit();
        let sg = sg.as_mut_ptr();

        (*sg).name = name;
        rb_find(&raw mut session_groups, sg)
    }
}

/* Create a new session group. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_new(name: *const c_char) -> *mut session_group {
    unsafe {
        let mut sg = session_group_find(name);
        if !sg.is_null() {
            return sg;
        }

        sg = xcalloc1::<session_group>();
        (*sg).name = xstrdup(name).as_ptr();
        tailq_init(&raw mut (*sg).sessions);

        rb_insert(&raw mut session_groups, sg);
        sg
    }
}

/* Add a session to a session group. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_add(sg: *mut session_group, s: *mut session) {
    unsafe {
        if session_group_contains(s).is_null() {
            tailq_insert_tail(&raw mut (*sg).sessions, s);
        }
    }
}

/* Remove a session from its group and destroy the group if empty. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_remove(s: *mut session) {
    unsafe {
        let mut sg = session_group_contains(s);

        if sg.is_null() {
            return;
        }
        tailq_remove(&raw mut (*sg).sessions, s);
        if (tailq_empty(&raw mut (*sg).sessions)) {
            rb_remove(&raw mut session_groups, sg);
            free_((*sg).name.cast_mut());
            free_(sg);
        }
    }
}

/* Count number of sessions in session group. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_count(sg: *mut session_group) -> u32 {
    unsafe { tailq_foreach(&raw mut (*sg).sessions).count() as u32 }
}

/* Count number of clients attached to sessions in session group. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_attached_count(sg: *mut session_group) -> u32 {
    unsafe {
        tailq_foreach(&raw mut (*sg).sessions)
            .map(|s| (*s.as_ptr()).attached)
            .sum()
    }
}

/// Synchronize a session to its session group.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_synchronize_to(s: *mut session) {
    unsafe {
        let mut sg = session_group_contains(s);
        if sg.is_null() {
            return;
        }

        let mut target = null_mut();
        for target_ in tailq_foreach(&raw mut (*sg).sessions).map(|e| e.as_ptr()) {
            target = target_;
            if target != s {
                break;
            }
        }
        if !target.is_null() {
            session_group_synchronize1(target, s);
        }
    }
}

/* Synchronize a session group to a session. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_synchronize_from(target: *mut session) {
    unsafe {
        let mut sg = session_group_contains(target);
        if sg.is_null() {
            return;
        }

        for s in tailq_foreach(&raw mut (*sg).sessions).map(|e| e.as_ptr()) {
            if s != target {
                session_group_synchronize1(target, s);
            }
        }
    }
}

/*
 * Synchronize a session with a target session. This means destroying all
 * winlinks then recreating them, then updating the current window, last window
 * stack and alerts.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_group_synchronize1(target: *mut session, s: *mut session) {
    let mut old_windows = MaybeUninit::<winlinks>::uninit();
    let mut old_lastw = MaybeUninit::<winlink_stack>::uninit();

    unsafe {
        /* Don't do anything if the session is empty (it'll be destroyed). */
        let mut ww: *mut winlinks = &raw mut (*target).windows;
        if rb_empty(ww) {
            return;
        }

        /* If the current window has vanished, move to the next now. */
        if !(*s).curw.is_null()
            && winlink_find_by_index(ww, (*(*s).curw).idx).is_null()
            && session_last(s) != 0
            && session_previous(s, 0) != 0
        {
            session_next(s, 0);
        }

        /* Save the old pointer and reset it. */
        memcpy__(old_windows.as_mut_ptr(), &raw mut (*s).windows);
        rb_init(&raw mut (*s).windows);

        /* Link all the windows from the target. */
        for wl in rb_foreach(ww).map(|e| e.as_ptr()) {
            let wl2 = winlink_add(&raw mut (*s).windows, (*wl).idx);
            (*wl2).session = s;
            winlink_set_window(wl2, (*wl).window);
            notify_session_window(c"window-linked".as_ptr(), s, (*wl2).window);
            (*wl2).flags |= (*wl).flags & WINLINK_ALERTFLAGS;
        }

        /* Fix up the current window. */
        if !(*s).curw.is_null() {
            (*s).curw = winlink_find_by_index(&raw mut (*s).windows, (*(*s).curw).idx);
        } else {
            (*s).curw = winlink_find_by_index(&raw mut (*s).windows, (*(*target).curw).idx);
        }

        /* Fix up the last window stack. */
        memcpy__(old_lastw.as_mut_ptr(), &raw mut (*s).lastw);
        tailq_init(&raw mut (*s).lastw);

        for wl in tailq_foreach::<_, discr_sentry>(old_lastw.as_mut_ptr()).map(|e| e.as_ptr()) {
            if let Some(wl2) = NonNull::new(winlink_find_by_index(&raw mut (*s).windows, (*wl).idx))
            {
                tailq_insert_tail::<_, discr_sentry>(&raw mut (*s).lastw, wl2.as_ptr());
                (*wl2.as_ptr()).flags |= WINLINK_VISITED;
            }
        }

        /* Then free the old winlinks list. */
        while !rb_empty(old_windows.as_mut_ptr()) {
            let wl = rb_root(old_windows.as_mut_ptr());
            let wl2 = winlink_find_by_window_id(&raw mut (*s).windows, (*(*wl).window).id);
            if wl2.is_null() {
                notify_session_window(c"window-unlinked".as_ptr(), s, (*wl).window);
            }
            winlink_remove(old_windows.as_mut_ptr(), wl);
        }
    }
}

/// Renumber the windows across winlinks attached to a specific session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn session_renumber_windows(s: *mut session) {
    unsafe {
        // struct winlink *wl, *wl1, *wl_new;
        // struct winlinks old_wins;
        let mut old_wins = MaybeUninit::<winlinks>::uninit();
        let mut old_lastw = MaybeUninit::<winlink_stack>::uninit();
        // struct winlink_stack old_lastw;
        // int new_idx, new_curw_idx, marked_idx = -1;
        let mut marked_idx = -1;

        /* Save and replace old window list. */
        memcpy__(old_wins.as_mut_ptr(), &raw mut (*s).windows);
        rb_init(&raw mut (*s).windows);

        /* Start renumbering from the base-index if it's set. */
        let mut new_idx = options_get_number((*s).options, c"base-index".as_ptr()) as i32;
        let mut new_curw_idx = 0;

        /* Go through the winlinks and assign new indexes. */
        for wl in rb_foreach(old_wins.as_mut_ptr()).map(|e| e.as_ptr()) {
            let wl_new = winlink_add(&raw mut (*s).windows, new_idx);
            (*wl_new).session = s;
            winlink_set_window(wl_new, (*wl).window);
            (*wl_new).flags |= (*wl).flags & WINLINK_ALERTFLAGS;

            if wl == marked_pane.wl {
                marked_idx = (*wl_new).idx;
            }
            if wl == (*s).curw {
                new_curw_idx = (*wl_new).idx;
            }

            new_idx += 1;
        }

        /// Fix the stack of last windows now.
        memcpy__(old_lastw.as_mut_ptr(), &raw mut (*s).lastw);
        tailq_init(&raw mut (*s).lastw);
        for wl in tailq_foreach::<_, discr_sentry>(old_lastw.as_mut_ptr()).map(|e| e.as_ptr()) {
            (*wl).flags &= !WINLINK_VISITED;

            if let Some(wl_new) = winlink_find_by_window(&raw mut (*s).windows, (*wl).window) {
                tailq_insert_tail::<_, discr_sentry>(&raw mut (*s).lastw, wl_new.as_ptr());
                (*wl_new.as_ptr()).flags |= WINLINK_VISITED;
            }
        }

        /* Set the current window. */
        if (marked_idx != -1) {
            marked_pane.wl = winlink_find_by_index(&raw mut (*s).windows, marked_idx);
            if marked_pane.wl.is_null() {
                server_clear_marked();
            }
        }
        (*s).curw = winlink_find_by_index(&raw mut (*s).windows, new_curw_idx);

        // Free the old winlinks (reducing window references too).
        for wl in rb_foreach(old_wins.as_mut_ptr()).map(|e| e.as_ptr()) {
            winlink_remove(old_wins.as_mut_ptr(), wl);
        }
    }
}
