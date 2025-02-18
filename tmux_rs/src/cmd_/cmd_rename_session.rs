use compat_rs::tree::{rb_insert, rb_remove};
use libc::strcmp;

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_rename_session_entry: cmd_entry = cmd_entry {
    name: c"rename-session".as_ptr(),
    alias: c"rename".as_ptr(),

    args: args_parse::new(c"t:", 1, 1, None),
    usage: c"[-t target-session] new-name".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),
    source: unsafe { zeroed() },

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_rename_session_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_rename_session_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut s = (*target).s;

        let mut tmp = format_single_from_target(item, args_string(args, 0));
        let mut newname = session_check_name(tmp);
        if newname.is_null() {
            cmdq_error(item, c"invalid session: %s".as_ptr(), tmp);
            free_(tmp);
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        free_(tmp);
        if strcmp(newname, (*s).name) == 0 {
            free_(newname);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        if !session_find(newname).is_null() {
            cmdq_error(item, c"duplicate session: %s".as_ptr(), newname);
            free_(newname);
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        rb_remove(&raw mut sessions, s);
        free_((*s).name);
        (*s).name = newname;
        rb_insert(&raw mut sessions, s);

        server_status_session(s);
        notify_session(c"session-renamed".as_ptr(), s);
    }

    cmd_retval::CMD_RETURN_NORMAL
}
