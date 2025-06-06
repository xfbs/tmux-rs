use crate::*;

use crate::compat::queue::{
    tailq_first, tailq_foreach, tailq_foreach_reverse, tailq_insert_head, tailq_insert_tail,
    tailq_last, tailq_next, tailq_prev, tailq_remove,
};

#[unsafe(no_mangle)]
static mut cmd_rotate_window_entry: cmd_entry = cmd_entry {
    name: c"rotate-window".as_ptr(),
    alias: c"rotatew".as_ptr(),

    args: args_parse::new(c"Dt:UZ", 0, 0, None),
    usage: c"[-DUZ] [-t target-window]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_rotate_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_rotate_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut current = cmdq_get_current(item);
        let mut target = cmdq_get_target(item);
        let mut wl = (*target).wl;
        let mut w = (*wl).window;
        let mut wp: *mut window_pane;
        let mut wp2: *mut window_pane;
        let mut lc: *mut layout_cell;
        let mut sx: u32;
        let mut sy: u32;
        let mut xoff: u32;
        let mut yoff: u32;

        window_push_zoom(w, 0, args_has(args, b'Z'));

        if args_has_(args, 'D') {
            wp = tailq_last(&raw mut (*w).panes);
            tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
            tailq_insert_head!(&raw mut (*w).panes, wp, entry);

            lc = (*wp).layout_cell;
            xoff = (*wp).xoff;
            yoff = (*wp).yoff;

            sx = (*wp).sx;
            sy = (*wp).sy;

            for wp_ in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                wp = wp_;
                let wp2 = tailq_next::<_, _, discr_entry>(wp);
                if wp2.is_null() {
                    break;
                }
                (*wp).layout_cell = (*wp2).layout_cell;
                if !(*wp).layout_cell.is_null() {
                    (*(*wp).layout_cell).wp = wp;
                }
                (*wp).xoff = (*wp2).xoff;
                (*wp).yoff = (*wp2).yoff;
                window_pane_resize(wp, (*wp2).sx, (*wp2).sy);
            }
            (*wp).layout_cell = lc;
            if !(*wp).layout_cell.is_null() {
                (*(*wp).layout_cell).wp = wp;
            }
            (*wp).xoff = xoff;
            (*wp).yoff = yoff;
            window_pane_resize(wp, sx, sy);

            wp = tailq_prev::<_, _, discr_entry>((*w).active);
            if wp.is_null() {
                wp = tailq_last(&raw mut (*w).panes);
            }
        } else {
            wp = tailq_first(&raw mut (*w).panes);
            tailq_remove::<_, discr_entry>(&raw mut (*w).panes, wp);
            tailq_insert_tail::<_, discr_entry>(&raw mut (*w).panes, wp);

            lc = (*wp).layout_cell;
            xoff = (*wp).xoff;
            yoff = (*wp).yoff;
            sx = (*wp).sx;
            sy = (*wp).sy;
            for wp_ in
                tailq_foreach_reverse::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr)
            {
                wp = wp_;
                let wp2 = tailq_prev::<_, _, discr_entry>(wp);
                if wp2.is_null() {
                    break;
                }
                (*wp).layout_cell = (*wp2).layout_cell;
                if !(*wp).layout_cell.is_null() {
                    (*(*wp).layout_cell).wp = wp;
                }
                (*wp).xoff = (*wp2).xoff;
                (*wp).yoff = (*wp2).yoff;
                window_pane_resize(wp, (*wp2).sx, (*wp2).sy);
            }
            (*wp).layout_cell = lc;
            if !(*wp).layout_cell.is_null() {
                (*(*wp).layout_cell).wp = wp;
            }
            (*wp).xoff = xoff;
            (*wp).yoff = yoff;
            window_pane_resize(wp, sx, sy);

            wp = tailq_next::<_, _, discr_entry>((*w).active);
            if wp.is_null() {
                wp = tailq_first(&raw mut (*w).panes);
            }
        }

        window_set_active_pane(w, wp, 1);
        cmd_find_from_winlink_pane(current, wl, wp, 0);
        window_pop_zoom(w);
        server_redraw_window(w);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
