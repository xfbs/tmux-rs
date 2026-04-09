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
use crate::compat::HOST_NAME_MAX;
use crate::libc::{
    FIONREAD, FNM_CASEFOLD, TIOCSWINSZ, close, fnmatch, free, gethostname, gettimeofday, ioctl,
    isspace, memset, regcomp, regex_t, regexec, regfree, strlen, winsize,
};
#[cfg(feature = "utempter")]
use crate::utempter::utempter_remove_record;
use crate::*;
use crate::options_::{options_create, options_free, options_get_number___, options_get_string_};

/// Default pixel cell sizes.
pub const DEFAULT_XPIXEL: u32 = 16;
pub const DEFAULT_YPIXEL: u32 = 32;

pub static mut WINDOWS: windows = BTreeMap::new();

/// Central registry owning all window allocations. `Box<window>` provides a
/// stable heap address, so `*mut window` pointers derived from it remain valid
/// for the lifetime of the registry entry.
pub static mut WINDOW_REGISTRY: BTreeMap<WindowId, Box<window>> = BTreeMap::new();

pub static NEXT_WINDOW_ID: AtomicU32 = AtomicU32::new(0);

pub static mut ALL_WINDOW_PANES: window_pane_tree = BTreeMap::new();

/// Central registry owning all window_pane allocations. `Box<window_pane>`
/// provides a stable heap address, so `*mut window_pane` pointers derived
/// from it remain valid for the lifetime of the registry entry.
pub static mut PANE_REGISTRY: BTreeMap<PaneId, Box<window_pane>> = BTreeMap::new();

pub static NEXT_WINDOW_PANE_ID: AtomicU32 = AtomicU32::new(0);

