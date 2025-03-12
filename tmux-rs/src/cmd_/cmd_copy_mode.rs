use super::*;

#[unsafe(no_mangle)]
static mut cmd_copy_mode_entry: cmd_entry = cmd_entry {
    name: c"copy-mode".as_ptr(),
    alias: null_mut(),

    args: args_parse {
        template: c"deHMs:t:uq".as_ptr(),
        lower: 0,
        upper: 0,
        cb: None,
    },
    usage: c"[-deHMuq] [-s src-pane] [-t target-pane]".as_ptr(),

    source: cmd_entry_flag {
        flag: b's' as c_char,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },
    target: cmd_entry_flag {
        flag: b't' as c_char,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_copy_mode_exec),
};

#[unsafe(no_mangle)]
static mut cmd_clock_mode_entry: cmd_entry = cmd_entry {
    name: c"clock-mode".as_ptr(),
    alias: null_mut(),

    args: args_parse {
        template: c"t:".as_ptr(),
        lower: 0,
        upper: 0,
        cb: None,
    },
    usage: CMD_TARGET_PANE_USAGE.as_ptr(),

    source: unsafe { zeroed() },
    target: cmd_entry_flag {
        flag: b't' as c_char,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_copy_mode_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_copy_mode_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut event = cmdq_get_event(item);
        let mut source = cmdq_get_source(item);
        let mut target = cmdq_get_target(item);
        let mut c = cmdq_get_client(item);
        let mut s = null_mut();
        let mut wp = (*target).wp;

        if args_has(args, b'q') != 0 {
            window_pane_reset_mode_all(wp);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, b'M') != 0 {
            let wp = cmd_mouse_pane(&raw mut (*event).m, &raw mut s, null_mut());
            if wp.is_none() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            if c.is_null() || (*c).session != s {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
        }

        if cmd_get_entry(self_) == &raw mut cmd_clock_mode_entry {
            window_pane_set_mode(wp, null_mut(), &raw mut window_clock_mode, null_mut(), null_mut());
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let swp = if args_has(args, b's') != 0 { (*source).wp } else { wp };
        if window_pane_set_mode(wp, swp, &raw mut window_copy_mode, null_mut(), args) == 0 {
            if args_has(args, b'M') != 0 {
                window_copy_start_drag(c, &raw mut (*event).m);
            }
        }
        if args_has(args, b'u') != 0 {
            window_copy_pageup(wp, 0);
        }
        if args_has(args, b'd') != 0 {
            window_copy_pagedown(wp, 0, args_has(args, b'e'));
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
