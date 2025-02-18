use crate::*;

#[unsafe(no_mangle)]
static mut cmd_rename_window_entry: cmd_entry = cmd_entry {
    name: c"rename-window".as_ptr(),
    alias: c"renamew".as_ptr(),

    args: args_parse::new(c"t:", 1, 1, None),
    usage: c"[-t target-window] new-name".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),
    source: unsafe { zeroed() },

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_rename_window_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_rename_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut wl = (*target).wl;

        let mut newname = format_single_from_target(item, args_string(args, 0));
        window_set_name((*wl).window, newname);
        options_set_number((*(*wl).window).options, c"automatic-rename".as_ptr(), 0);

        server_redraw_window_borders((*wl).window);
        server_status_window((*wl).window);
        free_(newname);
    }

    cmd_retval::CMD_RETURN_NORMAL
}
