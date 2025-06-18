// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use core::mem::zeroed;
use core::ptr::null;

use libc::{SIGTERM, kill, pid_t};

use crate::{args_parse, cmd, cmd_entry, cmd_flag, cmd_get_entry, cmd_retval, cmdq_item};

pub static mut cmd_kill_server_entry: cmd_entry = cmd_entry {
    name: c"kill-server".as_ptr(),
    alias: null(),

    args: args_parse::new(c"", 0, 0, None),
    usage: c"".as_ptr(),

    flags: cmd_flag::empty(),
    exec: Some(cmd_kill_server_exec),
    ..unsafe { zeroed() }
};

pub static mut cmd_start_server_entry: cmd_entry = cmd_entry {
    name: c"start-server".as_ptr(),
    alias: c"start".as_ptr(),
    args: args_parse::new(c"", 0, 0, None),
    usage: c"".as_ptr(),
    flags: cmd_flag::CMD_STARTSERVER,
    exec: Some(cmd_kill_server_exec),
    ..unsafe { zeroed() }
};

unsafe extern "C" fn cmd_kill_server_exec(self_: *mut cmd, _: *mut cmdq_item) -> cmd_retval {
    unsafe {
        if cmd_get_entry(self_) == &raw mut cmd_kill_server_entry {
            kill(std::process::id() as pid_t, SIGTERM);
        }
    }

    cmd_retval::CMD_RETURN_NORMAL
}