/// Iterate over all **alive** panes as `*mut window_pane` pointers.
///
/// Uses `ALL_WINDOW_PANES` (alive set), not `PANE_REGISTRY`, because
/// destroyed panes remain in the registry until reclaimed.
#[allow(dead_code, reason = "introduced for Phase 2.3.6 foundation; used in 2.3.7+")]
#[inline]
pub unsafe fn panes_iter() -> impl Iterator<Item = *mut window_pane> {
    unsafe {
        (*(&raw mut ALL_WINDOW_PANES))
            .values()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// Look up a window_pane by ID in the global registry.
///
/// Returns the registry-stable raw pointer, or `None` if no allocation exists
/// for that ID. Note: the returned pointer may refer to a destroyed pane that
/// hasn't yet been reclaimed — use `ALL_WINDOW_PANES` lookup if you need an
/// *alive* pane.
#[allow(dead_code, reason = "introduced for Phase 2.3.6 foundation; used in 2.3.7+")]
pub unsafe fn pane_from_id(id: PaneId) -> Option<*mut window_pane> {
    unsafe {
        (*(&raw mut PANE_REGISTRY))
            .get_mut(&id)
            .map(|b| &mut **b as *mut window_pane)
    }
}

/// Look up a pane by ID and return a shared reference, suitable for
/// read-only access. See `client_ref` for the rationale and aliasing
/// caveats — same convention applies.
#[allow(dead_code, reason = "Phase 2.4 hook; used opportunistically going forward")]
pub unsafe fn pane_ref(id: PaneId) -> Option<&'static window_pane> {
    unsafe {
        (*(&raw const PANE_REGISTRY))
            .get(&id)
            .map(|b| &**b as &window_pane)
    }
}

/// Iterate over all **alive** windows as `*mut window` pointers.
///
/// Uses the `WINDOWS` id index (not `WINDOW_REGISTRY`), because destroyed
/// windows remain in the registry until their reference count drains.
/// Callers expect to only see live, usable windows.
#[inline]
pub unsafe fn windows_iter() -> impl Iterator<Item = *mut window> {
    unsafe {
        (*(&raw mut WINDOWS))
            .values()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

/// Look up a window by ID in the global registry.
///
/// Returns the registry-stable raw pointer, or `None` if no allocation exists
/// for that ID. Note: the returned pointer may refer to a destroyed window
/// whose reference count has not yet drained — use `WINDOWS` lookup if you
/// need an *alive* window.
#[allow(dead_code, reason = "introduced for Phase 2.3.0 foundation; used in 2.3.1+")]
pub unsafe fn window_from_id(id: WindowId) -> Option<*mut window> {
    unsafe {
        (*(&raw mut WINDOW_REGISTRY))
            .get_mut(&id)
            .map(|b| &mut **b as *mut window)
    }
}

/// Look up a window by ID and return a shared reference, suitable for
/// read-only access. See `client_ref` for the rationale and aliasing
/// caveats — same convention applies.
#[allow(dead_code, reason = "Phase 2.4 hook; used opportunistically going forward")]
pub unsafe fn window_ref(id: WindowId) -> Option<&'static window> {
    unsafe {
        (*(&raw const WINDOW_REGISTRY))
            .get(&id)
            .map(|b| &**b as &window)
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_input_data {
    item: *mut cmdq_item,
    wp: u32,
    file: *mut client_file,
}

pub unsafe fn winlink_find_by_window(
    wwl: *mut winlinks,
    w: *mut window,
) -> Option<NonNull<winlink>> {
    unsafe {
        let target = if w.is_null() { None } else { Some(WindowId((*w).id)) };
        for &wl in (*wwl).values() {
            if (*wl).window == target {
                return NonNull::new(wl);
            }
        }
        None
    }
}

pub unsafe fn winlink_find_by_index(wwl: *mut winlinks, idx: i32) -> *mut winlink {
    unsafe {
        if idx < 0 {
            fatalx("bad index");
        }

        (*wwl).get(&idx).copied().unwrap_or(null_mut())
    }
}

pub unsafe fn winlink_find_by_window_id(wwl: *mut winlinks, id: u32) -> *mut winlink {
    unsafe {
        for &wl in (*wwl).values() {
            if (*wl).window == Some(WindowId(id)) {
                return wl;
            }
        }

        null_mut()
    }
}

unsafe fn winlink_next_index(wwl: *mut winlinks, idx: i32) -> i32 {
    let mut i = idx;

    loop {
        if unsafe { winlink_find_by_index(wwl, i).is_null() } {
            return i;
        }

        if i == i32::MAX {
            i = 0;
        } else {
            i += 1;
        }

        if i == idx {
            break;
        }
    }

    -1
}

pub unsafe fn winlink_count(wwl: *mut winlinks) -> u32 {
    unsafe { (*wwl).len() as u32 }
}

pub unsafe fn winlink_add(wwl: *mut winlinks, mut idx: i32) -> *mut winlink {
    unsafe {
        if idx < 0 {
            idx = winlink_next_index(wwl, -idx - 1);
            if idx == -1 {
                return null_mut();
            }
        } else if !winlink_find_by_index(wwl, idx).is_null() {
            return null_mut();
        }

        let wl: *mut winlink = xcalloc_::<winlink>(1).as_ptr();
        (*wl).idx = idx;
        (*wwl).insert(idx, wl);

        wl
    }
}

/// Resolve `winlink.window` (an `Option<WindowId>`) to the underlying
/// `*mut window`, or null if absent.
#[inline]
pub unsafe fn winlink_window(wl: *mut winlink) -> *mut window {
    unsafe {
        (*wl).window
            .and_then(|id| window_from_id(id))
            .unwrap_or(null_mut())
    }
}

/// Resolve a window_pane's owner window through the registry.
///
/// Returns null if the pane has no owner (None) or the owner's allocation
/// has been reclaimed. Most callers can assume non-null since panes are
/// always created against a window.
#[inline]
pub unsafe fn window_pane_window(wp: *mut window_pane) -> *mut window {
    unsafe {
        (*wp).window
            .and_then(|id| window_from_id(id))
            .unwrap_or(null_mut())
    }
}

/// Set a window_pane's owner window field from a `*mut window` (possibly null).
#[inline]
pub unsafe fn window_pane_set_window(wp: *mut window_pane, w: *mut window) {
    unsafe {
        (*wp).window = if w.is_null() { None } else { Some(WindowId((*w).id)) };
    }
}

/// Resolve a window's active pane through the registry.
///
/// Returns null if the window has no active pane (None) or the pane's
/// allocation has been reclaimed.
#[inline]
pub unsafe fn window_active_pane(w: *mut window) -> *mut window_pane {
    unsafe {
        (*w).active
            .and_then(|id| pane_from_id(id))
            .unwrap_or(null_mut())
    }
}

/// Set a window's active pane field from a `*mut window_pane` (possibly null).
#[inline]
pub unsafe fn window_set_active_pane_field(w: *mut window, wp: *mut window_pane) {
    unsafe {
        (*w).active = if wp.is_null() { None } else { Some(PaneId((*wp).id)) };
    }
}

/// Convert a `*mut window_pane` (possibly null) to `Option<PaneId>`.
#[inline]
pub unsafe fn pane_id_from_ptr(wp: *mut window_pane) -> Option<PaneId> {
    unsafe {
        if wp.is_null() { None } else { Some(PaneId((*wp).id)) }
    }
}

/// Resolve `Option<PaneId>` to a `*mut window_pane`, returning null if None or
/// the allocation has been reclaimed.
#[inline]
pub unsafe fn pane_ptr_from_id(id: Option<PaneId>) -> *mut window_pane {
    unsafe { id.and_then(|i| pane_from_id(i)).unwrap_or(null_mut()) }
}

/// Read the pane's `layout_cell` field as a raw pointer.
///
/// Phase 2.5 step 4 accessor: this currently just dereferences the
/// `*mut layout_cell` field directly, but its purpose is to centralize
/// the read so that step 4.5 can flip the field type to
/// `Option<LayoutCellId>` by changing only the accessor body. Callers
/// remain unchanged across the field flip.
#[inline]
pub unsafe fn pane_layout_cell(wp: *mut window_pane) -> *mut layout_cell {
    unsafe { (*wp).layout_cell }
}

/// Write the pane's `layout_cell` field from a raw pointer.
///
/// Phase 2.5 step 4 accessor — see [`pane_layout_cell`]. After the
/// field flip, this will resolve `lc` to a `LayoutCellId` via the
/// arena's `id_of_ptr`. For now it's a direct field write.
#[inline]
pub unsafe fn pane_set_layout_cell(wp: *mut window_pane, lc: *mut layout_cell) {
    unsafe { (*wp).layout_cell = lc }
}

pub unsafe fn winlink_set_window(wl: *mut winlink, w: *mut window) {
    unsafe {
        let prev = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
        if !prev.is_null() {
            (*prev).winlinks.retain(|&p| p != wl);
            window_remove_ref(prev, c!("winlink_set_window"));
        }
        (*w).winlinks.push(wl);
        (*wl).window = Some(WindowId((*w).id));
        window_add_ref(w, c!("winlink_set_window"));
    }
}

pub unsafe fn winlink_remove(wwl: *mut winlinks, wl: *mut winlink) {
    unsafe {
        let w = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());

        if !w.is_null() {
            (*w).winlinks.retain(|&p| p != wl);
            window_remove_ref(w, c!("winlink_remove"));
        }

        (*wwl).remove(&(*wl).idx);
        free(wl as _);
    }
}

pub unsafe fn winlink_next(wwl: *mut winlinks, wl: *mut winlink) -> *mut winlink {
    unsafe {
        (*wwl)
            .range((std::ops::Bound::Excluded((*wl).idx), std::ops::Bound::Unbounded))
            .next()
            .map(|(_, &v)| v)
            .unwrap_or(null_mut())
    }
}

pub unsafe fn winlink_previous(wwl: *mut winlinks, wl: *mut winlink) -> *mut winlink {
    unsafe {
        (*wwl)
            .range(..(*wl).idx)
            .next_back()
            .map(|(_, &v)| v)
            .unwrap_or(null_mut())
    }
}

pub unsafe fn winlink_next_by_number(
    mut wl: *mut winlink,
    s: *mut session,
    n: i32,
) -> *mut winlink {
    unsafe {
        for _ in 0..n {
            wl = winlink_next(&raw mut (*s).windows, wl);
            if wl.is_null() {
                wl = (*(&raw mut (*s).windows)).values().next().copied().unwrap_or(null_mut());
            }
        }
    }

    wl
}

pub unsafe fn winlink_previous_by_number(
    mut wl: *mut winlink,
    s: *mut session,
    n: i32,
) -> *mut winlink {
    unsafe {
        for _ in 0..n {
            wl = winlink_previous(&raw mut (*s).windows, wl);
            if wl.is_null() {
                wl = (*(&raw mut (*s).windows)).values().next_back().copied().unwrap_or(null_mut());
            }
        }
    }

    wl
}

pub unsafe fn winlink_stack_push(stack: *mut winlink_stack, wl: *mut winlink) {
    if wl.is_null() {
        return;
    }

    unsafe {
        winlink_stack_remove(stack, wl);
        (*stack).insert(0, wl);
        (*wl).flags |= winlink_flags::WINLINK_VISITED;
    }
}

pub unsafe fn winlink_stack_remove(stack: *mut winlink_stack, wl: *mut winlink) {
    unsafe {
        if !wl.is_null() && (*wl).flags.intersects(winlink_flags::WINLINK_VISITED) {
            (*stack).retain(|&p| p != wl);
            (*wl).flags &= !winlink_flags::WINLINK_VISITED;
        }
    }
}

pub unsafe fn window_find_by_id_str(s: &str) -> *mut window {
    unsafe {
        if !s.starts_with('@') {
            return null_mut();
        }

        let Ok(id) = strtonum_(&s[1..], 0, u32::MAX) else {
            return null_mut();
        };

        window_find_by_id(id)
    }
}

pub unsafe fn window_find_by_id(id: u32) -> *mut window {
    unsafe {
        (*(&raw mut WINDOWS))
            .get(&id)
            .copied()
            .unwrap_or(null_mut())
    }
}

pub unsafe fn window_update_activity(w: NonNull<window>) {
    unsafe {
        gettimeofday(&raw mut (*w.as_ptr()).activity_time, null_mut());
        alerts_queue(w, window_flag::ACTIVITY);
    }
}

/// Get the next pane in the window's pane list, or null if this is the last.
pub unsafe fn window_pane_next_in_list(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        let panes = &(*window_pane_window(wp)).panes;
        match panes.iter().position(|&p| p == wp) {
            Some(i) if i + 1 < panes.len() => panes[i + 1],
            _ => null_mut(),
        }
    }
}

/// Get the previous pane in the window's pane list, or null if this is the first.
pub unsafe fn window_pane_prev_in_list(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        let panes = &(*window_pane_window(wp)).panes;
        match panes.iter().position(|&p| p == wp) {
            Some(i) if i > 0 => panes[i - 1],
            _ => null_mut(),
        }
    }
}

/// Create a new window.
///
/// Allocates the window in `WINDOW_REGISTRY`. The `Box` in the registry
/// provides a stable heap address, so the returned `*mut window` is valid
/// for the lifetime of the registry entry.
pub unsafe fn window_create(sx: u32, sy: u32, mut xpixel: u32, mut ypixel: u32) -> *mut window {
    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL;
    }
    unsafe {
        let mut boxed: Box<window> = Box::new(MaybeUninit::<window>::zeroed().assume_init_read());
        let w: *mut window = &mut *boxed;

        // xcalloc'd zero bytes are NOT a guaranteed-valid `Option<String>::None`.
        // Write None explicitly so reads of `name` don't observe a "zeroed Some".
        std::ptr::write(&raw mut (*w).name, Some(String::new()));
        (*w).flags = window_flag::empty();

        std::ptr::write(&raw mut (*w).panes, Vec::new());
        std::ptr::write(&raw mut (*w).last_panes, Vec::new());
        (*w).active = None;

        (*w).lastlayout = -1;
        (*w).layout_root = null_mut();
        // xcalloc'd zero bytes are not a valid LayoutArena (the inner Vec
        // would be UB on first use). Initialize explicitly. The Box drop
        // in `window_destroy` will run LayoutArena's Drop, so no manual
        // teardown is needed.
        std::ptr::write(&raw mut (*w).layout, LayoutArena::new());

        (*w).sx = sx;
        (*w).sy = sy;
        (*w).manual_sx = sx;
        (*w).manual_sy = sy;
        (*w).xpixel = xpixel;
        (*w).ypixel = ypixel;

        (*w).options = options_create(GLOBAL_W_OPTIONS);

        (*w).references = 0;
        std::ptr::write(&raw mut (*w).winlinks, Vec::new());

        (*w).id = NEXT_WINDOW_ID.fetch_add(1, atomic::Ordering::Relaxed);
        let id = WindowId((*w).id);

        // Insert into the registry (owns the Box) and the alive set.
        let entry = (*(&raw mut WINDOW_REGISTRY)).entry(id).or_insert(boxed);
        let w_stable: *mut window = &mut **entry;
        (*(&raw mut WINDOWS)).insert((*w_stable).id, w_stable);

        window_set_fill_character(NonNull::new_unchecked(w_stable));
        window_update_activity(NonNull::new_unchecked(w_stable));

        log_debug!(
            "{}: @{} create {}x{} ({}x{})",
            "window_create",
            (*w_stable).id,
            sx,
            sy,
            (*w_stable).xpixel,
            (*w_stable).ypixel,
        );
        w_stable
    }
}

