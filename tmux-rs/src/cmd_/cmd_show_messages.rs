use crate::*;

use crate::compat::queue::{list_foreach, tailq_foreach_reverse};

const SHOW_MESSAGES_TEMPLATE: &CStr = c"#{t/p:message_time}: #{message_text}";

#[unsafe(no_mangle)]
static mut cmd_show_messages_entry: cmd_entry = cmd_entry {
    name: c"show-messages".as_ptr(),
    alias: c"showmsgs".as_ptr(),

    args: args_parse::new(c"JTt:", 0, 0, None),
    usage: c"[-JT] [-t target-client]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_TFLAG),
    exec: Some(cmd_show_messages_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_messages_terminals(self_: *mut cmd, item: *mut cmdq_item, blank: i32) -> c_int {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut tc = cmdq_get_target_client(item);
        let mut blank = 0;

        let mut n = 0u32;
        for term in list_foreach::<_, discr_entry>(&raw mut tty_terms).map(NonNull::as_ptr) {
            if (args_has(args, b't') != 0 && term != (*tc).tty.term) {
                continue;
            }
            if (blank != 0) {
                cmdq_print(item, c"%s".as_ptr(), c"".as_ptr());
                blank = 0;
            }
            cmdq_print(item, c"Terminal %u: %s for %s, flags=0x%x:".as_ptr(), n, (*term).name, (*(*(*term).tty).client).name, (*term).flags);
            n += 1;
            for i in 0..tty_term_ncodes() {
                cmdq_print(item, c"%s".as_ptr(), tty_term_describe(term, std::mem::transmute::<_, tty_code_code>(i)));
            }
        }
        (n != 0) as i32
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_messages_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        //struct message_entry	*msg;
        //char			*s;
        //int			 done, blank;
        //struct format_tree	*ft;

        let mut done = 0;
        let mut blank = 0;
        if (args_has(args, b'T') != 0) {
            blank = cmd_show_messages_terminals(self_, item, blank);
            done = 1;
        }
        if (args_has(args, b'J') != 0) {
            job_print_summary(item, blank);
            done = 1;
        }
        if (done != 0) {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        let mut ft = format_create_from_target(item);

        unsafe extern "C" {
            pub static mut message_log: message_list; // TODO remove
        }

        for msg in tailq_foreach_reverse(&raw mut message_log).map(NonNull::as_ptr) {
            format_add(ft, c"message_text".as_ptr(), c"%s".as_ptr(), (*msg).msg);
            format_add(ft, c"message_number".as_ptr(), c"%u".as_ptr(), (*msg).msg_num);
            format_add_tv(ft, c"message_time".as_ptr(), &raw mut (*msg).msg_time);

            let s = format_expand(ft, SHOW_MESSAGES_TEMPLATE.as_ptr());
            cmdq_print(item, c"%s".as_ptr(), s);
            free_(s);
        }
        format_free(ft);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
