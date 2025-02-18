use crate::*;

#[unsafe(no_mangle)]
static mut cmd_respawn_pane_entry: cmd_entry = cmd_entry {
    name: c"respawn-pane".as_ptr(),
    alias: c"respawnp".as_ptr(),

    args: args_parse::new(c"c:e:kt:", 0, -1, None),
    usage: c"[-k] [-c start-directory] [-e environment] [-t target-pane] [shell-command]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: 0,
    exec: Some(cmd_respawn_pane_exec),
    source: unsafe { zeroed() },
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_respawn_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut sc: spawn_context = unsafe { zeroed() };
        let mut s = (*target).s;
        let mut wl = (*target).wl;
        let mut wp = (*target).wp;
        let mut cause = null_mut();

        sc.item = item;
        sc.s = s;
        sc.wl = wl;

        sc.wp0 = wp;

        args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);
        sc.environ = environ_create();

        let mut av = args_first_value(args, b'e');
        while !av.is_null() {
            environ_put(sc.environ, (*av).union_.string, 0);
            av = args_next_value(av);
        }

        sc.idx = -1;
        sc.cwd = args_get(args, b'c');

        sc.flags = SPAWN_RESPAWN;
        if (args_has(args, b'k')) != 0 {
            sc.flags |= SPAWN_KILL;
        }

        if (spawn_pane(&raw mut sc, &raw mut cause).is_null()) {
            cmdq_error(item, c"respawn pane failed: %s".as_ptr(), cause);
            free_(cause);
            if !sc.argv.is_null() {
                cmd_free_argv(sc.argc, sc.argv);
            }
            environ_free(sc.environ);
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        (*wp).flags |= PANE_REDRAW;
        server_redraw_window_borders((*wp).window);
        server_status_window((*wp).window);

        if !sc.argv.is_null() {
            cmd_free_argv(sc.argc, sc.argv);
        }
        environ_free(sc.environ);
        cmd_retval::CMD_RETURN_NORMAL
    }
}
