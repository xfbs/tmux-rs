use crate::*;

const DISPLAY_MESSAGE_TEMPLATE: &CStr = c"[#{session_name}] #{window_index}:#{window_name}, current pane #{pane_index} - (%H:%M %d-%b-%y)";

#[unsafe(no_mangle)]
static mut cmd_display_message_entry: cmd_entry = cmd_entry {
    name: c"display-message".as_ptr(),
    alias: c"display".as_ptr(),

    args: args_parse::new(c"ac:d:lINpt:F:v", 0, 1, None),
    usage: c"[-aIlNpv] [-c target-client] [-d delay] [-F format] [-t target-pane] [message]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_CFLAG).union(cmd_flag::CMD_CLIENT_CANFAIL),
    exec: Some(cmd_display_message_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_display_message_each(key: *const c_char, value: *const c_char, arg: *mut c_void) {
    let item = arg as *mut cmdq_item;

    unsafe {
        cmdq_print(item, c"%s=%s".as_ptr(), key, value);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_display_message_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut tc = cmdq_get_target_client(item);
        let mut s = (*target).s;
        let mut wl = (*target).wl;
        let mut wp = (*target).wp;
        let mut cause: *mut c_char = null_mut();
        let mut delay = -1;
        let mut Nflag = args_has(args, b'N');
        let mut count = args_count(args);

        if (args_has_(args, 'I')) {
            if (wp.is_null()) {
                return (cmd_retval::CMD_RETURN_NORMAL);
            }
            match (window_pane_start_input(wp, item, &raw mut cause)) {
                -1 => {
                    cmdq_error(item, c"%s".as_ptr(), cause);
                    free_(cause);
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
                1 => {
                    return (cmd_retval::CMD_RETURN_NORMAL);
                }
                0 => {
                    return (cmd_retval::CMD_RETURN_WAIT);
                }
                _ => (),
            }
        }

        if (args_has_(args, 'F') && count != 0) {
            cmdq_error(item, c"only one of -F or argument must be given".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        if (args_has_(args, 'd')) {
            delay = args_strtonum(args, b'd', 0, u32::MAX as i64, &raw mut cause);
            if (!cause.is_null()) {
                cmdq_error(item, c"delay %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        let mut template = if (count != 0) { args_string(args, 0) } else { args_get_(args, 'F') };
        if (template.is_null()) {
            template = DISPLAY_MESSAGE_TEMPLATE.as_ptr();
        }

        /*
         * -c is intended to be the client where the message should be
         * displayed if -p is not given. But it makes sense to use it for the
         * formats too, assuming it matches the session. If it doesn't, use the
         * best client for the session.
         */
        let mut c = if (!tc.is_null() && (*tc).session == s) {
            tc
        } else if (!s.is_null()) {
            cmd_find_best_client(s)
        } else {
            null_mut()
        };

        let flags = if (args_has_(args, 'v')) { format_flags::FORMAT_VERBOSE } else { format_flags::empty() };
        let mut ft = format_create(cmdq_get_client(item), item, FORMAT_NONE, flags);
        format_defaults(ft, c, NonNull::new(s), NonNull::new(wl), NonNull::new(wp));

        if (args_has_(args, 'a')) {
            format_each(ft, Some(cmd_display_message_each), item as _);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        let msg = if (args_has_(args, 'l')) { xstrdup(template).as_ptr() } else { format_expand_time(ft, template) };

        if (cmdq_get_client(item).is_null()) {
            cmdq_error(item, c"%s".as_ptr(), msg);
        } else if (args_has_(args, 'p')) {
            cmdq_print(item, c"%s".as_ptr(), msg);
        } else if (!tc.is_null() && (*tc).flags.intersects(client_flag::CONTROL)) {
            let evb = evbuffer_new();
            if (evb.is_null()) {
                fatalx(c"out of memory");
            }
            evbuffer_add_printf(evb, c"%%message %s".as_ptr(), msg);
            server_client_print(tc, 0, evb);
            evbuffer_free(evb);
        } else if (!tc.is_null()) {
            status_message_set(tc, delay as i32, 0, Nflag, c"%s".as_ptr(), msg);
        }
        free_(msg);

        format_free(ft);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
