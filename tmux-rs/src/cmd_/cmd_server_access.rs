use compat_rs::queue::tailq_foreach;
use libc::{getpwnam, getuid};

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_server_access_entry: cmd_entry = cmd_entry {
    name: c"server-access".as_ptr(),
    alias: null(),

    args: args_parse::new(c"adlrw", 0, 1, None),
    usage: c"[-adlrw] [-t target-pane] [user]".as_ptr(),

    flags: CMD_CLIENT_CANFAIL,
    exec: Some(cmd_server_access_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_server_access_deny(item: *mut cmdq_item, pw: *mut libc::passwd) -> cmd_retval {
    unsafe {
        let mut user = server_acl_user_find((*pw).pw_uid);
        if user.is_null() {
            cmdq_error(item, c"user %s not found".as_ptr(), (*pw).pw_name);
            return cmd_retval::CMD_RETURN_ERROR;
        }
        for loop_ in compat_rs::queue::tailq_foreach_(&raw mut clients).map(NonNull::as_ptr) {
            let uid = proc_get_peer_uid((*loop_).peer);
            if (uid == server_acl_get_uid(user)) {
                (*loop_).exit_message = xstrdup_(c"access not allowed").as_ptr();
                (*loop_).flags |= client_flag::EXIT;
            }
        }
        server_acl_user_deny((*pw).pw_uid);

        return (cmd_retval::CMD_RETURN_NORMAL);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_server_access_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut c = cmdq_get_target_client(item);

        if (args_has_(args, 'l')) {
            server_acl_display(item);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        if (args_count(args) == 0) {
            cmdq_error(item, c"missing user argument".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        let mut name = format_single(item, args_string(args, 0), c, null_mut(), null_mut(), null_mut());
        let mut pw = null_mut();
        if (*name != b'\0' as _) {
            pw = getpwnam(name);
        }
        if pw.is_null() {
            cmdq_error(item, c"unknown user: %s".as_ptr(), name);
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        free_(name);

        if ((*pw).pw_uid == 0 || (*pw).pw_uid == getuid()) {
            cmdq_error(item, c"%s owns the server, can't change access".as_ptr(), (*pw).pw_name);
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        if (args_has_(args, 'a') && args_has_(args, 'd')) {
            cmdq_error(item, c"-a and -d cannot be used together".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        if (args_has_(args, 'w') && args_has_(args, 'r')) {
            cmdq_error(item, c"-r and -w cannot be used together".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        if (args_has_(args, 'd')) {
            return (cmd_server_access_deny(item, pw));
        }
        if (args_has_(args, 'a')) {
            if (!server_acl_user_find((*pw).pw_uid).is_null()) {
                cmdq_error(item, c"user %s is already added".as_ptr(), (*pw).pw_name);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            server_acl_user_allow((*pw).pw_uid);
            /* Do not return - allow -r or -w with -a. */
        } else if (args_has_(args, 'r') || args_has_(args, 'w')) {
            /* -r or -w implies -a if user does not exist. */
            if (server_acl_user_find((*pw).pw_uid).is_null()) {
                server_acl_user_allow((*pw).pw_uid);
            }
        }

        if (args_has_(args, 'w')) {
            if (server_acl_user_find((*pw).pw_uid).is_null()) {
                cmdq_error(item, c"user %s not found".as_ptr(), (*pw).pw_name);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            server_acl_user_allow_write((*pw).pw_uid);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        if (args_has_(args, 'r')) {
            if (server_acl_user_find((*pw).pw_uid).is_null()) {
                cmdq_error(item, c"user %s not found".as_ptr(), (*pw).pw_name);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            server_acl_user_deny_write((*pw).pw_uid);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
