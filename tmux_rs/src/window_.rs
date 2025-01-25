use super::*;

use compat_rs::{
    HOST_NAME_MAX,
    queue::{
        tailq_empty, tailq_first, tailq_foreach, tailq_foreach_safe, tailq_init, tailq_insert_after,
        tailq_insert_before, tailq_insert_head, tailq_insert_tail, tailq_last, tailq_next, tailq_prev, tailq_remove,
    },
    strtonum,
    tree::{rb_find, rb_foreach, rb_insert, rb_min, rb_next, rb_prev, rb_remove},
    vis::{VIS_CSTYLE, VIS_NL, VIS_OCTAL, VIS_TAB},
};
use libc::{
    FIONREAD, FNM_CASEFOLD, TIOCSWINSZ, close, fnmatch, free, gethostname, gettimeofday, ioctl, isspace, memset,
    regcomp, regex_t, regexec, regfree, strcasecmp, strlen, winsize,
};
use libevent_sys::{
    EV_READ, EV_WRITE, bufferevent, bufferevent_disable, bufferevent_enable, bufferevent_free, bufferevent_new,
    bufferevent_write, evbuffer, evbuffer_drain, event_del, event_initialized,
};

#[cfg(feature = "utempter")]
use crate::utempter::utempter_remove_record;

#[unsafe(no_mangle)]
pub static mut windows: windows = unsafe { std::mem::zeroed() };

#[unsafe(no_mangle)]
pub static mut all_window_panes: window_pane_tree = unsafe { std::mem::zeroed() };
static mut next_window_pane_id: u32 = 0;
static mut next_window_id: u32 = 0;
static mut next_active_point: u32 = 0;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_input_data {
    item: *mut cmdq_item,
    wp: u32,
    file: *mut client_file,
}

