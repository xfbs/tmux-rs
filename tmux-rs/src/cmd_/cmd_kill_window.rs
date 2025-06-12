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

use crate::compat::tree::{rb_foreach, rb_next, rb_prev};

#[unsafe(no_mangle)]
static mut cmd_kill_window_entry: cmd_entry = cmd_entry {
    name: c"kill-window".as_ptr(),
    alias: c"killw".as_ptr(),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: c"[-a] [-t target-window]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_kill_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_unlink_window_entry: cmd_entry = cmd_entry {
    name: c"unlink-window".as_ptr(),
    alias: c"unlinkw".as_ptr(),

    args: args_parse::new(c"kt:", 0, 0, None),
    usage: c"[-k] [-t target-window]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_kill_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_kill_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let wl = (*target).wl;
        //*loop;
        let w = (*wl).window;
        let s = (*target).s;
        let mut found = 0u32;

        if cmd_get_entry(self_) == &raw mut cmd_unlink_window_entry {
            if !args_has(args, b'k') != 0 && session_is_linked(s, w) == 0 {
                cmdq_error(item, c"window only linked to one session".as_ptr());
                return cmd_retval::CMD_RETURN_ERROR;
            }
            server_unlink_window(s, wl);
            recalculate_sizes();
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if args_has(args, b'a') != 0 {
            if rb_prev(wl).is_null() && rb_next(wl).is_null() {
                return cmd_retval::CMD_RETURN_NORMAL;
            }

            /* Kill all windows except the current one. */
            loop {
                found = 0;
                for loop_ in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
                    if (*loop_).window != (*wl).window {
                        server_kill_window((*loop_).window, 0);
                        found += 1;
                        break;
                    }
                }

                if found == 0 {
                    break;
                }
            }

            /*
             * If the current window appears in the session more than once,
             * kill it as well.
             */
            found = 0;
            for loop_ in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
                if (*loop_).window == (*wl).window {
                    found += 1;
                }
            }
            if found > 1 {
                {
                    server_kill_window((*wl).window, 0);
                }
            }

            server_renumber_all();
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        server_kill_window((*wl).window, 1);
        cmd_retval::CMD_RETURN_NORMAL
    }
}