/// Free a window after its reference count drains.
///
/// Removes the window from `WINDOW_REGISTRY`, which drops the `Box<window>`
/// and reclaims the allocation. Field cleanup that the old `free()` path did
/// (panes, layout, options, events) is performed first while the box is still
/// in the registry.
unsafe fn window_destroy(w: *mut window) {
    unsafe {
        log_debug!(
            "window @{} destroyed ({} references)",
            (*w).id,
            (*w).references
        );

        window_unzoom(w, 0);
        (*(&raw mut WINDOWS)).remove(&(*w).id);

        if !(*w).layout_root.is_null() {
            layout_free_cell((*w).layout_root);
        }
        if !(*w).saved_layout_root.is_null() {
            layout_free_cell((*w).saved_layout_root);
        }
        free((*w).old_layout as _);

        window_destroy_panes(w);

        if event_initialized(&raw mut (*w).name_event) != 0 {
            event_del(&raw mut (*w).name_event);
        }

        if event_initialized(&raw mut (*w).alerts_timer) != 0 {
            event_del(&raw mut (*w).alerts_timer);
        }
        if event_initialized(&raw mut (*w).offset_timer) != 0 {
            event_del(&raw mut (*w).offset_timer);
        }

        options_free((*w).options);
        free((*w).fill_character as _);

        // `name` is `Option<String>` and is dropped automatically by `Box`'s drop
        // when we remove the entry from `WINDOW_REGISTRY` below. Don't manually
        // drop it here — that would double-free.

        // Drop the Box from the registry. This deallocates the window.
        let _ = (*(&raw mut WINDOW_REGISTRY)).remove(&WindowId((*w).id));
    }
}

pub unsafe fn window_pane_destroy_ready(wp: &window_pane) -> bool {
    let mut n = 0;
    unsafe {
        if wp.pipe_fd != -1 {
            if EVBUFFER_LENGTH((*wp.pipe_event).output) != 0 {
                return false;
            }
            if ioctl(wp.fd, FIONREAD, &raw mut n) != -1 && n > 0 {
                return false;
            }
        }

        if !wp.flags.intersects(window_pane_flags::PANE_EXITED) {
            return false;
        }
    }

    true
}

pub unsafe fn window_add_ref(w: *mut window, from: *const u8) {
    unsafe {
        (*w).references += 1;
        log_debug!(
            "{}: @{} {}, now {}",
            "window_add_ref",
            (*w).id,
            _s(from),
            (*w).references,
        );
    }
}

pub unsafe fn window_remove_ref(w: *mut window, from: *const u8) {
    unsafe {
        (*w).references -= 1;
        log_debug!(
            "{}: @{} {}, now {}",
            "window_remove_ref",
            (*w).id,
            _s(from),
            (*w).references,
        );

        if (*w).references == 0 {
            window_destroy(w);
        }
    }
}

pub unsafe fn window_set_name(w: *mut window, new_name: *const u8) {
    unsafe {
        let visited = utf8_stravis_(
            new_name,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        (*w).name = Some(String::from_utf8_lossy(&visited).into_owned());
        notify_window(c"window-renamed", w);
    }
}

pub unsafe fn window_resize(w: *mut window, sx: u32, sy: u32, mut xpixel: i32, mut ypixel: i32) {
    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL as i32;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL as i32;
    }

    unsafe {
        log_debug!(
            "{}: @{} resize {}x{} ({}x{})",
            "window_resize",
            (*w).id,
            sx,
            sy,
            if xpixel == -1 {
                (*w).xpixel
            } else {
                xpixel as u32
            },
            if ypixel == -1 {
                (*w).ypixel
            } else {
                ypixel as u32
            },
        );

        (*w).sx = sx;
        (*w).sy = sy;
        if xpixel != -1 {
            (*w).xpixel = xpixel as u32;
        }
        if ypixel != -1 {
            (*w).ypixel = ypixel as u32;
        }
    }
}

pub unsafe fn window_pane_send_resize(wp: *mut window_pane, sx: u32, sy: u32) {
    unsafe {
        let w = window_pane_window(wp);
        let mut ws: winsize = core::mem::zeroed();

        if (*wp).fd == -1 {
            return;
        }

        log_debug!(
            "{}: %%{} resize to {},{}",
            "window_pane_send_resize",
            (*wp).id,
            sx,
            sy,
        );

        memset(&raw mut ws as _, 0, size_of::<winsize>());

        ws.ws_col = sx as u16;
        ws.ws_row = sy as u16;
        ws.ws_xpixel = (*w).xpixel as u16 * ws.ws_col;
        ws.ws_ypixel = (*w).ypixel as u16 * ws.ws_row;

        // TODO sun ifdef

        if ioctl((*wp).fd, TIOCSWINSZ, &ws) == -1 {
            fatal("ioctl failed");
        }
    }
}

pub fn window_has_pane(w: &window, wp: *mut window_pane) -> bool {
    w.panes.iter().any(|&wp1| wp1 == wp)
}

pub unsafe fn window_update_focus(w: *mut window) {
    unsafe {
        if !w.is_null() {
            log_debug!("{}: @{}", "window_update_focus", (*w).id);
            window_pane_update_focus(window_active_pane(w));
        }
    }
}

pub unsafe fn window_pane_update_focus(wp: *mut window_pane) {
    unsafe {
        let mut focused = false;

        if !wp.is_null() && !(*wp).flags.intersects(window_pane_flags::PANE_EXITED) {
            if wp != window_active_pane(window_pane_window(wp)) {
                focused = false;
            } else {
                for c in clients_iter() {
                    if !client_get_session(c).is_null()
                        && (*client_get_session(c)).attached != 0
                        && (*c).flags.intersects(client_flag::FOCUSED)
                        && winlink_window((*client_get_session(c)).curw) == window_pane_window(wp)
                    {
                        focused = true;
                        break;
                    }
                }
            }
            if !focused && (*wp).flags.intersects(window_pane_flags::PANE_FOCUSED) {
                log_debug!("{}: %%{} focus out", "window_pane_update_focus", (*wp).id);
                if (*wp).base.mode.intersects(mode_flag::MODE_FOCUSON) {
                    bufferevent_write((*wp).event, c!("\x1b[O") as _, 3);
                }
                notify_pane(c"pane-focus-out", wp);
                (*wp).flags &= !window_pane_flags::PANE_FOCUSED;
            } else if focused && !(*wp).flags.intersects(window_pane_flags::PANE_FOCUSED) {
                log_debug!("{}: %%{} focus in", "window_pane_update_focus", (*wp).id);
                if (*wp).base.mode.intersects(mode_flag::MODE_FOCUSON) {
                    bufferevent_write((*wp).event, c!("\x1b[I") as _, 3);
                }
                notify_pane(c"pane-focus-in", wp);
                (*wp).flags |= window_pane_flags::PANE_FOCUSED;
            } else {
                log_debug!(
                    "{}: %%{} focus unchanged",
                    "window_pane_update_focus",
                    (*wp).id,
                );
            }
        }
    }
}

