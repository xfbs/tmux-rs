use compat_rs::tree::rb_foreach;

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_kill_session_entry: cmd_entry = cmd_entry {
    name: c"kill-session".as_ptr(),
    alias: null_mut(),

    args: args_parse {
        template: c"aCt:".as_ptr(),
        upper: 0,
        lower: 0,
        cb: None,
    },
    usage: c"[-aC] [-t target-session]".as_ptr(),

    source: unsafe { zeroed() },
    target: cmd_entry_flag {
        flag: b't' as _,
        type_: cmd_find_type::CMD_FIND_SESSION,
        flags: 0,
    },

    flags: cmd_flag::empty(),
    exec: Some(cmd_kill_session_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_kill_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut s = (*target).s;

        if args_has(args, b'C') != 0 {
            for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
                (*(*wl).window).flags &= !WINDOW_ALERTFLAGS;
                (*wl).flags &= !WINLINK_ALERTFLAGS;
            }
            server_redraw_session(s);
        } else if args_has(args, b'a') != 0 {
            for sloop in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
                if sloop != s {
                    server_destroy_session(sloop);
                    session_destroy(sloop, 1, c"cmd_kill_session_exec".as_ptr());
                }
            }
        } else {
            server_destroy_session(s);
            session_destroy(s, 1, c"cmd_kill_session_exec".as_ptr());
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}
