use crate::*;

use compat_rs::strtonum;

#[unsafe(no_mangle)]
static mut cmd_resize_window_entry: cmd_entry = cmd_entry {
    name: c"resize-window".as_ptr(),
    alias: c"resizew".as_ptr(),

    args: args_parse::new(c"aADLRt:Ux:y:", 0, 1, None),
    usage: c"[-aADLRU] [-x width] [-y height] [-t target-window] [adjustment]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_resize_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_resize_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut wl = (*target).wl;
        let mut w = (*wl).window;
        let mut s = (*target).s;
        let mut errstr: *const c_char = null();
        let mut cause = null_mut();
        let mut adjust: u32 = 0;
        let mut xpixel = 0u32;
        let mut ypixel = 0u32;

        if (args_count(args) == 0) {
            adjust = 1;
        } else {
            adjust = strtonum(args_string(args, 0), 1, i32::MAX as i64, &raw mut errstr) as u32;
            if !errstr.is_null() {
                cmdq_error(item, c"adjustment %s".as_ptr(), errstr);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        let mut sx = (*w).sx;
        let mut sy = (*w).sy;

        if (args_has(args, b'x') != 0) {
            sx = args_strtonum(args, b'x', WINDOW_MINIMUM as _, WINDOW_MAXIMUM as _, &raw mut cause) as u32;
            if !cause.is_null() {
                cmdq_error(item, c"width %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }
        if (args_has(args, b'y') != 0) {
            sy = args_strtonum(args, b'y', WINDOW_MINIMUM as _, WINDOW_MAXIMUM as _, &raw mut cause) as u32;
            if !cause.is_null() {
                cmdq_error(item, c"height %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        if (args_has(args, b'L') != 0) {
            if (sx >= adjust) {
                sx -= adjust;
            }
        } else if (args_has(args, b'R') != 0) {
            sx += adjust;
        } else if (args_has(args, b'U') != 0) {
            if (sy >= adjust) {
                sy -= adjust;
            }
        } else if (args_has(args, b'D') != 0) {
            sy += adjust;
        }

        if (args_has(args, b'A') != 0) {
            default_window_size(
                null_mut(),
                s,
                w,
                &raw mut sx,
                &raw mut sy,
                &raw mut xpixel,
                &raw mut ypixel,
                WINDOW_SIZE_LARGEST,
            );
        } else if (args_has(args, b'a') != 0) {
            default_window_size(
                null_mut(),
                s,
                w,
                &raw mut sx,
                &raw mut sy,
                &raw mut xpixel,
                &raw mut ypixel,
                WINDOW_SIZE_SMALLEST,
            );
        }

        options_set_number((*w).options, c"window-size".as_ptr(), WINDOW_SIZE_MANUAL as i64);
        (*w).manual_sx = sx;
        (*w).manual_sy = sy;
        recalculate_size(w, 1);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
