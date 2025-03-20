use crate::*;

#[unsafe(no_mangle)]
static mut cmd_kill_pane_entry: cmd_entry = cmd_entry {
    name: c"kill-pane".as_ptr(),
    alias: c"killp".as_ptr(),

    args: args_parse::new(c"at:", 0, 0, None),
    usage: c"[-a] [-t target-client]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_kill_pane_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_kill_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut wl = (*target).wl;
        let mut wp = (*target).wp;

        if args_has(args, b'a') != 0 {
            server_unzoom_window((*wl).window);
            for loopwp in tailq_foreach::<_, discr_entry>(&raw mut (*(*wl).window).panes).map(NonNull::as_ptr) {
                if (loopwp == wp) {
                    continue;
                }
                server_client_remove_pane(loopwp);
                layout_close_pane(loopwp);
                window_remove_pane((*wl).window, loopwp);
            }
            server_redraw_window((*wl).window);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        server_kill_pane(wp);
        return cmd_retval::CMD_RETURN_NORMAL;
    }
}