pub unsafe fn window_set_active_pane(w: *mut window, wp: *mut window_pane, notify: i32) -> i32 {
    static NEXT_ACTIVE_POINT: AtomicU32 = AtomicU32::new(0);

    let lastwp: *mut window_pane;
    unsafe {
        log_debug!("{}: pane %%{}", "window_set_active_pane", (*wp).id);

        if wp == window_active_pane(w) {
            return 0;
        }
        lastwp = window_active_pane(w);

        window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        window_pane_stack_push(&raw mut (*w).last_panes, lastwp);

        window_set_active_pane_field(w, wp);
        (*wp).active_point = NEXT_ACTIVE_POINT.fetch_add(1, atomic::Ordering::Relaxed);
        (*wp).flags |= window_pane_flags::PANE_CHANGED;

        if options_get_number___::<i64>(&*GLOBAL_OPTIONS, "focus-events") != 0 {
            window_pane_update_focus(lastwp);
            window_pane_update_focus(wp);
        }

        tty_update_window_offset(w);

        if notify != 0 {
            notify_window(c"window-pane-changed", w);
        }
    }
    1
}

fn window_pane_get_palette(wp: Option<&window_pane>, c: i32) -> i32 {
    if let Some(wp) = wp {
        colour_palette_get(Some(&wp.palette), c)
    } else {
        -1
    }
}

pub unsafe fn window_redraw_active_switch(w: *mut window, mut wp: *mut window_pane) {
    unsafe {
        if wp == window_active_pane(w) {
            return;
        }

        loop {
            // If the active and inactive styles or palettes are different,
            // need to redraw the panes.
            let gc1 = &raw mut (*wp).cached_gc;
            let gc2 = &raw mut (*wp).cached_active_gc;
            if grid_cells_look_equal(gc1, gc2) == 0 {
                (*wp).flags |= window_pane_flags::PANE_REDRAW;
            } else {
                let mut c1 = window_pane_get_palette(ptr_to_ref(wp), (*gc1).fg);
                let mut c2 = window_pane_get_palette(ptr_to_ref(wp), (*gc2).fg);
                if c1 != c2 {
                    (*wp).flags |= window_pane_flags::PANE_REDRAW;
                } else {
                    c1 = window_pane_get_palette(ptr_to_ref(wp), (*gc1).bg);
                    c2 = window_pane_get_palette(ptr_to_ref(wp), (*gc2).bg);
                    if c1 != c2 {
                        (*wp).flags |= window_pane_flags::PANE_REDRAW;
                    }
                }
            }
            if wp == window_active_pane(w) {
                break;
            }
            wp = window_active_pane(w);
        }
    }
}

pub unsafe fn window_get_active_at(w: &window, x: u32, y: u32) -> *mut window_pane {
    unsafe {
        for &wp in w.panes.iter() {
            if !window_pane_visible(wp) {
                continue;
            }
            if x < (*wp).xoff || x > (*wp).xoff + (*wp).sx {
                continue;
            }
            if y < (*wp).yoff || y > (*wp).yoff + (*wp).sy {
                continue;
            }
            return wp;
        }
        null_mut()
    }
}

pub unsafe fn window_find_string(w: &window, s: &str) -> *mut window_pane {
    unsafe {
        let mut top: u32 = 0;
        let mut bottom: u32 = w.sy - 1;

        let mut x = w.sx / 2;
        let mut y = w.sy / 2;

        let status: Result<pane_status, _> =
            options_get_number___::<i32>(&*w.options, "pane-border-status").try_into();
        match status {
            Ok(pane_status::PANE_STATUS_TOP) => top += 1,
            Ok(pane_status::PANE_STATUS_BOTTOM) => bottom -= 1,
            _ => (),
        }

        if s.eq_ignore_ascii_case("top") {
            y = top;
        } else if s.eq_ignore_ascii_case("bottom") {
            y = bottom;
        } else if s.eq_ignore_ascii_case("left") {
            x = 0;
        } else if s.eq_ignore_ascii_case("right") {
            x = w.sx - 1;
        } else if s.eq_ignore_ascii_case("top-left") {
            x = 0;
            y = top;
        } else if s.eq_ignore_ascii_case("top-right") {
            x = w.sx - 1;
            y = top;
        } else if s.eq_ignore_ascii_case("bottom-left") {
            x = 0;
            y = bottom;
        } else if s.eq_ignore_ascii_case("bottom-right") {
            x = w.sx - 1;
            y = bottom;
        } else {
            return null_mut();
        }

        window_get_active_at(w, x, y)
    }
}

pub unsafe fn window_zoom(wp: *mut window_pane) -> i32 {
    unsafe {
        let w = window_pane_window(wp);

        if (*w).flags.intersects(window_flag::ZOOMED) {
            return -1;
        }

        if window_count_panes(&*w) == 1 {
            return -1;
        }

        if window_active_pane(w) != wp {
            window_set_active_pane(w, wp, 1);
        }

        for &wp1 in (*w).panes.iter() {
            (*wp1).saved_layout_cell = pane_layout_cell(wp1);
            pane_set_layout_cell(wp1, null_mut());
        }

        (*w).saved_layout_root = (*w).layout_root;
        layout_init(w, wp);
        (*w).flags |= window_flag::ZOOMED;
        notify_window(c"window-layout-changed", w);

        0
    }
}

pub unsafe fn window_unzoom(w: *mut window, notify: i32) -> i32 {
    unsafe {
        if !(*w).flags.intersects(window_flag::ZOOMED) {
            return -1;
        }

        (*w).flags &= !window_flag::ZOOMED;
        layout_free(w);
        (*w).layout_root = (*w).saved_layout_root;
        (*w).saved_layout_root = null_mut();

        for &wp in (*w).panes.iter() {
            pane_set_layout_cell(wp, (*wp).saved_layout_cell);
            (*wp).saved_layout_cell = null_mut();
        }
        layout_fix_panes(&*w, null_mut());

        if notify != 0 {
            notify_window(c"window-layout-changed", w);
        }

        0
    }
}

pub unsafe fn window_push_zoom(w: *mut window, always: bool, flag: bool) -> bool {
    unsafe {
        log_debug!(
            "{}: @{} {}",
            "window_push_zoom",
            (*w).id,
            (flag && (*w).flags.intersects(window_flag::ZOOMED)) as i32,
        );
        if flag && (always || (*w).flags.intersects(window_flag::ZOOMED)) {
            (*w).flags |= window_flag::WASZOOMED;
        } else {
            (*w).flags &= !window_flag::WASZOOMED;
        }

        window_unzoom(w, 1) == 0
    }
}

pub unsafe fn window_pop_zoom(w: *mut window) -> bool {
    unsafe {
        log_debug!(
            "{}: @{} {}",
            "window_pop_zoom",
            (*w).id,
            (*w).flags.intersects(window_flag::WASZOOMED) as i32,
        );
        if (*w).flags.intersects(window_flag::WASZOOMED) {
            return window_zoom(window_active_pane(w)) == 0;
        }
    }

    false
}

