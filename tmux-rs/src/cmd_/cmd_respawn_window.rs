use crate::*;

#[unsafe(no_mangle)]
static mut cmd_respawn_window_entry: cmd_entry = cmd_entry {
    name: c"respawn-window".as_ptr(),
    alias: c"respawnw".as_ptr(),

    args: args_parse::new(c"c:e:kt:", 0, -1, None),
    usage: c"[-k] [-c start-directory] [-e environment] [-t target-window] [shell-command]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),
    source: unsafe { zeroed() },

    flags: 0,
    exec: Some(cmd_respawn_window_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_respawn_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut sc: spawn_context = unsafe { zeroed() };
        let mut tc = cmdq_get_target_client(item);
        let mut s = (*target).s;
        let mut wl = (*target).wl;
        let mut cause: *mut c_char = null_mut();
        // struct args_value	*av;

        sc.item = item;
        sc.s = s;
        sc.wl = wl;
        sc.tc = tc;

        args_to_vector(args, &raw mut sc.argc, &raw mut sc.argv);
        sc.environ = environ_create().as_ptr();

        let mut av = args_first_value(args, b'e');
        while !av.is_null() {
            environ_put(sc.environ, (*av).union_.string, 0);
            av = args_next_value(av);
        }

        sc.idx = -1;
        sc.cwd = args_get(args, b'c');

        sc.flags = SPAWN_RESPAWN;
        if args_has(args, b'k') != 0 {
            sc.flags |= SPAWN_KILL;
        }

        if (spawn_window(&raw mut sc, &raw mut cause).is_null()) {
            cmdq_error(item, c"respawn window failed: %s".as_ptr(), cause);
            free_(cause);
            if !sc.argv.is_null() {
                cmd_free_argv(sc.argc, sc.argv);
            }
            environ_free(sc.environ);
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        server_redraw_window((*wl).window);

        if !sc.argv.is_null() {
            cmd_free_argv(sc.argc, sc.argv);
        }
        environ_free(sc.environ);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
