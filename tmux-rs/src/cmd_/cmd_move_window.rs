use crate::*;

#[unsafe(no_mangle)]
static mut cmd_move_window_entry: cmd_entry = cmd_entry {
    name: c"move-window".as_ptr(),
    alias: c"movew".as_ptr(),

    args: args_parse::new(c"abdkrs:t:", 0, 0, None),
    usage: c"[-abdkr] [-s src-window] [-t dst-window]".as_ptr(),

    source: cmd_entry_flag::new(b's', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_move_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_link_window_entry: cmd_entry = cmd_entry {
    name: c"link-window".as_ptr(),
    alias: c"linkw".as_ptr(),

    args: args_parse::new(c"abdks:t:", 0, 0, None),
    usage: c"[-abdk] [-s src-window] [-t dst-window]".as_ptr(),

    source: cmd_entry_flag::new(b's', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_move_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_move_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut source = cmdq_get_source(item);
        let mut target = zeroed();
        let mut tflag = args_get(args, b't');
        let mut src = (*source).s;
        let mut wl = (*source).wl;
        let mut cause = null_mut();

        if (args_has_(args, 'r')) {
            if (cmd_find_target(&raw mut target, item, tflag, cmd_find_type::CMD_FIND_SESSION, CMD_FIND_QUIET) != 0) {
                return (cmd_retval::CMD_RETURN_ERROR);
            }

            session_renumber_windows(target.s);
            recalculate_sizes();
            server_status_session(target.s);

            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        if (cmd_find_target(&raw mut target, item, tflag, cmd_find_type::CMD_FIND_WINDOW, CMD_FIND_WINDOW_INDEX) != 0) {
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        let dst = target.s;
        let mut idx = target.idx;

        let kflag = args_has(args, b'k');
        let dflag = args_has(args, b'd');
        let sflag = args_has_(args, 's');

        let before = args_has(args, b'b');
        if (args_has_(args, 'a') || before != 0) {
            if !target.wl.is_null() {
                idx = winlink_shuffle_up(dst, target.wl, before);
            } else {
                idx = winlink_shuffle_up(dst, (*dst).curw, before);
            }
            if (idx == -1) {
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        if (server_link_window(src, wl, dst, idx, kflag, !dflag, &raw mut cause) != 0) {
            cmdq_error(item, c"%s".as_ptr(), cause);
            free_(cause);
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        if (cmd_get_entry(self_) == &raw mut cmd_move_window_entry) {
            server_unlink_window(src, wl);
        }

        /*
         * Renumber the winlinks in the src session only, the destination
         * session already has the correct winlink id to us, either
         * automatically or specified by -s.
         */
        if !sflag && options_get_number((*src).options, c"renumber-windows".as_ptr()) != 0 {
            session_renumber_windows(src);
        }

        recalculate_sizes();

        cmd_retval::CMD_RETURN_NORMAL
    }
}
