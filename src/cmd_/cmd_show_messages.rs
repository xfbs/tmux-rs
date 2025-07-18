// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::compat::queue::{list_foreach, tailq_foreach_reverse};

const SHOW_MESSAGES_TEMPLATE: &CStr = c"#{t/p:message_time}: #{message_text}";

pub static mut cmd_show_messages_entry: cmd_entry = cmd_entry {
    name: c"show-messages".as_ptr(),
    alias: c"showmsgs".as_ptr(),

    args: args_parse::new(c"JTt:", 0, 0, None),
    usage: c"[-JT] [-t target-client]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: Some(cmd_show_messages_exec),
    ..unsafe { zeroed() }
};

unsafe extern "C" fn cmd_show_messages_terminals(
    self_: *mut cmd,
    item: *mut cmdq_item,
    mut blank: i32,
) -> c_int {
    unsafe {
        let args = cmd_get_args(self_);
        let tc = cmdq_get_target_client(item);

        let mut n = 0u32;
        for term in list_foreach::<_, discr_entry>(&raw mut tty_terms).map(NonNull::as_ptr) {
            if args_has(args, b't') != 0 && term != (*tc).tty.term {
                continue;
            }
            if blank != 0 {
                cmdq_print!(item, "");
                blank = 0;
            }
            cmdq_print!(
                item,
                "Terminal {}: {} for {}, flags=0x{:x}:",
                n,
                _s((*term).name),
                _s((*(*(*term).tty).client).name),
                (*term).flags,
            );
            n += 1;
            for i in 0..tty_term_ncodes() {
                cmdq_print!(
                    item,
                    "{}",
                    _s(tty_term_describe(term, tty_code_code::try_from(i).unwrap())),
                );
            }
        }
        (n != 0) as i32
    }
}

unsafe extern "C" fn cmd_show_messages_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);

        let mut done = false;
        let mut blank = 0;
        if args_has_(args, 'T') {
            blank = cmd_show_messages_terminals(self_, item, blank);
            done = true;
        }
        if args_has_(args, 'J') {
            job_print_summary(item, blank);
            done = true;
        }
        if done {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let ft = format_create_from_target(item);

        for msg in tailq_foreach_reverse(&raw mut crate::server::message_log).map(NonNull::as_ptr) {
            format_add!(ft, c"message_text".as_ptr(), "{}", _s((*msg).msg));
            format_add!(ft, c"message_number".as_ptr(), "{}", (*msg).msg_num,);
            format_add_tv(ft, c"message_time".as_ptr(), &raw mut (*msg).msg_time);

            let s = format_expand(ft, SHOW_MESSAGES_TEMPLATE.as_ptr());
            cmdq_print!(item, "{}", _s(s));
            free_(s);
        }
        format_free(ft);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
