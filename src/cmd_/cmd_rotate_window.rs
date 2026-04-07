// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::window_::{window_pane_next_in_list, window_pane_prev_in_list};
use crate::*;

pub static CMD_ROTATE_WINDOW_ENTRY: cmd_entry = cmd_entry {
    name: "rotate-window",
    alias: Some("rotatew"),

    args: args_parse::new("Dt:UZ", 0, 0, None),
    usage: "[-DUZ] [-t target-window]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_WINDOW,
        cmd_find_flags::empty(),
    ),

    flags: cmd_flag::empty(),
    exec: cmd_rotate_window_exec,
    source: cmd_entry_flag::zeroed(),
};

unsafe fn cmd_rotate_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let current = cmdq_get_current(item);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        let w = winlink_window(wl);
        let mut wp: *mut window_pane;
        let lc: *mut layout_cell;
        let sx: u32;
        let sy: u32;
        let xoff: u32;
        let yoff: u32;

        window_push_zoom(w, false, args_has(args, 'Z'));

        if args_has(args, 'D') {
            wp = (*w).panes.last().copied().unwrap_or(null_mut());
            (*w).panes.retain(|&p| p != wp);
            (*w).panes.insert(0, wp);

            lc = (*wp).layout_cell;
            xoff = (*wp).xoff;
            yoff = (*wp).yoff;

            sx = (*wp).sx;
            sy = (*wp).sy;

            for &wp_ in (*w).panes.iter() {
                wp = wp_;
                let wp2 = window_pane_next_in_list(wp);
                if wp2.is_null() {
                    break;
                }
                (*wp).layout_cell = (*wp2).layout_cell;
                if !(*wp).layout_cell.is_null() {
                    (*(*wp).layout_cell).wp = pane_id_from_ptr(wp);
                }
                (*wp).xoff = (*wp2).xoff;
                (*wp).yoff = (*wp2).yoff;
                window_pane_resize(wp, (*wp2).sx, (*wp2).sy);
            }
            (*wp).layout_cell = lc;
            if !(*wp).layout_cell.is_null() {
                (*(*wp).layout_cell).wp = pane_id_from_ptr(wp);
            }
            (*wp).xoff = xoff;
            (*wp).yoff = yoff;
            window_pane_resize(wp, sx, sy);

            wp = window_pane_prev_in_list(window_active_pane(w));
            if wp.is_null() {
                wp = (*w).panes.last().copied().unwrap_or(null_mut());
            }
        } else {
            wp = (*w).panes.first().copied().unwrap_or(null_mut());
            (*w).panes.retain(|&p| p != wp);
            (*w).panes.push(wp);

            lc = (*wp).layout_cell;
            xoff = (*wp).xoff;
            yoff = (*wp).yoff;
            sx = (*wp).sx;
            sy = (*wp).sy;
            for &wp_ in (*w).panes.iter().rev() {
                wp = wp_;
                let wp2 = window_pane_prev_in_list(wp);
                if wp2.is_null() {
                    break;
                }
                (*wp).layout_cell = (*wp2).layout_cell;
                if !(*wp).layout_cell.is_null() {
                    (*(*wp).layout_cell).wp = pane_id_from_ptr(wp);
                }
                (*wp).xoff = (*wp2).xoff;
                (*wp).yoff = (*wp2).yoff;
                window_pane_resize(wp, (*wp2).sx, (*wp2).sy);
            }
            (*wp).layout_cell = lc;
            if !(*wp).layout_cell.is_null() {
                (*(*wp).layout_cell).wp = pane_id_from_ptr(wp);
            }
            (*wp).xoff = xoff;
            (*wp).yoff = yoff;
            window_pane_resize(wp, sx, sy);

            wp = window_pane_next_in_list(window_active_pane(w));
            if wp.is_null() {
                wp = (*w).panes.first().copied().unwrap_or(null_mut());
            }
        }

        window_set_active_pane(w, wp, 1);
        cmd_find_from_winlink_pane(current, wl, wp, cmd_find_flags::empty());
        window_pop_zoom(w);
        server_redraw_window(w);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
