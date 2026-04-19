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
use std::time::Duration;

use crate::*;
use crate::options_::*;

pub static mut SESSIONS: sessions = BTreeMap::new();

/// Central registry owning all session allocations. `Box<session>` provides a
/// stable heap address, so `*mut session` pointers derived from it remain valid
/// for the lifetime of the registry entry.
pub static mut SESSION_REGISTRY: BTreeMap<SessionId, Box<session>> = BTreeMap::new();

pub static NEXT_SESSION_ID: AtomicU32 = AtomicU32::new(0);

pub static mut SESSION_GROUPS: session_groups = BTreeMap::new();

/// Iterate over all **alive** sessions as `*mut session` pointers.
///
/// Uses the `SESSIONS` name index (not `SESSION_REGISTRY`), because
/// destroyed sessions remain in the registry until their reference count
/// drains. Callers expect to only see live, usable sessions.
#[inline]
pub unsafe fn sessions_iter() -> impl Iterator<Item = *mut session> {
    unsafe {
        (*(&raw mut SESSIONS))
            .values()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// Look up a session by ID in the global registry.
pub unsafe fn session_from_id(id: SessionId) -> Option<*mut session> {
    unsafe {
        (*(&raw mut SESSION_REGISTRY))
            .get_mut(&id)
            .map(|b| &mut **b as *mut session)
    }
}

/// Look up a session by ID and return a shared reference, suitable for
/// read-only access. See `client_ref` for the rationale and aliasing
/// caveats — same convention applies.
#[expect(dead_code, reason = "Phase 2.4 hook; used opportunistically going forward")]
pub unsafe fn session_ref(id: SessionId) -> Option<&'static session> {
    unsafe {
        (*(&raw const SESSION_REGISTRY))
            .get(&id)
            .map(|b| &**b as &session)
    }
}

/// Check whether a session is still alive (not yet destroyed).
///
/// Uses the `SESSIONS` name index, not `SESSION_REGISTRY`, because a destroyed
/// session remains in the registry until its reference count reaches zero and
/// `session_free` runs.
pub unsafe fn session_alive(s: *mut session) -> bool {
    unsafe { (*(&raw mut SESSIONS)).values().any(|&s_ptr| s_ptr == s) }
}

/// Find session by name.
pub unsafe fn session_find(name: &str) -> *mut session {
    unsafe { (*(&raw mut SESSIONS)).get(name).copied().unwrap_or(null_mut()) }
}

/// Find session by id parsed from a string.
pub unsafe fn session_find_by_id_str(s: &str) -> *mut session {
    unsafe {
        if !s.starts_with('$') {
            return null_mut();
        }

        let Ok(id) = strtonum_(&s[1..], 0, u32::MAX) else {
            return null_mut();
        };
        transmute_ptr(session_find_by_id(id))
    }
}

/// Find session by id. O(log n) via `SESSION_REGISTRY`.
pub unsafe fn session_find_by_id(id: u32) -> Option<NonNull<session>> {
    unsafe { session_from_id(SessionId(id)).and_then(NonNull::new) }
}

impl session {
    unsafe fn create(
        prefix: *const u8,
        name: Option<&str>,
        cwd: *const u8,
        env: *mut Environ,
        oo: *mut options,
        tio: *mut termios,
    ) -> Box<Self> {
        unsafe {
            let mut s: Box<session> = Box::new(MaybeUninit::<session>::zeroed().assume_init_read());
            s.references = 1;
            s.flags = 0;

            // xcalloc'd zero bytes are NOT a guaranteed-valid Option<PathBuf>::None,
            // so initialize via ptr::write before any read.
            std::ptr::write(
                &raw mut s.cwd,
                Some(PathBuf::from(
                    std::ffi::CStr::from_ptr(cwd as *const i8)
                        .to_string_lossy()
                        .into_owned(),
                )),
            );

            std::ptr::write(&raw mut s.lock_timer, None);
            std::ptr::write(&raw mut s.lastw, Vec::new());
            std::ptr::write(&raw mut s.windows, BTreeMap::new());

            std::ptr::write(&raw mut s.environ, Box::from_raw(env));
            s.options = oo;

            status_update_cache(s.as_mut());

            std::ptr::write(&raw mut s.tio,
                if !tio.is_null() { Some(Box::new(*tio)) } else { None });


            if let Some(name) = name {
                s.name = name.to_string().into();
                s.id = NEXT_SESSION_ID.fetch_add(1, atomic::Ordering::Relaxed);
            } else {
                loop {
                    s.id = NEXT_SESSION_ID.fetch_add(1, atomic::Ordering::Relaxed);
                    s.name = if !prefix.is_null() {
                        format!("{}-{}", _s(prefix), s.id).into()
                    } else {
                        format!("{}", s.id).into()
                    };

                    if !(*(&raw mut SESSIONS)).contains_key(&*s.name) {
                        break;
                    }
                }
            }
            (*(&raw mut SESSIONS)).insert(s.name.to_string(), s.as_mut());

            log_debug!("new session {} ${}", s.name, s.id);

            if libc::gettimeofday(&raw mut s.creation_time, null_mut()) != 0 {
                fatal("gettimeofday failed");
            }
            session_update_activity(s.as_mut(), &raw mut s.creation_time);

            s
        }
    }
}

/// Create a new session.
///
/// Allocates the session in `SESSION_REGISTRY`. The `Box` in the registry
/// provides a stable heap address, so the returned `*mut session` is valid
/// for the lifetime of the registry entry.
pub unsafe fn session_create(
    prefix: *const u8,
    name: Option<&str>,
    cwd: *const u8,
    env: *mut Environ,
    oo: *mut options,
    tio: *mut termios,
) -> *mut session {
    unsafe {
        let boxed = session::create(prefix, name, cwd, env, oo, tio);
        let id = SessionId(boxed.id);
        let s = (*(&raw mut SESSION_REGISTRY)).entry(id).or_insert(boxed);
        &mut **s as *mut session
    }
}

/// Add a reference to a session.
pub unsafe fn session_add_ref(s: *mut session, from: *const u8) {
    let __func__ = "session_add_ref";
    unsafe {
        (*s).references += 1;
        log_debug!(
            "{}: {} {}, now {}",
            __func__,
            (*s).name,
            _s(from),
            (*s).references
        );
    }
}

/// Remove a reference from a session.
pub unsafe fn session_remove_ref(s: *mut session, from: *const u8) {
    let __func__ = "session_remove_ref";
    unsafe {
        (*s).references -= 1;
        log_debug!(
            "{}: {} {}, now {}",
            __func__,
            (*s).name,
            _s(from),
            (*s).references
        );

        if (*s).references == 0 {
            let sid = SessionId((*s).id);
            defer(Box::new(move || session_free_deferred(sid)));
        }
    }
}

/// Free session.
///
/// Removes the session from `SESSION_REGISTRY`, which drops the `Box<session>`
/// and deallocates the memory.
unsafe fn session_free_deferred(sid: SessionId) {
    unsafe {
        let Some(s) = session_from_id(sid) else { return };

        log_debug!(
            "session {} freed ({} references)",
            (*s).name,
            (*s).references
        );

        if (*s).references == 0 {
            // environ: Box<Environ> dropped automatically by registry removal
            options_free((*s).options);
            (*s).name = Cow::Borrowed("");
            // Drop the Box from the registry. Box drop runs Drop on all fields,
            // including `cwd: Option<PathBuf>`. (The Vec/BTreeMap fields were
            // already emptied in session_destroy; the Cow name was replaced above.)
            let _ = (*(&raw mut SESSION_REGISTRY)).remove(&sid);
        }
    }
}

/// Destroy a session.
pub unsafe fn session_destroy(s: *mut session, notify: i32, from: *const u8) {
    let __func__ = c!("session_destroy");
    unsafe {
        log_debug!("session {} destroyed ({})", (*s).name, _s(from));

        if (*s).curw.is_null() {
            return;
        }
        (*s).curw = null_mut();

        (*(&raw mut SESSIONS)).remove(&*(*s).name);
        if notify != 0 {
            notify_session(c"session-closed", s);
        }

        (*s).tio = None; // Box<termios> dropped

        // Drop the timer handle to deregister from the event loop.
        (*s).lock_timer = None;

        session_group_remove(s);

        while let Some(&wl) = (*s).lastw.first() {
            winlink_stack_remove(&raw mut (*s).lastw, wl);
        }
        while let Some(&wl) = (*(&raw mut (*s).windows)).values().next() {
            let w = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            notify_session_window(c"window-unlinked", s, w);
            winlink_remove(&raw mut (*s).windows, wl);
        }

        // `cwd` is `Option<PathBuf>`, dropped automatically by Box drop in session_free.

        session_remove_ref(s, __func__);
    }
}

/// Sanitize session name.
pub unsafe fn session_check_name(name: *const u8) -> Option<String> {
    unsafe {
        if *name == b'\0' {
            return None;
        }
        let copy = xstrdup(name).as_ptr();
        let mut cp = copy;
        while *cp != b'\0' {
            if *cp == b':' || *cp == b'.' {
                *cp = b'_';
            }
            cp = cp.add(1);
        }
        let new_name = utf8_stravis_(
            CStr::from_ptr(copy.cast()).to_bytes(),
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        free_(copy);
        Some(String::from_utf8(new_name).unwrap())
    }
}

/// Update activity time.
pub unsafe fn session_update_activity(s: *mut session, from: *mut timeval) {
    unsafe {
        let last = &raw mut (*s).last_activity_time;

        memcpy__(last, &raw mut (*s).activity_time);
        if from.is_null() {
            libc::gettimeofday(&raw mut (*s).activity_time, null_mut());
        } else {
            memcpy__(&raw mut (*s).activity_time, from);
        }

        log_debug!(
            "session ${} {} activity {}.{:06} (last {}.{:06})",
            (*s).id,
            (*s).name,
            (*s).activity_time.tv_sec,
            (*s).activity_time.tv_usec,
            (*last).tv_sec,
            (*last).tv_usec,
        );

        // Cancel any existing lock timer.
        (*s).lock_timer = None;

        if (*s).attached != 0 {
            let lock_after = options_get_number_((*s).options, "lock-after-time");
            if lock_after != 0 {
                let sid = SessionId((*s).id);
                (*s).lock_timer = timer_add(
                    Duration::from_secs(lock_after as u64),
                    Box::new(move || {
                        let Some(s) = session_from_id(sid) else { return };
                        if (*s).attached == 0 {
                            return;
                        }
                        log_debug!(
                            "session {} locked, activity time {}",
                            (*s).name,
                            (*s).activity_time.tv_sec,
                        );
                        server_lock_session(s);
                        recalculate_sizes();
                    }),
                );
            }
        }
    }
}

/// Find the next usable session.
pub unsafe fn session_next_session(s: *mut session) -> *mut session {
    unsafe {
        let sessions = &*(&raw mut SESSIONS);
        if sessions.is_empty() || !session_alive(s) {
            return null_mut();
        }

        let name = &*(*s).name;
        // Find the next session after this one in sorted order, wrapping around.
        let s2 = sessions
            .range::<str, _>((std::ops::Bound::Excluded(name), std::ops::Bound::Unbounded))
            .next().map_or_else(|| *sessions.values().next().unwrap(), |(_, &v)| v);
        if s2 == s {
            return null_mut();
        }
        s2
    }
}

/// Find the previous usable session.
pub unsafe fn session_previous_session(s: *mut session) -> *mut session {
    unsafe {
        let sessions = &*(&raw mut SESSIONS);
        if sessions.is_empty() || !session_alive(s) {
            return null_mut();
        }

        let name = &*(*s).name;
        // Find the previous session before this one in sorted order, wrapping around.
        let s2 = sessions
            .range::<str, _>((std::ops::Bound::Unbounded, std::ops::Bound::Excluded(name)))
            .next_back().map_or_else(|| *sessions.values().next_back().unwrap(), |(_, &v)| v);
        if s2 == s {
            return null_mut();
        }
        s2
    }
}

/// Attach a window to a session.
pub unsafe fn session_attach(
    s: *mut session,
    w: *mut window,
    idx: i32,
) -> Result<*mut winlink, String> {
    unsafe {
        let wl = winlink_add(&raw mut (*s).windows, idx);

        if wl.is_null() {
            return Err(format!("index in use: {idx}"));
        }
        (*wl).session = Some(SessionId((*s).id));
        winlink_set_window(wl, w);
        notify_session_window(c"window-linked", s, w);

        session_group_synchronize_from(s);
        Ok(wl)
    }
}

/// Detach a window from a session.
pub unsafe fn session_detach(s: *mut session, wl: *mut winlink) -> i32 {
    unsafe {
        if (*s).curw == wl && session_last(s) != 0 && session_previous(s, false) != 0 {
            session_next(s, false);
        }

        (*wl).flags &= !WINLINK_ALERTFLAGS;
        let w_unlink = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
        notify_session_window(c"window-unlinked", s, w_unlink);
        winlink_stack_remove(&raw mut (*s).lastw, wl);
        winlink_remove(&raw mut (*s).windows, wl);

        session_group_synchronize_from(s);

        if (*(&raw mut (*s).windows)).is_empty() {
            return 1;
        }
        0
    }
}

/// Return if session has window.
///
/// `s` may be null (e.g. when called with `client_get_session()` for an
/// unattached client) — in that case the answer is always `false`.
pub unsafe fn session_has(s: *mut session, w: &window) -> bool {
    if s.is_null() {
        return false;
    }
    let target = Some(SessionId(unsafe { (*s).id }));
    w.winlinks.iter()
        .any(|&wl| unsafe { (*wl).session } == target)
}

/// Return 1 if a window is linked outside this session (not including session groups). The window must be in this session!
pub unsafe fn session_is_linked(s: *mut session, w: &window) -> bool {
    unsafe {
        let sg = session_group_contains(s);
        if !sg.is_null() {
            return w.references != session_group_count(&*sg);
        }
        w.references != 1
    }
}

pub unsafe fn session_next_alert(mut wl: *mut winlink, s: *mut session) -> *mut winlink {
    unsafe {
        while !wl.is_null() {
            if (*wl).flags.intersects(WINLINK_ALERTFLAGS) {
                break;
            }
            wl = winlink_next(&raw mut (*s).windows, wl);
        }
    }
    wl
}

/// Move session to next window.
pub unsafe fn session_next(s: *mut session, alert: bool) -> i32 {
    unsafe {
        if (*s).curw.is_null() {
            return -1;
        }

        let mut wl = winlink_next(&raw mut (*s).windows, (*s).curw);
        if alert {
            wl = session_next_alert(wl, s);
        }
        if wl.is_null() {
            wl = (*(&raw mut (*s).windows)).values().next().copied().unwrap_or(null_mut());
            if alert
                && ({
                    (wl = session_next_alert(wl, s));
                    wl.is_null()
                })
            {
                return -1;
            }
        }
        session_set_current(s, wl)
    }
}

pub unsafe fn session_previous_alert(mut wl: *mut winlink, s: *mut session) -> *mut winlink {
    unsafe {
        while !wl.is_null() {
            if (*wl).flags.intersects(WINLINK_ALERTFLAGS) {
                break;
            }
            wl = winlink_previous(&raw mut (*s).windows, wl);
        }
        wl
    }
}

/// Move session to previous window.
pub unsafe fn session_previous(s: *mut session, alert: bool) -> i32 {
    unsafe {
        if (*s).curw.is_null() {
            return -1;
        }

        let mut wl = winlink_previous(&raw mut (*s).windows, (*s).curw);
        if alert {
            wl = session_previous_alert(wl, s);
        }
        if wl.is_null() {
            wl = (*(&raw mut (*s).windows)).values().next_back().copied().unwrap_or(null_mut());
            if alert
                && ({
                    (wl = session_previous_alert(wl, s));
                    wl.is_null()
                })
            {
                return -1;
            }
        }
        session_set_current(s, wl)
    }
}

/// Move session to specific window.
pub unsafe fn session_select(s: *mut session, idx: i32) -> i32 {
    unsafe {
        let wl = winlink_find_by_index(&raw mut (*s).windows, idx);
        session_set_current(s, wl)
    }
}

/// Move session to last used window.
pub unsafe fn session_last(s: *mut session) -> i32 {
    unsafe {
        let wl = (*s).lastw.first().copied().unwrap_or(null_mut());
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
pub unsafe fn session_set_current(s: *mut session, wl: *mut winlink) -> i32 {
    unsafe {
        let old: *mut winlink = (*s).curw;

        if wl.is_null() {
            return -1;
        }
        if wl == (*s).curw {
            return 1;
        }

        winlink_stack_remove(&raw mut (*s).lastw, wl);
        winlink_stack_push(&raw mut (*s).lastw, (*s).curw);
        (*s).curw = wl;
        let w_cur = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
        if options_get_number_(GLOBAL_OPTIONS, "focus-events") != 0 {
            if !old.is_null() {
                let w_old = (*old).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
                window_update_focus(w_old);
            }
            window_update_focus(w_cur);
        }
        winlink_clear_flags(wl);
        window_update_activity(NonNull::new_unchecked(w_cur));
        tty_update_window_offset(w_cur);
        notify_session(c"session-window-changed", s);
        0
    }
}

/// Find the session group containing a session.
pub unsafe fn session_group_contains(target: *mut session) -> *mut session_group {
    unsafe {
        for sg in (*(&raw mut SESSION_GROUPS)).values_mut() {
            for &s in &sg.sessions {
                if s == target {
                    return &mut **sg as *mut session_group;
                }
            }
        }

        null_mut()
    }
}

/// Find session group by name.
pub unsafe fn session_group_find(name: &str) -> *mut session_group {
    unsafe {
        (*(&raw mut SESSION_GROUPS))
            .get_mut(name)
            .map_or(null_mut(), |sg| &mut **sg as *mut session_group)
    }
}

/// Create a new session group.
pub unsafe fn session_group_new(name: &str) -> *mut session_group {
    unsafe {
        let sg = session_group_find(name);
        if !sg.is_null() {
            return sg;
        }

        let sg_box = Box::new(session_group {
            name: name.to_string().into(),
            sessions: Vec::new(),
        });

        let sg_groups = &mut *(&raw mut SESSION_GROUPS);
        sg_groups.insert(name.to_string(), sg_box);
        &mut **sg_groups.get_mut(name).unwrap() as *mut session_group
    }
}

/// Add a session to a session group.
pub unsafe fn session_group_add(sg: *mut session_group, s: *mut session) {
    unsafe {
        if session_group_contains(s).is_null() {
            (*sg).sessions.push(s);
        }
    }
}

/// Remove a session from its group and destroy the group if empty.
pub unsafe fn session_group_remove(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);

        if sg.is_null() {
            return;
        }
        (*sg).sessions.retain(|&p| p != s);
        if (*sg).sessions.is_empty() {
            let name = (*sg).name.to_string();
            (*(&raw mut SESSION_GROUPS)).remove(&name);
        }
    }
}

/// Count number of sessions in session group.
pub fn session_group_count(sg: &session_group) -> u32 {
    sg.sessions.len() as u32
}

/// Count number of clients attached to sessions in session group.
///
/// Sums `attached` across the group's sessions. The session pointers are
/// still raw, so the dereference stays inside an `unsafe` block.
pub fn session_group_attached_count(sg: &session_group) -> u32 {
    sg.sessions.iter()
        .map(|&s| unsafe { (*s).attached })
        .sum()
}

/// Synchronize a session to its session group.
pub unsafe fn session_group_synchronize_to(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            return;
        }

        let mut target = null_mut();
        for &target_ in &(*sg).sessions {
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

/// Synchronize a session group to a session.
pub unsafe fn session_group_synchronize_from(target: *mut session) {
    unsafe {
        let sg = session_group_contains(target);
        if sg.is_null() {
            return;
        }

        for &s in &(*sg).sessions {
            if s != target {
                session_group_synchronize1(target, s);
            }
        }
    }
}

// Synchronize a session with a target session. This means destroying all
// winlinks then recreating them, then updating the current window, last window
// stack and alerts.
pub unsafe fn session_group_synchronize1(target: *mut session, s: *mut session) {
    unsafe {
        // Don't do anything if the session is empty (it'll be destroyed).
        let ww: *mut winlinks = &raw mut (*target).windows;
        if (*ww).is_empty() {
            return;
        }

        // If the current window has vanished, move to the next now.
        if !(*s).curw.is_null()
            && winlink_find_by_index(ww, (*(*s).curw).idx).is_null()
            && session_last(s) != 0
            && session_previous(s, false) != 0
        {
            session_next(s, false);
        }

        // Save the old pointer and reset it.
        let mut old_windows = std::mem::take(&mut (*s).windows);

        // Link all the windows from the target.
        for &wl in (*ww).values() {
            let wl2 = winlink_add(&raw mut (*s).windows, (*wl).idx);
            (*wl2).session = Some(SessionId((*s).id));
            let w_src = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            winlink_set_window(wl2, w_src);
            let w_dst = (*wl2).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            notify_session_window(c"window-linked", s, w_dst);
            (*wl2).flags |= (*wl).flags & WINLINK_ALERTFLAGS;
        }

        // Fix up the current window.
        if !(*s).curw.is_null() {
            (*s).curw = winlink_find_by_index(&raw mut (*s).windows, (*(*s).curw).idx);
        } else {
            (*s).curw = winlink_find_by_index(&raw mut (*s).windows, (*(*target).curw).idx);
        }

        // Fix up the last window stack.
        let old_lastw = std::mem::take(&mut (*s).lastw);

        for &wl in &old_lastw {
            if let Some(wl2) = NonNull::new(winlink_find_by_index(&raw mut (*s).windows, (*wl).idx))
            {
                (*s).lastw.push(wl2.as_ptr());
                (*wl2.as_ptr()).flags |= winlink_flags::WINLINK_VISITED;
            }
        }

        // Then free the old winlinks list.
        while let Some(&wl) = old_windows.values().next() {
            let w_old = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            let wl2 = if w_old.is_null() { null_mut() } else { winlink_find_by_window_id(&raw mut (*s).windows, (*w_old).id) };
            if wl2.is_null() {
                notify_session_window(c"window-unlinked", s, w_old);
            }
            winlink_remove(&raw mut old_windows, wl);
        }
    }
}

/// Renumber the windows across winlinks attached to a specific session.
pub unsafe fn session_renumber_windows(s: *mut session) {
    unsafe {
        let mut marked_idx = -1;

        // Save and replace old window list.
        let mut old_wins = std::mem::take(&mut (*s).windows);

        // Start renumbering from the base-index if it's set.
        let mut new_idx = options_get_number_((*s).options, "base-index") as i32;
        let mut new_curw_idx = 0;

        // Go through the winlinks and assign new indexes.
        let old_values: Vec<*mut winlink> = old_wins.values().copied().collect();
        for wl in old_values.iter().copied() {
            let wl_new = winlink_add(&raw mut (*s).windows, new_idx);
            (*wl_new).session = Some(SessionId((*s).id));
            let w_src = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            winlink_set_window(wl_new, w_src);
            (*wl_new).flags |= (*wl).flags & WINLINK_ALERTFLAGS;

            if wl == MARKED_PANE.wl {
                marked_idx = (*wl_new).idx;
            }
            if wl == (*s).curw {
                new_curw_idx = (*wl_new).idx;
            }

            new_idx += 1;
        }

        // Fix the stack of last windows now.
        let old_lastw = std::mem::take(&mut (*s).lastw);
        for &wl in &old_lastw {
            (*wl).flags &= !winlink_flags::WINLINK_VISITED;

            let w_lookup = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
            if let Some(wl_new) = winlink_find_by_window(&raw mut (*s).windows, w_lookup) {
                (*s).lastw.push(wl_new.as_ptr());
                (*wl_new.as_ptr()).flags |= winlink_flags::WINLINK_VISITED;
            }
        }

        // Set the current window.
        if marked_idx != -1 {
            MARKED_PANE.wl = winlink_find_by_index(&raw mut (*s).windows, marked_idx);
            if MARKED_PANE.wl.is_null() {
                server_clear_marked();
            }
        }
        (*s).curw = winlink_find_by_index(&raw mut (*s).windows, new_curw_idx);

        // Free the old winlinks (reducing window references too).
        for wl in old_values {
            winlink_remove(&raw mut old_wins, wl);
        }
    }
}
