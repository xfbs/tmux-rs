use crate::*;

#[unsafe(no_mangle)]
static mut cmd_lock_server_entry: cmd_entry = cmd_entry {
    name: c"lock-server".as_ptr(),
    alias: c"lock".as_ptr(),

    args: args_parse::new(c"", 0, 0, None),
    usage: c"".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_lock_server_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_lock_session_entry: cmd_entry = cmd_entry {
    name: c"lock-session".as_ptr(),
    alias: c"locks".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_lock_server_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_lock_client_entry: cmd_entry = cmd_entry {
    name: c"lock-client".as_ptr(),
    alias: c"lockc".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-client]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: Some(cmd_lock_server_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_lock_server_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut target = cmdq_get_target(item);
        let mut tc = cmdq_get_target_client(item);

        if cmd_get_entry(self_) == &raw mut cmd_lock_server_entry {
            server_lock();
        } else if cmd_get_entry(self_) == &raw mut cmd_lock_session_entry {
            server_lock_session((*target).s);
        } else {
            server_lock_client(tc);
        }
        recalculate_sizes();
    }

    cmd_retval::CMD_RETURN_NORMAL
}
