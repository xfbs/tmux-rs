use core::mem::zeroed;
use core::ptr::null;

use libc::{SIGTERM, getpid, kill};

use crate::{
    args_parse, cmd, cmd_entry, cmd_entry_flag, cmd_find_type, cmd_flag, cmd_get_entry, cmd_retval, cmdq_item,
};

#[unsafe(no_mangle)]
pub static mut cmd_kill_server_entry: cmd_entry = cmd_entry {
    name: c"kill-server".as_ptr(),
    alias: null(),

    args: args_parse::new(c"", 0, 0, None),
    usage: c"".as_ptr(),

    flags: cmd_flag::empty(),
    exec: Some(cmd_kill_server_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
pub static mut cmd_start_server_entry: cmd_entry = cmd_entry {
    name: c"start-server".as_ptr(),
    alias: c"start".as_ptr(),
    args: args_parse::new(c"", 0, 0, None),
    usage: c"".as_ptr(),
    flags: cmd_flag::CMD_STARTSERVER,
    exec: Some(cmd_kill_server_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_kill_server_exec(self_: *mut cmd, _: *mut cmdq_item) -> cmd_retval {
    unsafe {
        if cmd_get_entry(self_) == &raw mut cmd_kill_server_entry {
            kill(getpid(), SIGTERM);
        }
    }

    cmd_retval::CMD_RETURN_NORMAL
}