pub unsafe fn window_add_pane(
    w: *mut window,
    mut other: *mut window_pane,
    hlimit: u32,
    flags: spawn_flags,
) -> *mut window_pane {
    let func = "window_add_pane";
    unsafe {
        if other.is_null() {
            other = window_active_pane(w);
        }

        let wp = window_pane_create(w, (*w).sx, (*w).sy, hlimit);
        if (*w).panes.is_empty() {
            log_debug!("{}: @{} at start", func, (*w).id);
            (*w).panes.insert(0, wp);
        } else if flags.intersects(SPAWN_BEFORE) {
            log_debug!("{}: @{} before %%{}", func, (*w).id, (*wp).id);
            if flags.intersects(SPAWN_FULLSIZE) {
                (*w).panes.insert(0, wp);
            } else {
                let pos = (*w).panes.iter().position(|&p| p == other).unwrap();
                (*w).panes.insert(pos, wp);
            }
        } else {
            log_debug!("{}: @{} after %%{}", func, (*w).id, (*wp).id);
            if flags.intersects(SPAWN_FULLSIZE) {
                (*w).panes.push(wp);
            } else {
                let pos = (*w).panes.iter().position(|&p| p == other).unwrap();
                (*w).panes.insert(pos + 1, wp);
            }
        }

        wp
    }
}

pub unsafe fn window_lost_pane(w: *mut window, wp: *mut window_pane) {
    unsafe {
        log_debug!("{}: @{} pane %%{}", "window_lost_pane", (*w).id, (*wp).id);

        if MARKED_PANE.wp == Some(PaneId((*wp).id)) {
            server_clear_marked();
        }

        window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        if wp == window_active_pane(w) {
            let mut new_active = (*w).last_panes.first().copied().unwrap_or(null_mut());
            if new_active.is_null() {
                new_active = window_pane_prev_in_list(wp);
                if new_active.is_null() {
                    new_active = window_pane_next_in_list(wp);
                }
            }
            window_set_active_pane_field(w, new_active);
            if !new_active.is_null() {
                window_pane_stack_remove(&raw mut (*w).last_panes, new_active);
                (*new_active).flags |= window_pane_flags::PANE_CHANGED;
                notify_window(c"window-pane-changed", w);
                window_update_focus(w);
            }
        }
    }
}

pub unsafe fn window_remove_pane(w: *mut window, wp: *mut window_pane) {
    unsafe {
        window_lost_pane(w, wp);

        (*w).panes.retain(|&p| p != wp);
        window_pane_destroy(wp);
    }
}

pub unsafe fn window_pane_at_index(w: &window, idx: u32) -> *mut window_pane {
    unsafe {
        let mut n: u32 = options_get_number___::<u32>(&*w.options, "pane-base-index");

        for &wp in w.panes.iter() {
            if n == idx {
                return wp;
            }
            n += 1;
        }

        null_mut()
    }
}

pub unsafe fn window_pane_next_by_number(
    w: *mut window,
    mut wp: *mut window_pane,
    n: u32,
) -> *mut window_pane {
    unsafe {
        for _ in 0..n {
            wp = window_pane_next_in_list(wp);
            if wp.is_null() {
                wp = (*w).panes.first().copied().unwrap_or(null_mut());
            }
        }
    }

    wp
}

pub unsafe fn window_pane_previous_by_number(
    w: *mut window,
    mut wp: *mut window_pane,
    n: u32,
) -> *mut window_pane {
    unsafe {
        for _ in 0..n {
            wp = window_pane_prev_in_list(wp);
            if wp.is_null() {
                wp = (*w).panes.last().copied().unwrap_or(null_mut());
            }
        }
    }

    wp
}

pub unsafe fn window_pane_index(wp: *mut window_pane, i: *mut u32) -> i32 {
    unsafe {
        let w = window_pane_window(wp);

        *i = options_get_number___::<u32>(&*(*w).options, "pane-base-index") as _;
        for &wq in (*w).panes.iter() {
            if wp == wq {
                return 0;
            }
            (*i) += 1;
        }
        -1
    }
}

pub fn window_count_panes(w: &window) -> u32 {
    w.panes.len() as u32
}

pub unsafe fn window_destroy_panes(w: *mut window) {
    unsafe {
        // Clear visited flags for all panes in last_panes stack
        for &wp in (*w).last_panes.iter() {
            (*wp).flags &= !window_pane_flags::PANE_VISITED;
        }
        (*w).last_panes.clear();

        while let Some(wp) = (*w).panes.pop() {
            window_pane_destroy(wp);
        }
    }
}

pub unsafe fn window_printable_flags(wl: *mut winlink, escape: i32) -> *const u8 {
    static mut FLAGS: [u8; 32] = [0; 32];

    unsafe {
        let s = (*wl).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut());

        let mut pos = 0;
        if (*wl).flags.intersects(winlink_flags::WINLINK_ACTIVITY) {
            FLAGS[pos] = b'#';
            pos += 1;
            if escape != 0 {
                FLAGS[pos] = b'#';
                pos += 1;
            }
        }
        if (*wl).flags.intersects(winlink_flags::WINLINK_BELL) {
            FLAGS[pos] = b'!';
            pos += 1;
        }
        if (*wl).flags.intersects(winlink_flags::WINLINK_SILENCE) {
            FLAGS[pos] = b'~';
            pos += 1;
        }
        if !s.is_null() && wl == (*s).curw {
            FLAGS[pos] = b'*';
            pos += 1;
        }
        if !s.is_null() && wl == (*s).lastw.first().copied().unwrap_or(null_mut()) {
            FLAGS[pos] = b'-';
            pos += 1;
        }
        if server_check_marked() && wl == MARKED_PANE.wl {
            FLAGS[pos] = b'M';
            pos += 1;
        }
        let w_zoom = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
        if !w_zoom.is_null() && (*w_zoom).flags.intersects(window_flag::ZOOMED) {
            FLAGS[pos] = b'Z';
            pos += 1;
        }
        FLAGS[pos] = b'\0';
        &raw mut FLAGS as *mut u8
    }
}

pub unsafe fn window_pane_find_by_id_str(s: &str) -> *mut window_pane {
    unsafe {
        if !s.starts_with('%') {
            return null_mut();
        }

        match strtonum_(&s[1..], 0, u32::MAX) {
            Ok(id) => window_pane_find_by_id(id),
            Err(_errstr) => null_mut(),
        }
    }
}

pub unsafe fn window_pane_find_by_id(id: u32) -> *mut window_pane {
    unsafe {
        (*(&raw mut ALL_WINDOW_PANES))
            .get(&id)
            .copied()
            .unwrap_or(null_mut())
    }
}

/// Create a new window_pane.
///
/// Allocates the pane in `PANE_REGISTRY`. The `Box` in the registry
/// provides a stable heap address, so the returned `*mut window_pane`
/// is valid for the lifetime of the registry entry.
pub unsafe fn window_pane_create(
    w: *mut window,
    sx: u32,
    sy: u32,
    hlimit: u32,
) -> *mut window_pane {
    unsafe {
        let mut host: [u8; HOST_NAME_MAX + 1] = zeroed();
        let mut boxed: Box<window_pane> =
            Box::new(MaybeUninit::<window_pane>::zeroed().assume_init_read());
        let wp: *mut window_pane = &mut *boxed;

        window_pane_set_window(wp, w);
        (*wp).options = options_create((*w).options);
        (*wp).flags = window_pane_flags::PANE_STYLECHANGED;

        (*wp).id = NEXT_WINDOW_PANE_ID.fetch_add(1, atomic::Ordering::Relaxed);
        let pid = PaneId((*wp).id);

        // Insert into the registry (owns the Box) and the alive set.
        let entry = (*(&raw mut PANE_REGISTRY)).entry(pid).or_insert(boxed);
        let wp: *mut window_pane = &mut **entry;
        (*(&raw mut ALL_WINDOW_PANES)).insert((*wp).id, wp);

        (*wp).fd = -1;

        // xcalloc'd zero bytes are NOT a guaranteed-valid `Option<PathBuf>::None`.
        // Write None explicitly so reads of cwd/shell don't observe a "zeroed Some".
        std::ptr::write(&raw mut (*wp).cwd, None);
        std::ptr::write(&raw mut (*wp).shell, None);
        std::ptr::write(&raw mut (*wp).searchstr, None);

        std::ptr::write(&raw mut (*wp).modes, Vec::new());

        (*wp).resize_queue = Vec::new();

        (*wp).sx = sx;
        (*wp).sy = sy;

        (*wp).pipe_fd = -1;

        (*wp).control_bg = -1;
        (*wp).control_fg = -1;

        (*wp).palette = colour_palette_init();
        colour_palette_from_option(Some(&mut (*wp).palette), (*wp).options);

        screen_init(&raw mut (*wp).base, sx, sy, hlimit);
        (*wp).screen = &raw mut (*wp).base;
        window_pane_default_cursor(wp);

        screen_init(&raw mut (*wp).status_screen, 1, 1, 0);

        if gethostname(host.as_mut_ptr(), size_of_val(&host)) == 0 {
            screen_set_title(&raw mut (*wp).base, host.as_ptr());
        }

        wp
    }
}

