#![allow(clippy::missing_safety_doc)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::{
    ops::ControlFlow,
    ptr::{NonNull, null_mut},
};
use std::{ffi::c_char, ptr::null};

use compat_rs::{
    queue::{tailq_foreach, tailq_insert_head, tailq_insert_tail, tailq_remove},
    strtonum,
    tree::{rb_find, rb_foreach, rb_insert, rb_min, rb_next, rb_prev, rb_remove},
};
use libc::{FIONREAD, TIOCSWINSZ, free, ioctl, winsize};
use libc::{gettimeofday, memset};
use log::{fatal, fatalx, log_debug};
use tmux_h::*;
use xmalloc::{xcalloc, xstrdup};

pub static mut windows: windows = unsafe { std::mem::zeroed() };

pub static mut all_window_panes: window_pane_tree = unsafe { std::mem::zeroed() };
static mut next_window_pane_id: u32 = 0;
static mut next_window_id: u32 = 0;
static mut next_active_point: u32 = 0;

#[expect(unused)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct window_pane_input_data {
    item: *mut cmdq_item,
    wp: u32,
    file: *mut client_file,
}

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
pub unsafe extern "C" fn winlink_find_by_window(
    wwl: *mut winlinks,
    w: *mut window,
) -> *mut winlink {
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

        let wl: NonNull<winlink> = xcalloc(1, size_of::<winlink>()).cast();
        (*wl.as_ptr()).idx = idx;
        rb_insert(wwl, wl.as_ptr());

        wl.as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn winlink_set_window(wl: *mut winlink, w: *mut window) {
    unsafe {
        if (*wl).window.is_null() {
            tailq_remove!(&raw mut (*(*wl).window).winlinks, wl, wentry);
            window_remove_ref((*wl).window, c"winlink_set_window".as_ptr());
        }
        tailq_insert_tail!(&raw mut (*w).winlinks, wl, wentry);
        (*wl).window = w;
        window_add_ref(w, c"winlink_set_window".as_ptr());
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn winlink_remove(wwl: *mut winlinks, wl: *mut winlink) {
    unsafe {
        let w = (*wl).window;

        if !w.is_null() {
            tailq_remove!(&raw mut (*w).winlinks, wl, wentry);
            window_remove_ref(w, c"winlink_remove".as_ptr());
        }

        rb_remove(wwl, wl);
        free(wl as _);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn winlink_next(wl: *mut winlink) -> *mut winlink {
    unsafe { rb_next(wl) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn winlink_previous(wl: *mut winlink) -> *mut winlink {
    unsafe { rb_prev(wl) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn winlink_next_by_number(
    mut wl: *mut winlink,
    s: *mut session,
    n: i32,
) -> *mut winlink {
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
unsafe extern "C" fn winlink_previous_by_number(
    mut wl: *mut winlink,
    s: *mut session,
    n: i32,
) -> *mut winlink {
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
unsafe extern "C" fn winlink_stack_push(stack: *mut winlink_stack, wl: *mut winlink) {
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
unsafe extern "C" fn winlink_stack_remove(stack: *mut winlink_stack, wl: *mut winlink) {
    unsafe {
        if !wl.is_null() && (*wl).flags & WINLINK_VISITED != 0 {
            tailq_remove!(stack, wl, sentry);
            (*wl).flags &= !WINLINK_VISITED;
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_find_by_id_str(s: *const c_char) -> *mut window {
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
unsafe extern "C" fn window_find_by_id(id: u32) -> *mut window {
    unsafe {
        let mut w: window = std::mem::zeroed();

        w.id = id;
        rb_find(&raw mut windows, &raw mut w)
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_update_activity(w: *mut window) {
    unsafe {
        gettimeofday(&raw mut (*w).activity_time, null_mut());
        todo!()
        // alerts_queue(w, WINDOW_ACTIVITY);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_create(
    sx: u32,
    sy: u32,
    mut xpixel: u32,
    mut ypixel: u32,
) -> *mut window {
    if xpixel == 0 {
        xpixel = DEFAULT_XPIXEL as u32;
    }
    if ypixel == 0 {
        ypixel = DEFAULT_YPIXEL as u32;
    }

    let w: &mut window = unsafe { xcalloc(1, size_of::<window>()).cast().as_mut() };
    w.name = unsafe { xstrdup(c"".as_ptr()) }.as_ptr();
    w.flags = 0;

    // tailq_init
    // tailq_init
    w.active = null_mut();

    w.lastlayout = -1;
    w.layout_root = null_mut();

    w.sx = sx;
    w.sy = sy;
    w.manual_sx = sx;
    w.manual_sy = sy;
    w.xpixel = xpixel;
    w.ypixel = ypixel;

    w.options = options_create(global_w_options);

    w.references = 0;
    // tailq_init

    w.id = next_window_id;
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

        if event_initialized(&raw mut (*w).name_event) {
            event_del(&raw mut (*w).name_event);
        }

        if event_initialized(&raw mut (*w).alerts_timer) {
            event_del(&raw mut (*w).alerts_timer);
        }
        if event_initialized(&raw mut (*w).offset_timer) {
            event_del(&raw mut (*w).offset_timer);
        }

        options_free((*w).options);
        free((*w).fill_character as _);

        free((*w).name as _);
        free(w as _);
    }
}

pub unsafe extern "C" fn window_pane_destroy_ready(wp: *mut window_pane) -> i32 {
    let mut n = 0;

    if (*wp).pipe_fd != -1 {
        if evbuffer_length((*(*wp).pipe_event).output) != 0 {
            return 0;
        }
        if ioctl((*wp).fd, FIONREAD, &raw mut n) != -1 && n > 0 {
            return 0;
        }
    }

    if !(*wp).flags & PANE_EXITED {
        return 0;
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe fn window_add_ref(w: *mut window, from: *const c_char) {
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
pub unsafe fn window_remove_ref(w: *mut window, from: *const c_char) {
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
pub unsafe fn window_set_name(w: *mut window, new_name: *mut c_char) {
    unsafe {
        free((*w).name as _);
        utf8_stravis(
            &raw mut (*w).name,
            new_name,
            VIS_OCTAL | VIS_CSTYLE | VIS_TAB | VIS_NL,
        );
        notify_window(c"window-renamed".as_ptr(), w);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn window_resize(w: *mut window, sx: u32, sy: u32, mut xpixel: i32, mut ypixel: i32) {
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

fn window_pane_send_resize(wp: *mut window_pane, sx: u32, sy: u32) {
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
unsafe extern "C" fn window_has_pane(w: *mut window, wp: *mut window_pane) -> i32 {
    unsafe {
        if tailq_foreach(&raw mut (*w).panes, |wp1| {
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
unsafe extern "C" fn window_update_focus(w: *mut window) {
    if !w.is_null() {
        log_debug(
            c"%s: @%u".as_ptr(),
            c"window_update_focus".as_ptr(),
            (*w).id,
        );
        window_pane_update_focus((*w).active);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn window_pane_update_focus(wp: *mut window_pane) {
    unsafe {
        let mut focused = false;

        if !wp.is_null() && ((!(*wp).flags) & PANE_EXITED) != 0 {
            if wp != (*(*wp).window).active {
                focused = false
            } else {
                // TODO import clients from server.c
                tailq_foreach(&raw mut clients, |c| {
                    if !(*c).session.is_null()
                        && (*c).session.attached != 0
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
                    bufferevent_write((*wp).event, c"\x1b[O".as_ptr(), 3);
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
                    bufferevent_write((*wp).event, c"\x1b[I".as_ptr(), 3);
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

TODO: continue from here translating from window.c.bak
