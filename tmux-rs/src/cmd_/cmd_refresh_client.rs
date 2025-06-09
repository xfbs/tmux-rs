use crate::*;

use libc::{sscanf, strchr, strcmp};

use crate::compat::strtonum;

#[unsafe(no_mangle)]
static mut cmd_refresh_client_entry: cmd_entry = cmd_entry {
    name: c"refresh-client".as_ptr(),
    alias: c"refresh".as_ptr(),

    args: args_parse::new(c"A:B:cC:Df:r:F:l::LRSt:U", 0, 1, None),
    usage: c"[-cDlLRSU] [-A pane:state] [-B name:what:format] [-C XxY] [-f flags] [-r pane:report] [-t target-client] [adjustment]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: Some(cmd_refresh_client_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_refresh_client_update_subscription(
    tc: *mut client,
    value: *const c_char,
) {
    unsafe {
        let mut split = null_mut::<c_char>();
        let mut subid = -1;
        let mut copy = null_mut();
        'out: {
            let mut name = xstrdup(value).as_ptr();
            copy = name;
            split = strchr(copy, ':' as i32);
            if split.is_null() {
                control_remove_sub(tc, copy);
                break 'out;
            }
            *split = b'\0' as _;
            split = split.add(1);

            let what = split;
            split = strchr(what, ':' as i32);
            if split.is_null() {
                break 'out;
            }
            *split = b'\0' as c_char;
            split = split.add(1);

            let subtype = if (strcmp(what, c"%*".as_ptr()) == 0) {
                control_sub_type::CONTROL_SUB_ALL_PANES
            } else if (sscanf(what, c"%%%d".as_ptr(), &subid) == 1 && subid >= 0) {
                control_sub_type::CONTROL_SUB_PANE
            } else if strcmp(what, c"@*".as_ptr()) == 0 {
                control_sub_type::CONTROL_SUB_ALL_WINDOWS
            } else if sscanf(what, c"@%d".as_ptr(), &subid) == 1 && subid >= 0 {
                control_sub_type::CONTROL_SUB_WINDOW
            } else {
                control_sub_type::CONTROL_SUB_SESSION
            };
            control_add_sub(tc, name, subtype, subid, split);
        }

        // out:
        free_(copy);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_refresh_client_control_client_size(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    let __func__ = "cmd_refresh_client_control_client_size";
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut tc = cmdq_get_target_client(item);
        let mut size = args_get(args, b'C');
        let mut w: u32 = 0;
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        // u_int w, x, y;
        // struct client_window *cw;

        if (sscanf(
            size,
            c"@%u:%ux%u".as_ptr(),
            &raw mut w,
            &raw mut x,
            &raw mut y,
        ) == 3)
        {
            if (x < WINDOW_MINIMUM
                || x > WINDOW_MAXIMUM
                || y < WINDOW_MINIMUM
                || y > WINDOW_MAXIMUM)
            {
                cmdq_error(item, c"size too small or too big".as_ptr());
                return cmd_retval::CMD_RETURN_ERROR;
            }
            log_debug!(
                "{}: client {} window @{}: size {}x{}",
                __func__,
                _s((*tc).name),
                w,
                x,
                y
            );
            let cw = server_client_add_client_window(tc, w).as_ptr();
            (*cw).sx = x;
            (*cw).sy = y;
            (*tc).flags |= client_flag::WINDOWSIZECHANGED;
            recalculate_sizes_now(1);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        if (sscanf(size, c"@%u:".as_ptr(), &w) == 1) {
            let cw = server_client_get_client_window(tc, w);
            if (!cw.is_null()) {
                log_debug!(
                    "{}: client {} window @{}: no size",
                    __func__,
                    _s((*tc).name),
                    w
                );
                (*cw).sx = 0;
                (*cw).sy = 0;
                recalculate_sizes_now(1);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if (sscanf(size, c"%u,%u".as_ptr(), &x, &y) != 2
            && sscanf(size, c"%ux%u".as_ptr(), &x, &y) != 2)
        {
            cmdq_error(item, c"bad size argument".as_ptr());
            return cmd_retval::CMD_RETURN_ERROR;
        }
        if (x < WINDOW_MINIMUM || x > WINDOW_MAXIMUM || y < WINDOW_MINIMUM || y > WINDOW_MAXIMUM) {
            cmdq_error(item, c"size too small or too big".as_ptr());
            return cmd_retval::CMD_RETURN_ERROR;
        }
        tty_set_size(&raw mut (*tc).tty, x, y, 0, 0);
        (*tc).flags |= client_flag::SIZECHANGED;
        recalculate_sizes_now(1);
    }
    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_refresh_client_update_offset(tc: *mut client, value: *const c_char) {
    unsafe {
        let mut pane: u32 = 0;

        if *value != b'%' as c_char {
            return;
        }
        let mut copy = xstrdup(value).as_ptr();
        'out: {
            let mut split = strchr(copy, ':' as i32);
            if split.is_null() {
                break 'out;
            }
            *split = b'\0' as c_char;
            split = split.add(1);

            if sscanf(copy, c"%%%u".as_ptr(), &raw mut pane) != 1 {
                break 'out;
            }
            let wp = window_pane_find_by_id(pane);
            if wp.is_null() {
                break 'out;
            }

            if (strcmp(split, c"on".as_ptr()) == 0) {
                control_set_pane_on(tc, wp);
            } else if (strcmp(split, c"off".as_ptr()) == 0) {
                control_set_pane_off(tc, wp);
            } else if (strcmp(split, c"continue".as_ptr()) == 0) {
                control_continue_pane(tc, wp);
            } else if strcmp(split, c"pause".as_ptr()) == 0 {
                control_pause_pane(tc, wp);
            }
        }

        // out:
        free_(copy);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_refresh_client_clipboard(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut tc = cmdq_get_target_client(item);
        let mut fs: cmd_find_state = zeroed();
        // const char *p;
        // u_int i;
        // struct cmd_find_state fs;

        let p = args_get_(args, 'l');
        if (p.is_null()) {
            if (*tc).flags.intersects(client_flag::CLIPBOARDBUFFER) {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            (*tc).flags |= client_flag::CLIPBOARDBUFFER;
        } else {
            if cmd_find_target(&raw mut fs, item, p, cmd_find_type::CMD_FIND_PANE, 0) != 0 {
                return cmd_retval::CMD_RETURN_ERROR;
            }
            let mut i = 0;
            for j in 0..(*tc).clipboard_npanes {
                i = j;
                if *(*tc).clipboard_panes.add(i as usize) == (*fs.wp).id {
                    break;
                }
            }
            if i != (*tc).clipboard_npanes {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            (*tc).clipboard_panes =
                xreallocarray_((*tc).clipboard_panes, (*tc).clipboard_npanes as usize + 1).as_ptr();
            *(*tc).clipboard_panes.add((*tc).clipboard_npanes as usize) = (*fs.wp).id;
            (*tc).clipboard_npanes += 1;
        }
        tty_clipboard_query(&raw mut (*tc).tty);
    }
    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_refresh_report(tty: *mut tty, value: *const c_char) {
    unsafe {
        let mut pane: u32 = 0;
        let mut size: usize = 0;

        if *value != b'%' as _ {
            return;
        }
        let mut copy = xstrdup(value).as_ptr();
        'out: {
            let mut split = strchr(copy, ':' as i32);
            if split.is_null() {
                break 'out;
            }
            *split = b'\0' as _;
            split = split.add(1);

            if sscanf(copy, c"%%%u".as_ptr(), &pane) != 1 {
                break 'out;
            }
            let wp = window_pane_find_by_id(pane);
            if wp.is_null() {
                break 'out;
            }

            tty_keys_colours(
                tty,
                split,
                strlen(split),
                &raw mut size,
                &raw mut (*wp).control_fg,
                &raw mut (*wp).control_bg,
            );
        }
        // out:
        free_(copy);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_refresh_client_exec(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut tc = cmdq_get_target_client(item);
        let mut tty = &raw mut (*tc).tty;
        let mut errstr: *const c_char = null();
        let mut adjust: u32 = 0;

        'not_control_client: {
            if (args_has_(args, 'c')
                || args_has_(args, 'L')
                || args_has_(args, 'R')
                || args_has_(args, 'U')
                || args_has_(args, 'D'))
            {
                if (args_count(args) == 0) {
                    adjust = 1;
                } else {
                    adjust =
                        strtonum(args_string(args, 0), 1, i32::MAX as i64, &raw mut errstr) as u32;
                    if (!errstr.is_null()) {
                        cmdq_error(item, c"adjustment %s".as_ptr(), errstr);
                        return cmd_retval::CMD_RETURN_ERROR;
                    }
                }

                if (args_has_(args, 'c')) {
                    (*tc).pan_window = null_mut();
                } else {
                    let w = (*(*(*tc).session).curw).window;
                    if ((*tc).pan_window != w.cast()) {
                        (*tc).pan_window = w.cast();
                        (*tc).pan_ox = (*tty).oox;
                        (*tc).pan_oy = (*tty).ooy;
                    }
                    if (args_has_(args, 'L')) {
                        if ((*tc).pan_ox > adjust) {
                            (*tc).pan_ox -= adjust;
                        } else {
                            (*tc).pan_ox = 0;
                        }
                    } else if (args_has_(args, 'R')) {
                        (*tc).pan_ox += adjust;
                        if (*tc).pan_ox > (*w).sx - (*tty).osx {
                            (*tc).pan_ox = (*w).sx - (*tty).osx;
                        }
                    } else if (args_has_(args, 'U')) {
                        if ((*tc).pan_oy > adjust) {
                            (*tc).pan_oy -= adjust;
                        } else {
                            (*tc).pan_oy = 0;
                        }
                    } else if (args_has_(args, 'D')) {
                        (*tc).pan_oy += adjust;
                        if (*tc).pan_oy > (*w).sy - (*tty).osy {
                            (*tc).pan_oy = (*w).sy - (*tty).osy;
                        }
                    }
                }
                tty_update_client_offset(tc);
                server_redraw_client(tc);
                return cmd_retval::CMD_RETURN_NORMAL;
            }

            if args_has_(args, 'l') {
                return cmd_refresh_client_clipboard(self_, item);
            }

            if args_has_(args, 'F') {
                server_client_set_flags(tc, args_get(args, b'F'));
            } /* -F is an alias for -f */
            if args_has_(args, 'f') {
                server_client_set_flags(tc, args_get(args, b'f'));
            }
            if args_has_(args, 'r') {
                cmd_refresh_report(tty, args_get(args, b'r'));
            }

            if (args_has_(args, 'A')) {
                if !(*tc).flags.intersects(client_flag::CONTROL) {
                    break 'not_control_client;
                }
                let mut av = args_first_value(args, b'A');
                while (!av.is_null()) {
                    cmd_refresh_client_update_offset(tc, (*av).union_.string);
                    av = args_next_value(av);
                }
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if (args_has_(args, 'B')) {
                if !(*tc).flags.intersects(client_flag::CONTROL) {
                    break 'not_control_client;
                }
                let mut av = args_first_value(args, b'B');
                while (!av.is_null()) {
                    cmd_refresh_client_update_subscription(tc, (*av).union_.string);
                    av = args_next_value(av);
                }
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if (args_has_(args, 'C')) {
                if !(*tc).flags.intersects(client_flag::CONTROL) {
                    break 'not_control_client;
                }
                return cmd_refresh_client_control_client_size(self_, item);
            }

            if (args_has_(args, 'S')) {
                (*tc).flags |= client_flag::STATUSFORCE;
                server_status_client(tc);
            } else {
                (*tc).flags |= client_flag::STATUSFORCE;
                server_redraw_client(tc);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        // not_control_client:
        cmdq_error(item, c"not a control client".as_ptr());
        cmd_retval::CMD_RETURN_ERROR
    }
}
