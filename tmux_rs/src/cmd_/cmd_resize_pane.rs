use compat_rs::{queue::tailq_empty, strtonum};

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_resize_pane_entry: cmd_entry = cmd_entry {
    name: c"resize-pane".as_ptr(),
    alias: c"resizep".as_ptr(),

    args: args_parse::new(c"DLMRTt:Ux:y:Z", 0, 1, None),
    usage: c"[-DLMRTUZ] [-x width] [-y height] [-t target-pane] [adjustment]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_resize_pane_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_resize_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut event = cmdq_get_event(item);
        let mut wp = (*target).wp;
        let mut wl = (*target).wl;
        let mut w = (*wl).window;
        let mut c = cmdq_get_client(item);
        let mut s = (*target).s;
        let mut cause: *mut c_char = null_mut();
        let mut errstr: *const c_char = null();
        let mut adjust = 0u32;
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut status: i32 = 0;
        let mut gd = (*wp).base.grid;

        if (args_has_(args, 'T')) {
            if !tailq_empty(&raw mut (*wp).modes) {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            adjust = screen_size_y(&raw mut (*wp).base) - 1 - (*wp).base.cy;
            if (adjust > (*gd).hsize) {
                adjust = (*gd).hsize;
            }
            grid_remove_history(gd, adjust);
            (*wp).base.cy += adjust;
            (*wp).flags |= PANE_REDRAW;
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if (args_has_(args, 'M')) {
            if (*event).m.valid == 0 || cmd_mouse_window(&raw mut (*event).m, &raw mut s).is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if (c.is_null() || (*c).session != s) {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            (*c).tty.mouse_drag_update = Some(cmd_resize_pane_mouse_update);
            cmd_resize_pane_mouse_update(c, &raw mut (*event).m);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if (args_has_(args, 'Z')) {
            if ((*w).flags & WINDOW_ZOOMED != 0) {
                window_unzoom(w, 1);
            } else {
                window_zoom(wp);
            }
            server_redraw_window(w);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        server_unzoom_window(w);

        if (args_count(args) == 0) {
            adjust = 1;
        } else {
            adjust = strtonum(args_string(args, 0), 1, i32::MAX as i64, &raw mut errstr) as u32;
            if (!errstr.is_null()) {
                cmdq_error(item, c"adjustment %s".as_ptr(), errstr);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        if (args_has_(args, 'x')) {
            x = args_percentage(args, b'x', 0, i32::MAX as i64, (*w).sx as i64, &raw mut cause) as i32;
            if (!cause.is_null()) {
                cmdq_error(item, c"width %s".as_ptr(), cause);
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            layout_resize_pane_to(wp, layout_type::LAYOUT_LEFTRIGHT, x as u32);
        }
        if args_has_(args, 'y') {
            y = args_percentage(args, b'y', 0, i32::MAX as i64, (*w).sy as i64, &raw mut cause) as i32;
            if (!cause.is_null()) {
                cmdq_error(item, c"height %s".as_ptr(), cause);
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            status = options_get_number((*w).options, c"pane-border-status".as_ptr()) as i32;
            match status {
                PANE_STATUS_TOP => {
                    if (y != i32::MAX && (*wp).yoff == 1) {
                        y += 1;
                    }
                }
                PANE_STATUS_BOTTOM => {
                    if (y != i32::MAX && (*wp).yoff + (*wp).sy == (*w).sy - 1) {
                        y += 1;
                    }
                }
                _ => (),
            }
            layout_resize_pane_to(wp, layout_type::LAYOUT_TOPBOTTOM, y as u32);
        }

        if (args_has_(args, 'L')) {
            layout_resize_pane(wp, layout_type::LAYOUT_LEFTRIGHT, -(adjust as i32), 1);
        } else if (args_has_(args, 'R')) {
            layout_resize_pane(wp, layout_type::LAYOUT_LEFTRIGHT, adjust as i32, 1);
        } else if (args_has_(args, 'U')) {
            layout_resize_pane(wp, layout_type::LAYOUT_TOPBOTTOM, -(adjust as i32), 1);
        } else if (args_has_(args, 'D')) {
            layout_resize_pane(wp, layout_type::LAYOUT_TOPBOTTOM, adjust as i32, 1);
        }
        server_redraw_window((*wl).window);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_resize_pane_mouse_update(c: *mut client, m: *mut mouse_event) {
    unsafe {
        let mut wl: *mut winlink = null_mut();
        let mut w: *mut window = null_mut();
        let mut y: u32 = 0;
        let mut ly: u32 = 0;
        let mut x: u32 = 0;
        let mut lx: u32 = 0;
        const offsets: [[c_int; 2]; 5] = [[0, 0], [0, 1], [1, 0], [0, -1], [-1, 0]];
        let mut ncells: u32 = 0;
        let mut cells: [*mut layout_cell; offsets.len()] = zeroed();
        let mut resizes: u32 = 0;

        let wl = cmd_mouse_window(m, null_mut());
        if wl.is_null() {
            (*c).tty.mouse_drag_update = None;
            return;
        }
        w = (*wl).window;

        y = (*m).y + (*m).oy;
        x = (*m).x + (*m).ox;
        if ((*m).statusat == 0 && y >= (*m).statuslines) {
            y -= (*m).statuslines;
        } else if ((*m).statusat > 0 && y >= (*m).statusat as u32) {
            y = ((*m).statusat - 1) as u32;
        }
        ly = (*m).ly + (*m).oy;
        lx = (*m).lx + (*m).ox;
        if ((*m).statusat == 0 && ly >= (*m).statuslines) {
            ly -= (*m).statuslines;
        } else if ((*m).statusat > 0 && ly >= (*m).statusat as u32) {
            ly = ((*m).statusat - 1) as u32;
        }

        for i in 0..cells.len() {
            let mut lc =
                layout_search_by_border((*w).layout_root, lx + offsets[i][0] as u32, ly + offsets[i][1] as u32);
            if lc.is_null() {
                continue;
            }

            for j in 0..ncells {
                if cells[j as usize] == lc {
                    lc = null_mut();
                    break;
                }
            }
            if lc.is_null() {
                continue;
            }

            cells[ncells as usize] = lc;
            ncells += 1;
        }
        if (ncells == 0) {
            return;
        }

        for i in 0..ncells {
            let type_ = (*(*cells[i as usize]).parent).type_;
            if (y != ly && type_ == layout_type::LAYOUT_TOPBOTTOM) {
                layout_resize_layout(w, cells[i as usize], type_, y as i32 - ly as i32, 0);
                resizes += 1;
            } else if (x != lx && type_ == layout_type::LAYOUT_LEFTRIGHT) {
                layout_resize_layout(w, cells[i as usize], type_, x as i32 - lx as i32, 0);
                resizes += 1;
            }
        }
        if (resizes != 0) {
            server_redraw_window(w);
        }
    }
}
