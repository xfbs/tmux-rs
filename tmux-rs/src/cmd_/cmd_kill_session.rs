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
use crate::*;

use crate::compat::tree::rb_foreach;

#[unsafe(no_mangle)]
static mut cmd_kill_session_entry: cmd_entry = cmd_entry {
    name: c"kill-session".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"aCt:", 0, 0, None),
    usage: c"[-aC] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),
    source: unsafe { zeroed() },

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
