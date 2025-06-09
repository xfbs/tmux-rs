use crate::*;

#[unsafe(no_mangle)]
static mut cmd_choose_tree_entry: cmd_entry = cmd_entry {
    name: c"choose-tree".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:GK:NO:rst:wZ", 0, 1, Some(cmd_choose_tree_args_parse)),
    usage: c"[-GNrswZ] [-F format] [-f filter] [-K key-format] [-O sort-order] [-t target-pane] [template]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

#[unsafe(no_mangle)]
static mut cmd_choose_client_entry: cmd_entry = cmd_entry {
    name: c"choose-client".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:K:NO:rt:Z", 0, 1, Some(cmd_choose_tree_args_parse)),
    usage: c"[-NrZ] [-F format] [-f filter] [-K key-format] [-O sort-order] [-t target-pane] [template]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

#[unsafe(no_mangle)]
static mut cmd_choose_buffer_entry: cmd_entry = cmd_entry {
    name: c"choose-buffer".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:K:NO:rt:Z", 0, 1, Some(cmd_choose_tree_args_parse)),
    usage: c"[-NrZ] [-F format] [-f filter] [-K key-format] [-O sort-order] [-t target-pane] [template]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

#[unsafe(no_mangle)]
static mut cmd_customize_mode_entry: cmd_entry = cmd_entry {
    name: c"customize-mode".as_ptr(),
    alias: null_mut(),

    args: args_parse::new(c"F:f:Nt:Z", 0, 0, None),
    usage: c"[-NZ] [-F format] [-f filter] [-t target-pane]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),
    source: unsafe { zeroed() },

    flags: cmd_flag::empty(),
    exec: Some(cmd_choose_tree_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_choose_tree_args_parse(
    _args: *mut args,
    _idx: u32,
    _cause: *mut *mut c_char,
) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_choose_tree_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut wp = (*target).wp;

        let mut mode = if cmd_get_entry(self_) == &raw mut cmd_choose_buffer_entry {
            if paste_is_empty() != 0 {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            &raw mut window_buffer_mode
        } else if cmd_get_entry(self_) == &raw mut cmd_choose_client_entry {
            if server_client_how_many() == 0 {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            &raw mut window_client_mode
        } else if cmd_get_entry(self_) == &raw mut cmd_customize_mode_entry {
            &raw mut window_customize_mode
        } else {
            &raw mut window_tree_mode
        };

        window_pane_set_mode(wp, null_mut(), mode, target, args);
        cmd_retval::CMD_RETURN_NORMAL
    }
}
