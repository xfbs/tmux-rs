use compat_rs::queue::tailq_foreach;

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_detach_client_entry: cmd_entry = cmd_entry {
    name: c"detach-client".as_ptr(),
    alias: c"detach".as_ptr(),

    args: args_parse::new(c"aE:s:t:P", 0, 0, None),
    usage: c"[-aP] [-E shell-command] [-s target-session] [-t target-client]".as_ptr(),

    source: cmd_entry_flag::new(b's', cmd_find_type::CMD_FIND_SESSION, CMD_FIND_CANFAIL),

    flags: CMD_READONLY | CMD_CLIENT_TFLAG,
    exec: Some(cmd_detach_client_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_suspend_client_entry: cmd_entry = cmd_entry {
    name: c"suspend-client".as_ptr(),
    alias: c"suspendc".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-client]".as_ptr(),

    flags: CMD_CLIENT_TFLAG,
    exec: Some(cmd_detach_client_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_detach_client_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut source = cmdq_get_source(item);
        let mut tc = cmdq_get_target_client(item);
        let mut cmd = args_get(args, b'E');

        if cmd_get_entry(self_) == &raw mut cmd_suspend_client_entry {
            server_client_suspend(tc);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut msgtype = if args_has(args, b'P') != 0 {
            msgtype::MSG_DETACHKILL
        } else {
            msgtype::MSG_DETACH
        };

        let mut s: *mut session = null_mut();
        if args_has(args, b's') != 0 {
            s = (*source).s;
            if s.is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            tailq_foreach(&raw mut clients, |loop_| {
                if ((*loop_).session == s) {
                    if !cmd.is_null() {
                        server_client_exec(loop_, cmd);
                    } else {
                        server_client_detach(loop_, msgtype);
                    }
                }
                ControlFlow::<(), ()>::Continue(())
            });
            return cmd_retval::CMD_RETURN_STOP;
        }

        if args_has(args, b'a') != 0 {
            tailq_foreach(&raw mut clients, |loop_| {
                if !(*loop_).session.is_null() && loop_ != tc {
                    if !cmd.is_null() {
                        server_client_exec(loop_, cmd);
                    } else {
                        server_client_detach(loop_, msgtype);
                    }
                }
                ControlFlow::<(), ()>::Continue(())
            });
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if !cmd.is_null() {
            server_client_exec(tc, cmd);
        } else {
            server_client_detach(tc, msgtype);
        }
        return cmd_retval::CMD_RETURN_STOP;
    }
}