unsafe fn window_pane_destroy(wp: *mut window_pane) {
    unsafe {
        window_pane_reset_mode_all(wp);
        // `searchstr` is `Option<String>`, dropped automatically by Box drop.

        if (*wp).fd != -1 {
            #[cfg(feature = "utempter")]
            {
                utempter_remove_record((*wp).fd);
            }
            bufferevent_free((*wp).event);
            close((*wp).fd);
        }
        if !(*wp).ictx.is_null() {
            input_free((*wp).ictx);
        }

        screen_free(&raw mut (*wp).status_screen);

        screen_free(&raw mut (*wp).base);

        if (*wp).pipe_fd != -1 {
            bufferevent_free((*wp).pipe_event);
            close((*wp).pipe_fd);
        }

        if event_initialized(&raw mut (*wp).resize_timer) != 0 {
            event_del(&raw mut (*wp).resize_timer);
        }
        (*wp).resize_queue.clear();

        (*(&raw mut ALL_WINDOW_PANES)).remove(&(*wp).id);

        options_free((*wp).options);
        // `cwd` and `shell` are `Option<PathBuf>` and are dropped automatically
        // by Box drop when we remove from PANE_REGISTRY below.
        cmd_free_argv((*wp).argc, (*wp).argv);
        colour_palette_free(Some(&mut (*wp).palette));

        // Drop the Box from the registry. This deallocates the pane.
        let _ = (*(&raw mut PANE_REGISTRY)).remove(&PaneId((*wp).id));
    }
}

unsafe extern "C-unwind" fn window_pane_read_callback(_bufev: *mut bufferevent, data: *mut c_void) {
    unsafe {
        let wp: *mut window_pane = data as _;
        let evb: *mut evbuffer = (*(*wp).event).input;
        let wpo: *mut window_pane_offset = &raw mut (*wp).pipe_offset;
        let size = EVBUFFER_LENGTH(evb);
        let mut new_size: usize = 0;

        if (*wp).pipe_fd != -1 {
            let new_data = window_pane_get_new_data(wp, wpo, &raw mut new_size);
            if new_size > 0 {
                bufferevent_write((*wp).pipe_event, new_data, new_size);
                window_pane_update_used_data(wp, wpo, new_size);
            }
        }

        log_debug!("%%{} has {} bytes", (*wp).id, size);
        for c in clients_iter() {
            if !client_get_session(c).is_null() && (*c).flags.intersects(client_flag::CONTROL) {
                control_write_output(c, wp);
            }
        }
        input_parse_pane(wp);
        bufferevent_disable((*wp).event, EV_READ);
    }
}

unsafe extern "C-unwind" fn window_pane_error_callback(
    _bufev: *mut bufferevent,
    _what: c_short,
    data: *mut c_void,
) {
    let wp: *mut window_pane = data as _;
    unsafe {
        log_debug!("%%{} error", (*wp).id);
        (*wp).flags |= window_pane_flags::PANE_EXITED;

        if window_pane_destroy_ready(&*wp) {
            server_destroy_pane(wp, 1);
        }
    }
}

pub unsafe fn window_pane_set_event(wp: *mut window_pane) {
    unsafe {
        setblocking((*wp).fd, 0);

        (*wp).event = bufferevent_new(
            (*wp).fd,
            Some(window_pane_read_callback),
            None,
            Some(window_pane_error_callback),
            wp as _,
        );
        if (*wp).event.is_null() {
            fatalx("out of memory");
        }
        (*wp).ictx = input_init(wp, (*wp).event, &raw mut (*wp).palette);

        bufferevent_enable((*wp).event, EV_READ | EV_WRITE);
    }
}

pub unsafe fn window_pane_resize(wp: *mut window_pane, sx: u32, sy: u32) {
    unsafe {
        if sx == (*wp).sx && sy == (*wp).sy {
            return;
        }

        (*wp).resize_queue.push(window_pane_resize {
            sx,
            sy,
            osx: (*wp).sx,
            osy: (*wp).sy,
        });

        (*wp).sx = sx;
        (*wp).sy = sy;

        log_debug!(
            "{}: %%{} resize {}x{}",
            "window_pane_resize",
            (*wp).id,
            sx,
            sy,
        );
        screen_resize(
            &raw mut (*wp).base,
            sx,
            sy,
            (*wp).base.saved_grid.is_null() as i32,
        );

        if let Some(&wme) = (*wp).modes.first() {
            if let Some(wme) = NonNull::new(wme) {
                ((*(*wme.as_ptr()).mode).resize)(wme, sx, sy);
            }
        }
    }
}

