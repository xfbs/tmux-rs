use crate::*;

#[unsafe(no_mangle)]
static mut cmd_select_window_entry: cmd_entry = cmd_entry {
    name: c"select-window".as_ptr(),
    alias: c"selectw".as_ptr(),

    args: args_parse::new(c"lnpTt:", 0, 0, None),
    usage: c"[-lnpT] [-t target-window]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_next_window_entry: cmd_entry = cmd_entry {
    name: c"next-window".as_ptr(),
    alias: c"next".as_ptr(),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: c"[-a] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_previous_window_entry: cmd_entry = cmd_entry {
    name: c"previous-window".as_ptr(),
    alias: c"prev".as_ptr(),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: c"[-a] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_last_window_entry: cmd_entry = cmd_entry {
    name: c"last-window".as_ptr(),
    alias: c"last".as_ptr(),

    args: args_parse::new(c"t:", 0, 0, None),
    usage: c"[-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_select_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_select_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut c = cmdq_get_client(item);
        let mut current = cmdq_get_current(item);
        let mut target = cmdq_get_target(item);
        let mut wl = (*target).wl;
        let mut s = (*target).s;
        //int			 next, previous, last, activity;

        let mut next = (cmd_get_entry(self_) == &raw mut cmd_next_window_entry);
        if (args_has_(args, 'n')) {
            next = true;
        }
        let mut previous = (cmd_get_entry(self_) == &raw mut cmd_previous_window_entry);
        if (args_has_(args, 'p')) {
            previous = true;
        }
        let mut last = (cmd_get_entry(self_) == &raw mut cmd_last_window_entry);
        if (args_has_(args, 'l')) {
            last = true;
        }

        if (next || previous || last) {
            let activity = args_has(args, b'a');
            if (next) {
                if (session_next(s, activity) != 0) {
                    cmdq_error(item, c"no next window".as_ptr());
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
            } else if (previous) {
                if (session_previous(s, activity) != 0) {
                    cmdq_error(item, c"no previous window".as_ptr());
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
            } else {
                #[allow(clippy::collapsible_else_if)]
                if (session_last(s) != 0) {
                    cmdq_error(item, c"no last window".as_ptr());
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
            }
            cmd_find_from_session(current, s, 0);
            server_redraw_session(s);
            cmdq_insert_hook(s, item, current, c"after-select-window".as_ptr());
        } else {
            /*
             * If -T and select-window is invoked on same window as
             * current, switch to previous window.
             */
            if (args_has_(args, 'T') && wl == (*s).curw) {
                if (session_last(s) != 0) {
                    cmdq_error(item, c"no last window".as_ptr());
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                if ((*current).s == s) {
                    cmd_find_from_session(current, s, 0);
                }
                server_redraw_session(s);
            } else if (session_select(s, (*wl).idx) == 0) {
                cmd_find_from_session(current, s, 0);
                server_redraw_session(s);
            }
            cmdq_insert_hook(s, item, current, c"after-select-window".as_ptr());
        }
        if (!c.is_null() && !(*c).session.is_null()) {
            (*(*(*s).curw).window).latest = c as _;
        }
        recalculate_sizes();

        (cmd_retval::CMD_RETURN_NORMAL)
    }
}