// RB_GENERATE(windows, window, entry, window_cmp);
// RB_GENERATE(winlinks, winlink, entry, winlink_cmp);
// RB_GENERATE(window_pane_tree, window_pane, tree_entry, window_pane_cmp);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_cmp(w1: *const window, w2: *const window) -> i32 {
    unsafe { (*w1).id.wrapping_sub((*w2).id) as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_cmp(wl1: *const winlink, wl2: *const winlink) -> i32 {
    unsafe { (*wl1).idx.wrapping_sub((*wl2).idx) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_cmp(wp1: *const window_pane, wp2: *const window_pane) -> i32 {
    unsafe { (*wp1).id.wrapping_sub((*wp2).id) as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_find_by_window(wwl: *mut winlinks, w: *mut window) -> *mut winlink {
    unsafe {
        rb_foreach(wwl, |wl| {
            if (*wl).window == w {
                return ControlFlow::Break(wl);
            }

            ControlFlow::Continue(())
        })
        .unwrap_or(null_mut())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_find_by_index(wwl: *mut winlinks, idx: i32) -> *mut winlink {
    unsafe {
        if idx < 0 {
            fatalx(c"bad index".as_ptr());
        }

        let mut wl: winlink = std::mem::zeroed();
        wl.idx = idx;

        rb_find(wwl, &raw mut wl)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_find_by_window_id(wwl: *mut winlinks, id: u32) -> *mut winlink {
    unsafe {
        rb_foreach(wwl, |wl| {
            if (*(*wl).window).id == id {
                return ControlFlow::Break(wl);
            }
            ControlFlow::Continue(())
        })
        .unwrap_or(null_mut())
    }
}

unsafe extern "C" fn winlink_next_index(wwl: *mut winlinks, idx: i32) -> i32 {
    let mut i = idx;

    loop {
        if unsafe { winlink_find_by_index(wwl, i).is_null() } {
            return i;
        }

        if i == i32::MAX {
            i = 0
        } else {
            i += 1;
        }

        if i == idx {
            break;
        }
    }

    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_count(wwl: *mut winlinks) -> u32 {
    let mut n = 0;
    unsafe {
        rb_foreach(wwl, |_wl| {
            n += 1;
            ControlFlow::<(), ()>::Continue(())
        });
    }
    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_add(wwl: *mut winlinks, mut idx: i32) -> *mut winlink {
    unsafe {
        if idx < 0 {
            idx = winlink_next_index(wwl, -idx - 1);
            if idx == -1 {
                return null_mut();
            }
        } else if !winlink_find_by_index(wwl, idx).is_null() {
            return null_mut();
        }

        let wl: *mut winlink = xcalloc(1, size_of::<winlink>()).cast().as_ptr();
        (*wl).idx = idx;
        rb_insert(wwl, wl);

        wl
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_set_window(wl: *mut winlink, w: *mut window) {
    unsafe {
        if !(*wl).window.is_null() {
            tailq_remove::<_, wentry>(&raw mut (*(*wl).window).winlinks, wl);
            window_remove_ref((*wl).window, c"winlink_set_window".as_ptr());
        }
        tailq_insert_tail::<_, wentry>(&raw mut (*w).winlinks, wl);
        (*wl).window = w;
        window_add_ref(w, c"winlink_set_window".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_remove(wwl: *mut winlinks, wl: *mut winlink) {
    unsafe {
        let w = (*wl).window;

        if !w.is_null() {
            tailq_remove::<_, wentry>(&raw mut (*w).winlinks, wl);
            window_remove_ref(w, c"winlink_remove".as_ptr());
        }

        rb_remove(wwl, wl);
        free(wl as _);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_next(wl: *mut winlink) -> *mut winlink {
    unsafe { rb_next(wl) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_previous(wl: *mut winlink) -> *mut winlink {
    unsafe { rb_prev(wl) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_next_by_number(mut wl: *mut winlink, s: *mut session, n: i32) -> *mut winlink {
    unsafe {
        for _ in 0..n {
            wl = rb_next(wl);
            if wl.is_null() {
                wl = rb_min(&raw mut (*s).windows);
            }
        }
    }

    wl
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_previous_by_number(mut wl: *mut winlink, s: *mut session, n: i32) -> *mut winlink {
    unsafe {
        for _ in 0..n {
            wl = rb_prev(wl);
            if wl.is_null() {
                wl = rb_min(&raw mut (*s).windows);
            }
        }
    }

    wl
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_stack_push(stack: *mut winlink_stack, wl: *mut winlink) {
    if wl.is_null() {
        return;
    }

    unsafe {
        winlink_stack_remove(stack, wl);
        tailq_insert_head!(stack, wl, sentry);
        (*wl).flags |= WINLINK_VISITED;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_stack_remove(stack: *mut winlink_stack, wl: *mut winlink) {
    unsafe {
        if !wl.is_null() && (*wl).flags & WINLINK_VISITED != 0 {
            tailq_remove::<_, sentry>(stack, wl);
            (*wl).flags &= !WINLINK_VISITED;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_find_by_id_str(s: *const c_char) -> *mut window {
    unsafe {
        let mut errstr: *const c_char = null_mut();

        if *s != b'@' as i8 {
            return null_mut();
        }

        let id = strtonum(s.wrapping_add(1), 0, u32::MAX as i64, &raw mut errstr) as u32;
        if !errstr.is_null() {
            return null_mut();
        }

        window_find_by_id(id)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_find_by_id(id: u32) -> *mut window {
    unsafe {
        let mut w: window = std::mem::zeroed();

        w.id = id;
        rb_find(&raw mut windows, &raw mut w)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_update_activity(w: *mut window) {
    unsafe {
        gettimeofday(&raw mut (*w).activity_time, null_mut());
        alerts_queue(w, WINDOW_ACTIVITY);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_create(sx: u32, sy: u32, mut xpixel: u32, mut ypixel: u32) -> *mut window {
    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL as u32;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL as u32;
    }
    unsafe {
        let w: *mut window = xcalloc(1, size_of::<window>()).cast().as_ptr();
        (*w).name = xstrdup(c"".as_ptr()).as_ptr();
        (*w).flags = 0;

        // tailq_init
        // tailq_init
        (*w).active = null_mut();

        (*w).lastlayout = -1;
        (*w).layout_root = null_mut();

        (*w).sx = sx;
        (*w).sy = sy;
        (*w).manual_sx = sx;
        (*w).manual_sy = sy;
        (*w).xpixel = xpixel;
        (*w).ypixel = ypixel;

        (*w).options = options_create(global_w_options);

        (*w).references = 0;
        tailq_init(&raw mut (*w).winlinks);

        (*w).id = next_window_id;
        next_window_id += 1;
        rb_insert(&raw mut windows, w);

        window_set_fill_character(w);
        window_update_activity(w);

        log_debug(
            c"%s: @%u create %ux%u (%ux%u)".as_ptr(),
            c"window_create".as_ptr(),
            (*w).id,
            sx,
            sy,
            (*w).xpixel,
            (*w).ypixel,
        );
        w as _
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_destroy(w: *mut window) {
    unsafe {
        log_debug(
            c"window @%u destroyed (%d references)".as_ptr(),
            (*w).id,
            (*w).references,
        );

        window_unzoom(w, 0);
        rb_remove(&raw mut windows, w);

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

        free((*w).name as _);
        free(w as _);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_destroy_ready(wp: *mut window_pane) -> i32 {
    let mut n = 0;
    unsafe {
        if (*wp).pipe_fd != -1 {
            if EVBUFFER_LENGTH((*(*wp).pipe_event).output) != 0 {
                return 0;
            }
            if ioctl((*wp).fd, FIONREAD, &raw mut n) != -1 && n > 0 {
                return 0;
            }
        }

        if !(*wp).flags & PANE_EXITED != 0 {
            return 0;
        }
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_add_ref(w: *mut window, from: *const c_char) {
    unsafe {
        (*w).references += 1;
        log_debug(
            c"%s: @%u %s, now %d".as_ptr(),
            c"window_add_ref".as_ptr(),
            (*w).id,
            from,
            (*w).references,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_remove_ref(w: *mut window, from: *const c_char) {
    unsafe {
        (*w).references -= 1;
        log_debug(
            c"%s: @%u %s, now %d".as_ptr(),
            c"window_remove_ref".as_ptr(),
            (*w).id,
            from,
            (*w).references,
        );

        if (*w).references == 0 {
            window_destroy(w);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_set_name(w: *mut window, new_name: *mut c_char) {
    unsafe {
        free((*w).name as _);
        utf8_stravis(&raw mut (*w).name, new_name, VIS_OCTAL | VIS_CSTYLE | VIS_TAB | VIS_NL);
        notify_window(c"window-renamed".as_ptr(), w);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_resize(w: *mut window, sx: u32, sy: u32, mut xpixel: i32, mut ypixel: i32) {
    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL;
    }

    unsafe {
        log_debug(
            c"%s: @%u resize %ux%u (%ux%u)".as_ptr(),
            c"window_resize".as_ptr(),
            (*w).id,
            sx,
            sy,
            if xpixel == -1 { (*w).xpixel } else { xpixel as u32 },
            if ypixel == -1 { (*w).ypixel } else { ypixel as u32 },
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_send_resize(wp: *mut window_pane, sx: u32, sy: u32) {
    unsafe {
        let w = (*wp).window;
        let mut ws: winsize = core::mem::zeroed();

        if (*wp).fd == -1 {
            return;
        }

        log_debug(
            c"%s: %%%u resize to %u,%u".as_ptr(),
            c"window_pane_send_resize".as_ptr(),
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
            fatal(c"ioctl failed".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_has_pane(w: *mut window, wp: *mut window_pane) -> i32 {
    unsafe {
        if tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |wp1| {
            if wp1 == wp {
                return ControlFlow::Break(());
            }
            ControlFlow::Continue(())
        })
        .is_break()
        {
            return 1;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_update_focus(w: *mut window) {
    unsafe {
        if !w.is_null() {
            log_debug(c"%s: @%u".as_ptr(), c"window_update_focus".as_ptr(), (*w).id);
            window_pane_update_focus((*w).active);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_update_focus(wp: *mut window_pane) {
    unsafe {
        let mut focused = false;

        if !wp.is_null() && ((!(*wp).flags) & PANE_EXITED) != 0 {
            if wp != (*(*wp).window).active {
                focused = false
            } else {
                tailq_foreach(&raw mut clients, |c| {
                    if !(*c).session.is_null()
                        && (*(*c).session).attached != 0
                        && (*c).flags & CLIENT_FOCUSED != 0
                        && (*(*(*c).session).curw).window == (*wp).window
                    {
                        focused = true;
                        return ControlFlow::Break(());
                    }
                    ControlFlow::Continue(())
                });
            }
            if !focused && (*wp).flags & PANE_FOCUSED != 0 {
                log_debug(
                    c"%s: %%%u focus out".as_ptr(),
                    c"window_pane_update_focus".as_ptr(),
                    (*wp).id,
                );
                if (*wp).base.mode & MODE_FOCUSON != 0 {
                    bufferevent_write((*wp).event, c"\x1b[O".as_ptr() as _, 3);
                }
                notify_pane(c"pane-focus-out".as_ptr(), wp);
                (*wp).flags &= !PANE_FOCUSED;
            } else if focused && (!(*wp).flags & PANE_FOCUSED) != 0 {
                log_debug(
                    c"%s: %%%u focus in".as_ptr(),
                    c"window_pane_update_focus".as_ptr(),
                    (*wp).id,
                );
                if (*wp).base.mode & MODE_FOCUSON != 0 {
                    bufferevent_write((*wp).event, c"\x1b[I".as_ptr() as _, 3);
                }
                notify_pane(c"pane-focus-in".as_ptr(), wp);
                (*wp).flags |= PANE_FOCUSED;
            } else {
                log_debug(
                    c"%s: %%%u focus unchanged".as_ptr(),
                    c"window_pane_update_focus".as_ptr(),
                    (*wp).id,
                );
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_set_active_pane(w: *mut window, wp: *mut window_pane, notify: i32) -> i32 {
    let lastwp: *mut window_pane;
    unsafe {
        log_debug(c"%s: pane %%%u".as_ptr(), c"window_set_active_pane".as_ptr(), (*wp).id);

        if wp == (*w).active {
            return 0;
        }
        lastwp = (*w).active;

        window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        window_pane_stack_push(&raw mut (*w).last_panes, lastwp);

        (*w).active = wp;
        (*(*w).active).active_point = next_active_point;
        next_active_point += 1;
        (*(*w).active).flags |= PANE_CHANGED;

        if options_get_number(global_options, c"focus-events".as_ptr()) != 0 {
            window_pane_update_focus(lastwp);
            window_pane_update_focus((*w).active);
        }

        tty_update_window_offset(w);

        if notify != 0 {
            notify_window(c"window-pane-changed".as_ptr(), w);
        }
    }
    1
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_get_palette(wp: *mut window_pane, c: i32) -> i32 {
    if wp.is_null() {
        -1
    } else {
        unsafe { colour_palette_get(&raw mut (*wp).palette, c) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_redraw_active_switch(w: *mut window, mut wp: *mut window_pane) {
    unsafe {
        if wp == (*w).active {
            return;
        }

        loop {
            /*
             * If the active and inactive styles or palettes are different,
             * need to redraw the panes.
             */
            let gc1 = &raw mut (*wp).cached_gc;
            let gc2 = &raw mut (*wp).cached_active_gc;
            if grid_cells_look_equal(gc1, gc2) == 0 {
                (*wp).flags |= PANE_REDRAW;
            } else {
                let mut c1 = window_pane_get_palette(wp, (*gc1).fg);
                let mut c2 = window_pane_get_palette(wp, (*gc2).fg);
                if c1 != c2 {
                    (*wp).flags |= PANE_REDRAW;
                } else {
                    c1 = window_pane_get_palette(wp, (*gc1).bg);
                    c2 = window_pane_get_palette(wp, (*gc2).bg);
                    if c1 != c2 {
                        (*wp).flags |= PANE_REDRAW;
                    }
                }
            }
            if wp == (*w).active {
                break;
            }
            wp = (*w).active;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_get_active_at(w: *mut window, x: u32, y: u32) -> *mut window_pane {
    unsafe {
        if let ControlFlow::Break(value) = tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |wp| {
            if window_pane_visible(wp) != 0 {
                return ControlFlow::Continue(());
            }
            if x < (*wp).xoff || x > (*wp).xoff + (*wp).sx {
                return ControlFlow::Continue(());
            }
            if y < (*wp).yoff || y > (*wp).yoff + (*wp).sy {
                return ControlFlow::Continue(());
            }
            ControlFlow::Break(wp)
        }) {
            return value;
        }

        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_find_string(w: *mut window, s: *const c_char) -> *mut window_pane {
    unsafe {
        let mut top: u32 = 0;
        let mut bottom: u32 = (*w).sy - 1;

        let mut x = (*w).sx / 2;
        let mut y = (*w).sy / 2;

        let status = options_get_number((*w).options, c"pane-border-status".as_ptr());
        if status == PANE_STATUS_TOP as _ {
            top += 1;
        } else if status == PANE_STATUS_BOTTOM as _ {
            bottom -= 1;
        }

        if strcasecmp(s, c"top".as_ptr()) == 0 {
            y = top;
        } else if strcasecmp(s, c"bottom".as_ptr()) == 0 {
            y = bottom;
        } else if strcasecmp(s, c"left".as_ptr()) == 0 {
            x = 0;
        } else if strcasecmp(s, c"right".as_ptr()) == 0 {
            x = (*w).sx - 1;
        } else if strcasecmp(s, c"top-left".as_ptr()) == 0 {
            x = 0;
            y = top;
        } else if strcasecmp(s, c"top-right".as_ptr()) == 0 {
            x = (*w).sx - 1;
            y = top;
        } else if strcasecmp(s, c"bottom-left".as_ptr()) == 0 {
            x = 0;
            y = bottom;
        } else if strcasecmp(s, c"bottom-right".as_ptr()) == 0 {
            x = (*w).sx - 1;
            y = bottom;
        } else {
            return null_mut();
        }

        window_get_active_at(w, x, y)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_zoom(wp: *mut window_pane) -> i32 {
    unsafe {
        let w = (*wp).window;

        if (*w).flags & WINDOW_ZOOMED != 0 {
            return -1;
        }

        if window_count_panes(w) == 1 {
            return -1;
        }

        if (*w).active != wp {
            window_set_active_pane(w, wp, 1);
        }

        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |wp1| {
            (*wp1).saved_layout_cell = (*wp1).layout_cell;
            (*wp1).layout_cell = null_mut();
            ControlFlow::<(), ()>::Continue(())
        });

        (*w).saved_layout_root = (*w).layout_root;
        layout_init(w, wp);
        (*w).flags |= WINDOW_ZOOMED;
        notify_window(c"window-layout-changed".as_ptr(), w);

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_unzoom(w: *mut window, notify: i32) -> i32 {
    unsafe {
        if (*w).flags & WINDOW_ZOOMED == 0 {
            return -1;
        }

        (*w).flags &= !WINDOW_ZOOMED;
        layout_free(w);
        (*w).layout_root = (*w).saved_layout_root;
        (*w).saved_layout_root = null_mut();

        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |wp| {
            (*wp).layout_cell = (*wp).saved_layout_cell;
            (*wp).saved_layout_cell = null_mut();
            ControlFlow::<(), ()>::Continue(())
        });
        layout_fix_panes(w, null_mut());

        if notify != 0 {
            notify_window(c"window-layout-changed".as_ptr(), w);
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_push_zoom(w: *mut window, always: i32, flag: i32) -> i32 {
    unsafe {
        log_debug(
            c"%s: @%u %d".as_ptr(),
            c"window_push_zoom".as_ptr(),
            (*w).id,
            (flag != 0 && (*w).flags & WINDOW_ZOOMED != 0) as i32,
        );
        if flag != 0 && (always != 0 || (*w).flags & WINDOW_ZOOMED != 0) {
            (*w).flags |= WINDOW_WASZOOMED;
        } else {
            (*w).flags &= !WINDOW_WASZOOMED;
        }

        if window_unzoom(w, 1) == 0 { 1 } else { 0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pop_zoom(w: *mut window) -> i32 {
    unsafe {
        log_debug(
            c"%s: @%u %d".as_ptr(),
            c"window_pop_zoom".as_ptr(),
            (*w).id,
            !!((*w).flags & WINDOW_WASZOOMED),
        );
        if (*w).flags & WINDOW_WASZOOMED != 0 {
            return if window_zoom((*w).active) == 0 { 1 } else { 0 };
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_add_pane(
    w: *mut window,
    mut other: *mut window_pane,
    hlimit: u32,
    flags: i32,
) -> *mut window_pane {
    let func = c"window_add_pane".as_ptr();
    unsafe {
        if other.is_null() {
            other = (*w).active;
        }

        let wp = window_pane_create(w, (*w).sx, (*w).sy, hlimit);
        if tailq_empty(&raw mut (*w).panes) {
            log_debug(c"%s: @%u at start".as_ptr(), func, (*w).id);
            tailq_insert_head!(&raw mut (*w).panes, wp, entry);
        } else if flags & SPAWN_BEFORE != 0 {
            log_debug(c"%s: @%u before %%%u".as_ptr(), func, (*w).id, (*wp).id);
            if flags & SPAWN_FULLSIZE != 0 {
                tailq_insert_head!(&raw mut (*w).panes, wp, entry);
            } else {
                tailq_insert_before!(other, wp, entry);
            }
        } else {
            log_debug(c"%s: @%u after %%%u".as_ptr(), func, (*w).id, (*wp).id);
            if flags & SPAWN_FULLSIZE != 0 {
                tailq_insert_tail::<_, entry>(&raw mut (*w).panes, wp);
            } else {
                tailq_insert_after!(&raw mut (*w).panes, other, wp, entry);
            }
        }

        wp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_lost_pane(w: *mut window, wp: *mut window_pane) {
    unsafe {
        log_debug(
            c"%s: @%u pane %%%u".as_ptr(),
            c"window_lost_pane".as_ptr(),
            (*w).id,
            (*wp).id,
        );

        if wp == marked_pane.wp {
            server_clear_marked();
        }

        window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        if wp == (*w).active {
            (*w).active = tailq_first(&raw mut (*w).last_panes);
            if (*w).active.is_null() {
                (*w).active = tailq_prev!(wp, window_panes, entry);
                if (*w).active.is_null() {
                    (*w).active = tailq_next::<_, _, entry>(wp);
                }
            }
            if !(*w).active.is_null() {
                window_pane_stack_remove(&raw mut (*w).last_panes, (*w).active);
                (*(*w).active).flags |= PANE_CHANGED;
                notify_window(c"window-pane-changed".as_ptr(), w);
                window_update_focus(w);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_remove_pane(w: *mut window, wp: *mut window_pane) {
    unsafe {
        window_lost_pane(w, wp);

        tailq_remove::<_, entry>(&raw mut (*w).panes, wp);
        window_pane_destroy(wp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_at_index(w: *mut window, idx: u32) -> *mut window_pane {
    unsafe {
        let mut n: u32 = options_get_number((*w).options, c"pane-base-index".as_ptr()) as _;

        match tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |wp| {
            if n == idx {
                return ControlFlow::Break(wp);
            }
            n += 1;
            ControlFlow::Continue(())
        }) {
            ControlFlow::Break(wp) => wp,
            ControlFlow::Continue(()) => null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_next_by_number(
    w: *mut window,
    mut wp: *mut window_pane,
    n: u32,
) -> *mut window_pane {
    unsafe {
        for _ in 0..n {
            wp = tailq_next::<_, _, entry>(wp);
            if wp.is_null() {
                wp = tailq_first(&raw mut (*w).panes);
            }
        }
    }

    wp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_previous_by_number(
    w: *mut window,
    mut wp: *mut window_pane,
    n: u32,
) -> *mut window_pane {
    unsafe {
        for _ in 0..n {
            wp = tailq_prev!(wp, window_panes, entry);
            if wp.is_null() {
                wp = tailq_last!(&raw mut (*w).panes, window_panes);
            }
        }
    }

    wp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_index(wp: *mut window_pane, i: *mut u32) -> i32 {
    unsafe {
        let w = (*wp).window;

        *i = options_get_number((*w).options, c"pane-base-index".as_ptr()) as _;
        match tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |wq| {
            if wp == wq {
                return ControlFlow::Break(0);
            }
            (*i) += 1;
            ControlFlow::Continue(())
        }) {
            ControlFlow::Break(value) => value,
            ControlFlow::Continue(()) => -1,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_count_panes(w: *mut window) -> u32 {
    let mut n = 0;

    unsafe {
        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |_wp| {
            n += 1;
            ControlFlow::Continue::<(), ()>(())
        });
    }

    n
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_destroy_panes(w: *mut window) {
    let mut wp: *mut window_pane;
    unsafe {
        while !tailq_empty(&raw mut (*w).last_panes) {
            wp = tailq_first(&raw mut (*w).last_panes);
            window_pane_stack_remove(&raw mut (*w).last_panes, wp);
        }

        while !tailq_empty(&raw mut (*w).panes) {
            wp = tailq_first(&raw mut (*w).panes);
            tailq_remove::<_, entry>(&raw mut (*w).panes, wp);
            window_pane_destroy(wp);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_printable_flags(wl: *mut winlink, escape: i32) -> *const c_char {
    static mut flags: [c_char; 32] = [0; 32];

    unsafe {
        let s = (*wl).session;

        let mut pos = 0;
        if (*wl).flags & WINLINK_ACTIVITY != 0 {
            flags[pos] = b'#' as c_char;
            pos += 1;
            if escape != 0 {
                flags[pos] = b'#' as c_char;
                pos += 1;
            }
        }
        if (*wl).flags & WINLINK_BELL != 0 {
            flags[pos] = b'!' as c_char;
            pos += 1;
        }
        if (*wl).flags & WINLINK_SILENCE != 0 {
            flags[pos] = b'~' as c_char;
            pos += 1;
        }
        if wl == (*s).curw {
            flags[pos] = b'*' as c_char;
            pos += 1;
        }
        if wl == tailq_first(&raw mut (*s).lastw) {
            flags[pos] = b'-' as c_char;
            pos += 1;
        }
        if server_check_marked() != 0 && wl == marked_pane.wl {
            flags[pos] = b'M' as c_char;
            pos += 1;
        }
        if (*(*wl).window).flags & WINDOW_ZOOMED != 0 {
            flags[pos] = b'Z' as c_char;
            pos += 1;
        }
        flags[pos] = b'\0' as c_char;
        &raw mut flags as *mut i8
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_find_by_id_str(s: *mut c_char) -> *mut window_pane {
    let mut errstr: *const c_char = null_mut();
    unsafe {
        if *s != b'%' as c_char {
            return null_mut();
        }

        let id = strtonum(s.add(1), 0, u32::MAX as i64, &raw mut errstr) as u32;
        if !errstr.is_null() {
            null_mut()
        } else {
            window_pane_find_by_id(id)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_find_by_id(id: u32) -> *mut window_pane {
    unsafe {
        let mut wp: window_pane = zeroed();
        wp.id = id;
        rb_find(&raw mut all_window_panes, &raw mut wp)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_create(w: *mut window, sx: u32, sy: u32, hlimit: u32) -> *mut window_pane {
    unsafe {
        let mut host: [c_char; HOST_NAME_MAX + 1] = zeroed();
        let wp: *mut window_pane = xcalloc(1, size_of::<window_pane>()).cast().as_ptr();
        (*wp).window = w;
        (*wp).options = options_create((*w).options);
        (*wp).flags = PANE_STYLECHANGED;

        (*wp).id = next_window_pane_id;
        next_window_pane_id += 1;
        // TOOD, maybe the invariant is broken when creating
        // the ref and then passing a *mut away
        rb_insert(&raw mut all_window_panes, wp);

        (*wp).fd = -1;

        tailq_init(&raw mut (*wp).modes);

        tailq_init(&raw mut (*wp).resize_queue);

        (*wp).sx = sx;
        (*wp).sy = sy;

        (*wp).pipe_fd = -1;

        (*wp).control_bg = -1;
        (*wp).control_fg = -1;

        colour_palette_init(&raw mut (*wp).palette);
        colour_palette_from_option(&raw mut (*wp).palette, (*wp).options);

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

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_destroy(wp: *mut window_pane) {
    unsafe {
        window_pane_reset_mode_all(wp);
        free((*wp).searchstr as _);

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
        tailq_foreach_safe(&raw mut (*wp).resize_queue, |r| {
            tailq_remove::<_, ()>(&raw mut (*wp).resize_queue, r);
            free(r as _);
            ControlFlow::Continue::<(), ()>(())
        });

        rb_remove(&raw mut all_window_panes, wp);

        options_free((*wp).options);
        free((*wp).cwd as _);
        free((*wp).shell as _);
        cmd_free_argv((*wp).argc, (*wp).argv);
        colour_palette_free(&raw mut (*wp).palette);
        free(wp as _);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_read_callback(_bufev: *mut bufferevent, data: *mut c_void) {
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

        log_debug(c"%%%u has %zu bytes".as_ptr(), (*wp).id, size);
        tailq_foreach(&raw mut clients, |c| {
            if !(*c).session.is_null() && (*c).flags & CLIENT_CONTROL != 0 {
                control_write_output(c, wp);
            }
            ControlFlow::Continue::<(), ()>(())
        });
        input_parse_pane(wp);
        bufferevent_disable((*wp).event, EV_READ as i16);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_error_callback(_bufev: *mut bufferevent, _what: c_short, data: *mut c_void) {
    let wp: *mut window_pane = data as _;
    unsafe {
        log_debug(c"%%%u error".as_ptr(), (*wp).id);
        (*wp).flags |= PANE_EXITED;

        if window_pane_destroy_ready(wp) != 0 {
            server_destroy_pane(wp, 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_set_event(wp: *mut window_pane) {
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
            fatalx(c"out of memory".as_ptr());
        }
        (*wp).ictx = input_init(wp, (*wp).event, &raw mut (*wp).palette);

        bufferevent_enable((*wp).event, EV_READ as i16 | EV_WRITE as i16);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_resize(wp: *mut window_pane, sx: u32, sy: u32) {
    unsafe {
        if sx == (*wp).sx && sy == (*wp).sy {
            return;
        }

        let r: *mut window_pane_resize = xmalloc(size_of::<window_pane_resize>()).cast().as_ptr();
        (*r).sx = sx;
        (*r).sy = sy;
        (*r).osx = (*wp).sx;
        (*r).osy = (*wp).sy;
        tailq_insert_tail(&raw mut (*wp).resize_queue, r);

        (*wp).sx = sx;
        (*wp).sy = sy;

        log_debug(
            c"%s: %%%u resize %ux%u".as_ptr(),
            c"window_pane_resize".as_ptr(),
            (*wp).id,
            sx,
            sy,
        );
        screen_resize(&raw mut (*wp).base, sx, sy, (*wp).base.saved_grid.is_null() as i32);

        let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
        if !wme.is_null() {
            if let Some(resize) = (*(*wme).mode).resize {
                resize(wme, sx, sy);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_set_mode(
    wp: *mut window_pane,
    swp: *mut window_pane,
    mode: *mut window_mode,
    fs: *mut cmd_find_state,
    args: *mut args,
) -> i32 {
    unsafe {
        if !tailq_empty(&raw mut (*wp).modes) && (*tailq_first(&raw mut (*wp).modes)).mode == mode {
            return 1;
        }

        let control_flow = tailq_foreach(&raw mut (*wp).modes, |wme| {
            if (*wme).mode == mode {
                return ControlFlow::Break(wme);
            }
            ControlFlow::Continue(())
        });

        let wme: *mut window_mode_entry;
        if let ControlFlow::Break(break_value) = control_flow {
            wme = break_value;

            tailq_remove::<_, ()>(&raw mut (*wp).modes, wme);
            tailq_insert_head!(&raw mut (*wp).modes, wme, entry);
        } else {
            wme = xcalloc(1, size_of::<window_mode_entry>()).cast().as_ptr();
            (*wme).wp = wp;
            (*wme).swp = swp;
            (*wme).mode = mode;
            (*wme).prefix = 1;
            tailq_insert_head!(&raw mut (*wp).modes, wme, entry);
            (*wme).screen = (*(*wme).mode).init.unwrap()(wme, fs, args);
        }

        (*wp).screen = (*wme).screen;
        (*wp).flags |= PANE_REDRAW | PANE_CHANGED;

        server_redraw_window_borders((*wp).window);
        server_status_window((*wp).window);
        notify_pane(c"pane-mode-changed".as_ptr(), wp);

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_reset_mode(wp: *mut window_pane) {
    unsafe {
        if tailq_empty(&raw mut (*wp).modes) {
            return;
        }

        let wme = tailq_first(&raw mut (*wp).modes);
        tailq_remove::<_, ()>(&raw mut (*wp).modes, wme);
        (*(*wme).mode).free.unwrap()(wme);
        free(wme as _);

        let next = tailq_first(&raw mut (*wp).modes);
        let func = c"window_pane_reset_mode".as_ptr();
        if next.is_null() {
            (*wp).flags &= !PANE_UNSEENCHANGES;
            log_debug(c"%s: no next mode".as_ptr(), func);
            (*wp).screen = &raw mut (*wp).base;
        } else {
            log_debug(c"%s: next mode is %s".as_ptr(), func, (*(*next).mode).name);
            (*wp).screen = (*next).screen;
            if let Some(resize) = (*(*next).mode).resize {
                resize(next, (*wp).sx, (*wp).sy);
            }
        }
        (*wp).flags |= PANE_REDRAW | PANE_CHANGED;

        server_redraw_window_borders((*wp).window);
        server_status_window((*wp).window);
        notify_pane(c"pane-mode-changed".as_ptr(), wp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_reset_mode_all(wp: *mut window_pane) {
    unsafe {
        while !tailq_empty(&raw mut (*wp).modes) {
            window_pane_reset_mode(wp);
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_copy_key(wp: *mut window_pane, key: key_code) {
    unsafe {
        tailq_foreach::<_, _, _, entry>(&raw mut (*(*wp).window).panes, |loop_| {
            if loop_ != wp
                && tailq_empty(&raw mut (*loop_).modes)
                && (*loop_).fd != -1
                && (!(*loop_).flags & PANE_INPUTOFF) != 0
                && window_pane_visible(loop_) != 0
                && options_get_number((*loop_).options, c"synchronize-panes".as_ptr()) != 0
            {
                input_key_pane(loop_, key, null_mut());
            }
            ControlFlow::Continue::<(), ()>(())
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_key(
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
        let wme = tailq_first(&raw mut (*wp).modes);
        if !wme.is_null() {
            if (*(*wme).mode).key.is_some() && !c.is_null() {
                key &= !KEYC_MASK_FLAGS;
                (*(*wme).mode).key.unwrap()(wme, c, s, wl, key, m);
            }
            return 0;
        }

        if (*wp).fd == -1 || (*wp).flags & PANE_INPUTOFF != 0 {
            return 0;
        }

        if input_key_pane(wp, key, m) != 0 {
            return -1;
        }

        if KEYC_IS_MOUSE(key) {
            return 0;
        }
        if options_get_number((*wp).options, c"synchronize-panes".as_ptr()) != 0 {
            window_pane_copy_key(wp, key);
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_visible(wp: *mut window_pane) -> i32 {
    unsafe {
        if !(*(*wp).window).flags & WINDOW_ZOOMED != 0 {
            return 1;
        }
        if wp == (*(*wp).window).active { 1 } else { 0 }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_exited(wp: *mut window_pane) -> i32 {
    unsafe {
        if (*wp).fd == -1 || (*wp).flags & PANE_EXITED != 0 {
            1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_search(wp: *mut window_pane, term: *const c_char, regex: i32, ignore: i32) -> u32 {
    unsafe {
        let s: *mut screen = &raw mut (*wp).base;
        let mut r: regex_t = zeroed();
        let mut new: *mut c_char = null_mut();
        let mut flags = 0;

        if regex == 0 {
            if ignore != 0 {
                flags |= FNM_CASEFOLD;
            }
            xasprintf(&raw mut new, c"*%s*".as_ptr(), term);
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

            log_debug(c"%s: %s".as_ptr(), c"window_pane_search".as_ptr(), line);
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

/* Get MRU pane from a list. */
#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_choose_best(list: *mut *mut window_pane, size: u32) -> *mut window_pane {
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

/*
 * Find the pane directly above another. We build a list of those adjacent to
 * top edge and then choose the best.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_find_up(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        if wp.is_null() {
            return null_mut();
        }
        let w = (*wp).window;
        let status = options_get_number((*w).options, c"pane-border-status".as_ptr());

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).yoff;
        if status == PANE_STATUS_TOP as _ {
            if edge == 1 {
                edge = (*w).sy + 1;
            }
        } else if status == PANE_STATUS_BOTTOM as _ {
            if edge == 0 {
                edge = (*w).sy;
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if edge == 0 {
                edge = (*w).sy + 1;
            }
        }

        let left = (*wp).xoff;
        let right = (*wp).xoff + (*wp).sx;

        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |next| {
            if next == wp {
                return ControlFlow::Continue::<(), ()>(());
            }
            if (*next).yoff + (*next).sy + 1 != edge {
                return ControlFlow::Continue::<(), ()>(());
            }
            let end = (*next).xoff + (*next).sx - 1;

            let mut found = 0;
            #[allow(clippy::if_same_then_else)]
            if (*next).xoff < left && end > right {
                found = 1;
            } else if (*next).xoff >= left && (*next).xoff <= right {
                found = 1;
            } else if end >= left && end <= right {
                found = 1;
            }
            if found == 0 {
                return ControlFlow::Continue(());
            }
            list = xreallocarray(list as _, size + 1, size_of::<*mut window_pane>())
                .cast()
                .as_ptr();
            *list.add(size) = next;
            size += 1;
            ControlFlow::Continue::<(), ()>(())
        });

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/* Find the pane directly below another. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_find_down(wp: *mut window_pane) -> *mut window_pane {
    unsafe {
        if wp.is_null() {
            return null_mut();
        }
        let w = (*wp).window;
        let status = options_get_number((*w).options, c"pane-border-status".as_ptr());

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).yoff + (*wp).sy + 1;
        if status == PANE_STATUS_TOP as _ {
            if edge >= (*w).sy {
                edge = 1;
            }
        } else if status == PANE_STATUS_BOTTOM as _ {
            if edge >= (*w).sy - 1 {
                edge = 0;
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if edge >= (*w).sy {
                edge = 0;
            }
        }

        let left = (*wp).xoff;
        let right = (*wp).xoff + (*wp).sx;

        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |next| {
            if next == wp {
                return ControlFlow::Continue::<(), ()>(());
            }
            if (*next).yoff != edge {
                return ControlFlow::Continue::<(), ()>(());
            }
            let end = (*next).xoff + (*next).sx - 1;

            let mut found = 0;
            #[allow(clippy::if_same_then_else)]
            if (*next).xoff < left && end > right {
                found = 1;
            } else if (*next).xoff >= left && (*next).xoff <= right {
                found = 1;
            } else if end >= left && end <= right {
                found = 1;
            }
            if found == 0 {
                return ControlFlow::Continue::<(), ()>(());
            }
            list = xreallocarray(list as _, size + 1, size_of::<*mut window_pane>())
                .cast()
                .as_ptr();
            *list.add(size) = next;
            size += 1;

            ControlFlow::Continue::<(), ()>(())
        });

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/* Find the pane directly to the left of another. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_find_left(wp: *mut window_pane) -> *mut window_pane {
    if wp.is_null() {
        return null_mut();
    }
    unsafe {
        let w = (*wp).window;

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).xoff;
        if edge == 0 {
            edge = (*w).sx + 1;
        }

        let top = (*wp).yoff;
        let bottom = (*wp).yoff + (*wp).sy;

        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |next| {
            if next == wp {
                return ControlFlow::Continue::<(), ()>(());
            }
            if (*next).xoff + (*next).sx + 1 != edge {
                return ControlFlow::Continue::<(), ()>(());
            }
            let end = (*next).yoff + (*next).sy - 1;

            let mut found = false;
            #[allow(clippy::if_same_then_else)]
            if (*next).yoff < top && end > bottom {
                found = true;
            } else if (*next).yoff >= top && (*next).yoff <= bottom {
                found = true;
            } else if end >= top && end <= bottom {
                found = true;
            }
            if !found {
                return ControlFlow::Continue::<(), ()>(());
            }
            list = xreallocarray(list as _, size + 1, size_of::<*mut window_pane>())
                .cast()
                .as_ptr();
            *list.add(size) = next;
            size += 1;
            ControlFlow::Continue::<(), ()>(())
        });

        let best = window_pane_choose_best(list, size as u32);
        free(list as _);
        best
    }
}

/* Find the pane directly to the right of another. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_find_right(wp: *mut window_pane) -> *mut window_pane {
    if wp.is_null() {
        return null_mut();
    }
    unsafe {
        let w = (*wp).window;

        let mut list: *mut *mut window_pane = null_mut();
        let mut size = 0;

        let mut edge = (*wp).xoff + (*wp).sx + 1;
        if edge >= (*w).sx {
            edge = 0;
        }

        let top = (*wp).yoff;
        let bottom = (*wp).yoff + (*wp).sy;

        tailq_foreach::<_, _, _, entry>(&raw mut (*w).panes, |next| {
            if next == wp {
                return ControlFlow::Continue::<(), ()>(());
            }
            if (*next).xoff != edge {
                return ControlFlow::Continue::<(), ()>(());
            }
            let end = (*next).yoff + (*next).sy - 1;

            let mut found = false;
            #[allow(clippy::if_same_then_else)]
            if (*next).yoff < top && end > bottom {
                found = true;
            } else if (*next).yoff >= top && (*next).yoff <= bottom {
                found = true;
            } else if end >= top && end <= bottom {
                found = true;
            }
            if !found {
                return ControlFlow::Continue::<(), ()>(());
            }
            list = xreallocarray(list as _, size + 1, size_of::<*mut window_pane>())
                .cast()
                .as_ptr();
            *list.add(size) = next;
            size += 1;
            ControlFlow::Continue::<(), ()>(())
        });

        let best = window_pane_choose_best(list, size as _);
        free(list as _);
        best
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_stack_push(stack: *mut window_panes, wp: *mut window_pane) {
    unsafe {
        if !wp.is_null() {
            window_pane_stack_remove(stack, wp);
            tailq_insert_head!(stack, wp, sentry);
            (*wp).flags |= PANE_VISITED;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_stack_remove(stack: *mut window_panes, wp: *mut window_pane) {
    unsafe {
        if !wp.is_null() && (*wp).flags & PANE_VISITED != 0 {
            tailq_remove::<_, crate::sentry>(stack, wp);
            (*wp).flags &= !PANE_VISITED;
        }
    }
}

/* Clear alert flags for a winlink */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_clear_flags(wl: *mut winlink) {
    unsafe {
        (*(*wl).window).flags &= !WINDOW_ALERTFLAGS;
        tailq_foreach::<_, _, _, crate::wentry>(&raw mut (*(*wl).window).winlinks, |loop_| {
            if ((*loop_).flags & WINLINK_ALERTFLAGS) != 0 {
                (*loop_).flags &= !WINLINK_ALERTFLAGS;
                server_status_session((*loop_).session);
            }
            ControlFlow::Continue::<(), ()>(())
        });
    }
}

/* Shuffle window indexes up. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_shuffle_up(s: *mut session, mut wl: *mut winlink, before: i32) -> i32 {
    if wl.is_null() {
        return -1;
    }
    unsafe {
        let idx = if before != 0 { (*wl).idx } else { (*wl).idx + 1 };

        /* Find the next free index. */
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

        /* Move everything from last - 1 to idx up a bit. */
        while last > idx {
            wl = winlink_find_by_index(&raw mut (*s).windows, last - 1);
            rb_remove(&raw mut (*s).windows, wl);
            (*wl).idx += 1;
            rb_insert(&raw mut (*s).windows, wl);
            last -= 1;
        }

        idx
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_input_callback(
    c: *mut client,
    _path: *mut c_char,
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
        if !(*cdata).file.is_null() && (wp.is_null() || (*c).flags & CLIENT_DEAD != 0) {
            if wp.is_null() {
                (*c).retval = 1;
                (*c).flags |= CLIENT_EXIT;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_start_input(
    wp: *mut window_pane,
    item: *mut cmdq_item,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        let c: *mut client = cmdq_get_client(item);

        if !(*wp).flags & PANE_EMPTY != 0 {
            *cause = xstrdup(c"pane is not empty".as_ptr()).cast().as_ptr();
            return -1;
        }
        if ((*c).flags & (CLIENT_DEAD | CLIENT_EXITED)) != 0 {
            return 1;
        }
        if !(*c).session.is_null() {
            return 1;
        }

        let cdata: *mut window_pane_input_data = xmalloc(size_of::<window_pane_input_data>()).cast().as_ptr();
        (*cdata).item = item;
        (*cdata).wp = (*wp).id;
        (*cdata).file = file_read(c, c"-".as_ptr(), Some(window_pane_input_callback), cdata as _);
        (*c).references += 1;

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_get_new_data(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_update_used_data(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_set_fill_character(w: *mut window) {
    //const char		*value;
    //struct utf8_data	*ud;
    unsafe {
        free((*w).fill_character as _);
        (*w).fill_character = null_mut();

        let value = options_get_string((*w).options, c"fill-character".as_ptr());
        if *value != b'\0' as _ && utf8_isvalid(value) != 0 {
            let ud = utf8_fromcstr(value);
            if !ud.is_null() && (*ud).width == 1 {
                (*w).fill_character = ud;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_default_cursor(wp: *mut window_pane) {
    unsafe {
        let s = (*wp).screen;

        let c: i32 = options_get_number((*wp).options, c"cursor-colour".as_ptr()) as i32;
        (*s).default_ccolour = c;

        let c: i32 = options_get_number((*wp).options, c"cursor-style".as_ptr()) as i32;
        (*s).default_mode = 0;
        screen_set_cursor_style(c as u32, &raw mut (*s).default_cstyle, &raw mut (*s).default_mode);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn window_pane_mode(wp: *mut window_pane) -> i32 {
    unsafe {
        if !tailq_first(&raw mut (*wp).modes).is_null() {
            if (*tailq_first(&raw mut (*wp).modes)).mode == &raw mut window_copy_mode {
                return WINDOW_PANE_COPY_MODE;
            }
            if (*tailq_first(&raw mut (*wp).modes)).mode == &raw mut window_view_mode {
                return WINDOW_PANE_VIEW_MODE;
            }
        }
        WINDOW_PANE_NO_MODE
    }
}