pub unsafe fn window_pane_set_mode(
    wp: *mut window_pane,
    swp: *mut window_pane,
    mode: *const window_mode,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> i32 {
    unsafe {
        if !(*wp).modes.is_empty() && (*(&(*wp).modes)[0]).mode == mode {
            return 1;
        }

        let mut found_idx: Option<usize> = None;
        for (i, &wme) in (*wp).modes.iter().enumerate() {
            if (*wme).mode == mode {
                found_idx = Some(i);
                break;
            }
        }

        let wme;
        if let Some(idx) = found_idx {
            wme = (*wp).modes.remove(idx);
            (*wp).modes.insert(0, wme);
        } else {
            wme = xcalloc_::<window_mode_entry>(1).as_ptr();
            (*wme).wp = pane_id_from_ptr(wp);
            (*wme).swp = pane_id_from_ptr(swp);
            (*wme).mode = mode;
            (*wme).prefix = 1;
            (*wp).modes.insert(0, wme);
            (*wme).screen = ((*(*wme).mode).init)(NonNull::new_unchecked(wme), fs, args);
        }

        (*wp).screen = (*wme).screen;
        (*wp).flags |= window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_CHANGED;

        server_redraw_window_borders(window_pane_window(wp));
        server_status_window(window_pane_window(wp));
        notify_pane(c"pane-mode-changed", wp);

        0
    }
}

pub unsafe fn window_pane_reset_mode(wp: *mut window_pane) {
    let func = "window_pane_reset_mode";
    unsafe {
        if (*wp).modes.is_empty() {
            return;
        }

        let wme = (*wp).modes.remove(0);
        ((*(*wme).mode).free)(NonNull::new(wme).unwrap());
        free(wme as _);

        if let Some(&next) = (*wp).modes.first() {
            let next = NonNull::new(next).unwrap();
            log_debug!("{}: next mode is {}", func, (*(*next.as_ptr()).mode).name);
            (*wp).screen = (*next.as_ptr()).screen;
            ((*(*next.as_ptr()).mode).resize)(next, (*wp).sx, (*wp).sy);
        } else {
            (*wp).flags &= !window_pane_flags::PANE_UNSEENCHANGES;
            log_debug!("{}: no next mode", func);
            (*wp).screen = &raw mut (*wp).base;
        }
        (*wp).flags |= window_pane_flags::PANE_REDRAW | window_pane_flags::PANE_CHANGED;

        server_redraw_window_borders(window_pane_window(wp));
        server_status_window(window_pane_window(wp));
        notify_pane(c"pane-mode-changed", wp);
    }
}

pub unsafe fn window_pane_reset_mode_all(wp: *mut window_pane) {
    unsafe {
        while !(*wp).modes.is_empty() {
            window_pane_reset_mode(wp);
        }
    }
}

unsafe fn window_pane_copy_key(wp: *mut window_pane, key: key_code) {
    unsafe {
        for &loop_ in (*window_pane_window(wp)).panes.iter() {
            if loop_ != wp
                && (*loop_).modes.is_empty()
                && (*loop_).fd != -1
                && !(*loop_).flags.intersects(window_pane_flags::PANE_INPUTOFF)
                && window_pane_visible(loop_)
                && options_get_number___::<i64>(&*(*loop_).options, "synchronize-panes") != 0
            {
                input_key_pane(loop_, key, null_mut());
            }
        }
    }
}

pub unsafe fn window_pane_key(
    wp: *mut window_pane,
    c: *mut client,
    s: *mut session,
    wl: *mut winlink,
    mut key: key_code,
    m: *mut mouse_event,
) -> i32 {
    if KEYC_IS_MOUSE(key) && m.is_null() {
        return -1;
    }
    unsafe {
        if let Some(&wme) = (*wp).modes.first()
            && let Some(wme) = NonNull::new(wme)
            && let Some(key_fn) = (*(*wme.as_ptr()).mode).key
            && !c.is_null()
        {
            key &= !KEYC_MASK_FLAGS;
            key_fn(wme, c, s, wl, key, m);
            return 0;
        }

        if (*wp).fd == -1 || (*wp).flags.intersects(window_pane_flags::PANE_INPUTOFF) {
            return 0;
        }

        if input_key_pane(wp, key, m) != 0 {
            return -1;
        }

        if KEYC_IS_MOUSE(key) {
            return 0;
        }
        if options_get_number___::<i64>(&*(*wp).options, "synchronize-panes") != 0 {
            window_pane_copy_key(wp, key);
        }
    }

    0
}

pub unsafe fn window_pane_visible(wp: *const window_pane) -> bool {
    unsafe {
        if !(*window_pane_window(wp as *mut window_pane)).flags.intersects(window_flag::ZOOMED) {
            return true;
        }
        std::ptr::eq(wp, window_active_pane(window_pane_window(wp as *mut window_pane)))
    }
}

pub unsafe fn window_pane_exited(wp: *mut window_pane) -> bool {
    unsafe { (*wp).fd == -1 || (*wp).flags.intersects(window_pane_flags::PANE_EXITED) }
}

pub unsafe fn window_pane_search(
    wp: *mut window_pane,
    term: *const u8,
    regex: i32,
    ignore: i32,
) -> u32 {
    unsafe {
        let s: *mut screen = &raw mut (*wp).base;
        let mut r: regex_t = zeroed();
        let mut new: *mut u8 = null_mut();
        let mut flags = 0;

        if regex == 0 {
            if ignore != 0 {
                flags |= FNM_CASEFOLD;
            }
            new = format_nul!("*{}*", _s(term));
        } else {
            if ignore != 0 {
                flags |= REG_ICASE;
            }
            if regcomp(&raw mut r, term, flags | REG_EXTENDED) != 0 {
                return 0;
            }
        }

        let mut i = 0;
        for j in 0..screen_size_y(s) {
            i = j;

            let line = grid_view_string_cells((*s).grid, 0, i, screen_size_x(s));
            for n in (1..=strlen(line)).rev() {
                if isspace(line.add(n - 1) as c_uchar as c_int) == 0 {
                    break;
                }
                *line.add(n - 1) = b'\0' as _;
            }

            log_debug!("{}: {}", "window_pane_search", _s(line));
            let found = if regex == 0 {
                fnmatch(new, line, flags) == 0
            } else {
                regexec(&r, line, 0, null_mut(), 0) == 0
            };
            free(line as _);

            if found {
                break;
            }
        }

        if regex == 0 {
            free(new as _);
        } else {
            regfree(&raw mut r);
        }

        if i == screen_size_y(s) {
            return 0;
        }

        i + 1
    }
}

/// Get MRU pane from a list.
unsafe fn window_pane_choose_best(list: *mut *mut window_pane, size: u32) -> *mut window_pane {
    if size == 0 {
        return null_mut();
    }

    unsafe {
        let mut best = *list;
        for i in 1..size {
            let next = *list.add(i as usize);
            if (*next).active_point > (*best).active_point {
                best = next;
            }
        }
        best
    }
}

/// Find the pane directly above another. We build a list of those adjacent to top edge and then choose the best.
pub unsafe fn window_pane_find_up(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        if wp.is_null() {
            return null_mut();
        }
        let w = window_pane_window(wp);
        let status: pane_status = options_get_number___::<i32>(&*(*w).options, "pane-border-status")
            .try_into()
            .unwrap();

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).yoff;
        match status {
            pane_status::PANE_STATUS_TOP => {
                if edge == 1 {
                    edge = (*w).sy + 1;
                }
            }
            pane_status::PANE_STATUS_BOTTOM => {
                if edge == 0 {
                    edge = (*w).sy;
                }
            }
            _ => {
                if edge == 0 {
                    edge = (*w).sy + 1;
                }
            }
        }

        let left = (*wp).xoff;
        let right = (*wp).xoff + (*wp).sx;

        for &next in (*w).panes.iter() {
            if next == wp {
                continue;
            }
            if (*next).yoff + (*next).sy + 1 != edge {
                continue;
            }
            let end = (*next).xoff + (*next).sx - 1;

            let mut found = 0;
            #[expect(clippy::if_same_then_else)]
            if (*next).xoff < left && end > right {
                found = 1;
            } else if (*next).xoff >= left && (*next).xoff <= right {
                found = 1;
            } else if end >= left && end <= right {
                found = 1;
            }
            if found == 0 {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/// Find the pane directly below another.
pub unsafe fn window_pane_find_down(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        if wp.is_null() {
            return null_mut();
        }
        let w = window_pane_window(wp);
        let status: pane_status = options_get_number___::<i32>(&*(*w).options, "pane-border-status")
            .try_into()
            .unwrap();

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).yoff + (*wp).sy + 1;
        match status {
            pane_status::PANE_STATUS_TOP => {
                if edge >= (*w).sy {
                    edge = 1;
                }
            }
            pane_status::PANE_STATUS_BOTTOM => {
                if edge >= (*w).sy - 1 {
                    edge = 0;
                }
            }
            _ => {
                if edge >= (*w).sy {
                    edge = 0;
                }
            }
        }

        let left = (*wp).xoff;
        let right = (*wp).xoff + (*wp).sx;

        for &next in (*w).panes.iter() {
            if next == wp {
                continue;
            }
            if (*next).yoff != edge {
                continue;
            }
            let end = (*next).xoff + (*next).sx - 1;

            let mut found = 0;
            #[expect(clippy::if_same_then_else)]
            if (*next).xoff < left && end > right {
                found = 1;
            } else if (*next).xoff >= left && (*next).xoff <= right {
                found = 1;
            } else if end >= left && end <= right {
                found = 1;
            }
            if found == 0 {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/// Find the pane directly to the left of another.
pub unsafe fn window_pane_find_left(wp: *mut window_pane) -> *mut window_pane {
    if wp.is_null() {
        return null_mut();
    }
    unsafe {
        let w = window_pane_window(wp);

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).xoff;
        if edge == 0 {
            edge = (*w).sx + 1;
        }

        let top = (*wp).yoff;
        let bottom = (*wp).yoff + (*wp).sy;

        for &next in (*w).panes.iter() {
            if next == wp {
                continue;
            }
            if (*next).xoff + (*next).sx + 1 != edge {
                continue;
            }
            let end = (*next).yoff + (*next).sy - 1;

            let mut found = false;
            #[expect(clippy::if_same_then_else)]
            if (*next).yoff < top && end > bottom {
                found = true;
            } else if (*next).yoff >= top && (*next).yoff <= bottom {
                found = true;
            } else if end >= top && end <= bottom {
                found = true;
            }
            if !found {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/// Find the pane directly to the right of another.
pub unsafe fn window_pane_find_right(wp: *mut window_pane) -> *mut window_pane {
    if wp.is_null() {
        return null_mut();
    }
    unsafe {
        let w = window_pane_window(wp);

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).xoff + (*wp).sx + 1;
        if edge >= (*w).sx {
            edge = 0;
        }

        let top = (*wp).yoff;
        let bottom = (*wp).yoff + (*wp).sy;

        for &next in (*w).panes.iter() {
            if next == wp {
                continue;
            }
            if (*next).xoff != edge {
                continue;
            }
            let end = (*next).yoff + (*next).sy - 1;

            let mut found = false;
            #[expect(clippy::if_same_then_else)]
            if (*next).yoff < top && end > bottom {
                found = true;
            } else if (*next).yoff >= top && (*next).yoff <= bottom {
                found = true;
            } else if end >= top && end <= bottom {
                found = true;
            }
            if !found {
                continue;
            }
            list = xreallocarray_::<*mut window_pane>(list, size + 1).as_ptr();
            *list.add(size) = next;
            size += 1;
        }

        let best = window_pane_choose_best(list, size as _);
        free(list as _);
        best
    }
}

pub unsafe fn window_pane_stack_push(stack: *mut Vec<*mut window_pane>, wp: *mut window_pane) {
    unsafe {
        if !wp.is_null() {
            window_pane_stack_remove(stack, wp);
            (*stack).insert(0, wp);
            (*wp).flags |= window_pane_flags::PANE_VISITED;
        }
    }
}

pub unsafe fn window_pane_stack_remove(stack: *mut Vec<*mut window_pane>, wp: *mut window_pane) {
    unsafe {
        if !wp.is_null() && (*wp).flags.intersects(window_pane_flags::PANE_VISITED) {
            (*stack).retain(|&p| p != wp);
            (*wp).flags &= !window_pane_flags::PANE_VISITED;
        }
    }
}

/// Clear alert flags for a winlink
pub unsafe fn winlink_clear_flags(wl: *mut winlink) {
    unsafe {
        let w = (*wl).window.and_then(|id| window_from_id(id)).unwrap_or(null_mut());
        if w.is_null() { return; }
        (*w).flags &= !WINDOW_ALERTFLAGS;
        for &loop_ in (*w).winlinks.iter() {
            if (*loop_).flags.intersects(WINLINK_ALERTFLAGS) {
                (*loop_).flags &= !WINLINK_ALERTFLAGS;
                server_status_session((*loop_).session.and_then(|id| session_from_id(id)).unwrap_or(null_mut()));
            }
        }
    }
}

/// Shuffle window indexes up.
pub unsafe fn winlink_shuffle_up(s: *mut session, mut wl: *mut winlink, before: bool) -> i32 {
    if wl.is_null() {
        return -1;
    }
    unsafe {
        let idx = if before { (*wl).idx } else { (*wl).idx + 1 };

        // Find the next free index.
        let mut last = idx;
        for i in idx..i32::MAX {
            last = i;
            if winlink_find_by_index(&raw mut (*s).windows, last).is_null() {
                break;
            }
        }
        if last == i32::MAX {
            return -1;
        }

        // Move everything from last - 1 to idx up a bit.
        while last > idx {
            wl = winlink_find_by_index(&raw mut (*s).windows, last - 1);
            (*(&raw mut (*s).windows)).remove(&(*wl).idx);
            (*wl).idx += 1;
            (*(&raw mut (*s).windows)).insert((*wl).idx, wl);
            last -= 1;
        }

        idx
    }
}

unsafe fn window_pane_input_callback(
    c: *mut client,
    _path: *mut u8,
    error: i32,
    closed: i32,
    buffer: *mut evbuffer,
    data: *mut c_void,
) {
    unsafe {
        let cdata: *mut window_pane_input_data = data as *mut window_pane_input_data;
        let buf: *mut c_uchar = EVBUFFER_DATA(buffer);
        let len: usize = EVBUFFER_LENGTH(buffer);

        let wp = window_pane_find_by_id((*cdata).wp);
        if !(*cdata).file.is_null() && (wp.is_null() || (*c).flags.intersects(client_flag::DEAD)) {
            if wp.is_null() {
                (*c).retval = 1;
                (*c).flags |= client_flag::EXIT;
            }
            file_cancel((*cdata).file);
        } else if (*cdata).file.is_null() || closed != 0 || error != 0 {
            cmdq_continue((*cdata).item);
            server_client_unref(c);
            free(cdata as _);
        } else {
            input_parse_buffer(wp, buf, len);
            evbuffer_drain(buffer, len);
        }
    }
}

pub unsafe fn window_pane_start_input(
    wp: *mut window_pane,
    item: *mut cmdq_item,
) -> Result<i32, String> {
    unsafe {
        let c: *mut client = cmdq_get_client(item);

        if !(*wp).flags.intersects(window_pane_flags::PANE_EMPTY) {
            return Err("pane is not empty".to_string());
        }
        if (*c)
            .flags
            .intersects(client_flag::DEAD | client_flag::EXITED)
        {
            return Ok(1);
        }
        if !client_get_session(c).is_null() {
            return Ok(1);
        }

        let cdata = Box::leak(Box::new(window_pane_input_data {
            item,
            wp: (*wp).id,
            file: null_mut(),
        })) as *mut window_pane_input_data;
        (*cdata).file = file_read(c, c!("-"), Some(window_pane_input_callback), cdata as _);
        (*c).references += 1;

        Ok(0)
    }
}

pub unsafe fn window_pane_get_new_data(
    wp: *mut window_pane,
    wpo: *mut window_pane_offset,
    size: *mut usize,
) -> *mut c_void {
    unsafe {
        let used = (*wpo).used - (*wp).base_offset;

        *size = EVBUFFER_LENGTH((*(*wp).event).input) - used;
        EVBUFFER_DATA((*(*wp).event).input).add(used) as _
    }
}

pub unsafe fn window_pane_update_used_data(
    wp: *mut window_pane,
    wpo: *mut window_pane_offset,
    mut size: usize,
) {
    unsafe {
        let used = (*wpo).used - (*wp).base_offset;

        if size > EVBUFFER_LENGTH((*(*wp).event).input) - used {
            size = EVBUFFER_LENGTH((*(*wp).event).input) - used;
        }
        (*wpo).used += size;
    }
}

pub unsafe fn window_set_fill_character(w: NonNull<window>) {
    let w = w.as_ptr();
    unsafe {
        free((*w).fill_character as _);
        (*w).fill_character = null_mut();

        let value = options_get_string_((*w).options, "fill-character");
        if *value != b'\0' && utf8_isvalid(value) {
            let ud = utf8_fromcstr(value);
            if !ud.is_null() && (*ud).width == 1 {
                (*w).fill_character = ud;
            }
        }
    }
}

pub unsafe fn window_pane_default_cursor(wp: *mut window_pane) {
    unsafe {
        let s = (*wp).screen;

        let c: i32 = options_get_number___::<i32>(&*(*wp).options, "cursor-colour");
        (*s).default_ccolour = c;

        let c: i32 = options_get_number___::<i32>(&*(*wp).options, "cursor-style");
        (*s).default_mode = mode_flag::empty();
        screen_set_cursor_style(
            c as u32,
            &raw mut (*s).default_cstyle,
            &raw mut (*s).default_mode,
        );
    }
}

pub unsafe fn window_pane_mode(wp: *mut window_pane) -> i32 {
    unsafe {
        if let Some(&wme) = (*wp).modes.first() {
            if (*wme).mode.addr() == (&raw const WINDOW_COPY_MODE).addr() {
                return WINDOW_PANE_COPY_MODE;
            }
            if (*wme).mode.addr() == (&raw const WINDOW_VIEW_MODE).addr() {
                return WINDOW_PANE_VIEW_MODE;
            }
        }
        WINDOW_PANE_NO_MODE
    }
}
