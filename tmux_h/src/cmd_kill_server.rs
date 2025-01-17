use libc::{SIGTERM, getpid, kill};

use crate::{
    CMD_STARTSERVER, args_parse, cmd, cmd_entry, cmd_entry_flag, cmd_find_type, cmd_retval,
    cmdq_item,
};

unsafe extern "C" {
    fn cmd_get_entry(_: *mut cmd) -> *const cmd_entry;
}

#[unsafe(no_mangle)]
pub static mut cmd_kill_server_entry: cmd_entry = cmd_entry {
    name: c"kill-server".as_ptr(),
    alias: std::ptr::null(),
    args: args_parse {
        template: c"".as_ptr(),
        lower: 0,
        upper: 0,
        cb: None,
    },
    usage: c"".as_ptr(),
    source: cmd_entry_flag {
        flag: 0,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },
    target: cmd_entry_flag {
        flag: 0,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },
    flags: 0,
    exec: Some(cmd_kill_server_exec),
};

#[unsafe(no_mangle)]
pub static mut cmd_start_server_entry: cmd_entry = cmd_entry {
    name: c"start-server".as_ptr(),
    alias: c"start".as_ptr(),
    args: args_parse {
        template: c"".as_ptr(),
        lower: 0,
        upper: 0,
        cb: None,
    },
    usage: c"".as_ptr(),
    source: cmd_entry_flag {
        flag: 0,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },
    target: cmd_entry_flag {
        flag: 0,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },
    flags: CMD_STARTSERVER as _,
    exec: Some(cmd_kill_server_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_kill_server_exec(self_: *mut cmd, _: *mut cmdq_item) -> cmd_retval {
    unsafe {
        if cmd_get_entry(self_) == &raw const cmd_kill_server_entry {
            kill(getpid(), SIGTERM);
        }
    }

    cmd_retval::CMD_RETURN_NORMAL
}
