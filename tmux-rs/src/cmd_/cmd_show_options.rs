use crate::*;

#[unsafe(no_mangle)]
static mut cmd_show_options_entry: cmd_entry = cmd_entry {
    name: c"show-options".as_ptr(),
    alias: c"show".as_ptr(),

    args: args_parse::new(c"AgHpqst:vw", 0, 1, None),
    usage: c"[-AgHpqsvw] [-t target-pane] [option]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_options_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_show_window_options_entry: cmd_entry = cmd_entry {
    name: c"show-window-options".as_ptr(),
    alias: c"showw".as_ptr(),

    args: args_parse::new(c"gvt:", 0, 1, None),
    usage: c"[-gv] [-t target-window] [option]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_options_exec),

    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_show_hooks_entry: cmd_entry = cmd_entry {
    name: c"show-hooks".as_ptr(),
    alias: null(),

    args: args_parse::new(c"gpt:w", 0, 1, None),
    usage: c"[-gpw] [-t target-pane]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_options_exec),

    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_options_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut oo: *mut options = null_mut();
        let mut argument: *mut c_char = null_mut();
        let mut name: *mut c_char = null_mut();
        let mut cause: *mut c_char = null_mut();

        let mut window = 0;
        let mut idx = 0;
        let mut ambiguous = 0;
        let mut parent = 0;
        let mut scope = 0;
        let mut o: *mut options_entry = null_mut();

        let mut window = (cmd_get_entry(self_) == &raw mut cmd_show_window_options_entry) as i32;

        'fail: {
            'out: {
                if (args_count(args) == 0) {
                    let scope = options_scope_from_flags(args, window, target, &raw mut oo, &raw mut cause);
                    if (scope == OPTIONS_TABLE_NONE) {
                        if (args_has_(args, 'q')) {
                            return cmd_retval::CMD_RETURN_NORMAL;
                        }
                        cmdq_error(item, c"%s".as_ptr(), cause);
                        free_(cause);
                        return cmd_retval::CMD_RETURN_ERROR;
                    }
                    return cmd_show_options_all(self_, item, scope, oo);
                }
                argument = format_single_from_target(item, args_string(args, 0));

                name = options_match(argument, &raw mut idx, &raw mut ambiguous);
                if (name.is_null()) {
                    if (args_has_(args, 'q')) {
                        break 'out;
                    }
                    if (ambiguous != 0) {
                        cmdq_error(item, c"ambiguous option: %s".as_ptr(), argument);
                    } else {
                        cmdq_error(item, c"invalid option: %s".as_ptr(), argument);
                    }
                    break 'fail;
                }
                let scope = options_scope_from_name(args, window, name, target, &raw mut oo, &raw mut cause);
                if (scope == OPTIONS_TABLE_NONE) {
                    if (args_has_(args, 'q')) {
                        break 'out;
                    }
                    cmdq_error(item, c"%s".as_ptr(), cause);
                    free_(cause);
                    break 'fail;
                }
                o = options_get_only(oo, name);
                if (args_has_(args, 'A') && o.is_null()) {
                    o = options_get(oo, name);
                    parent = 1;
                } else {
                    parent = 0;
                }
                if (!o.is_null()) {
                    cmd_show_options_print(self_, item, o, idx, parent);
                } else if (*name == b'@' as _) {
                    if (args_has_(args, 'q')) {
                        break 'out;
                    }
                    cmdq_error(item, c"invalid option: %s".as_ptr(), argument);
                    break 'fail;
                }
            }
            // out:
            free_(name);
            free_(argument);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        // fail:
        free_(name);
        free_(argument);
        return cmd_retval::CMD_RETURN_ERROR;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_show_options_print(
    self_: *mut cmd,
    item: *mut cmdq_item,
    o: *mut options_entry,
    mut idx: i32,
    parent: i32,
) {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut a: *mut options_array_item = null_mut();
        let mut name = options_name(o);
        let mut value = null_mut();
        let mut tmp = null_mut();
        let mut escaped = null_mut();

        if (idx != -1) {
            xasprintf(&raw mut tmp, c"%s[%d]".as_ptr(), name, idx);
            name = tmp;
        } else {
            if (options_is_array(o) != 0) {
                a = options_array_first(o);
                if (a.is_null()) {
                    if (!args_has_(args, 'v')) {
                        cmdq_print(item, c"%s".as_ptr(), name);
                    }
                    return;
                }
                while (!a.is_null()) {
                    idx = options_array_item_index(a) as i32;
                    cmd_show_options_print(self_, item, o, idx, parent);
                    a = options_array_next(a);
                }
                return;
            }
        }

        value = options_to_string(o, idx, 0);
        if (args_has_(args, 'v')) {
            cmdq_print(item, c"%s".as_ptr(), value);
        } else if (options_is_string(o) != 0) {
            escaped = args_escape(value);
            if (parent != 0) {
                cmdq_print(item, c"%s* %s".as_ptr(), name, escaped);
            } else {
                cmdq_print(item, c"%s %s".as_ptr(), name, escaped);
            }
            free_(escaped);
        } else {
            if (parent != 0) {
                cmdq_print(item, c"%s* %s".as_ptr(), name, value);
            } else {
                cmdq_print(item, c"%s %s".as_ptr(), name, value);
            }
        }
        free_(value);

        free_(tmp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_show_options_all(
    self_: *mut cmd,
    item: *mut cmdq_item,
    scope: i32,
    oo: *mut options,
) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut o: *mut options_entry = null_mut();
        let mut a: *mut options_array_item = null_mut();
        let mut parent = 0;

        if (cmd_get_entry(self_) != &raw mut cmd_show_hooks_entry) {
            o = options_first(oo);
            while (!o.is_null()) {
                if (options_table_entry(o).is_null()) {
                    cmd_show_options_print(self_, item, o, -1, 0);
                }
                o = options_next(o);
            }
        }
        let mut oe = &raw const options_table as *const options_table_entry;
        while !(*oe).name.is_null() {
            if (!(*oe).scope & scope != 0) {
                oe = oe.add(1);
                continue;
            }

            if ((cmd_get_entry(self_) != &raw mut cmd_show_hooks_entry
                && !args_has_(args, 'H')
                && ((*oe).flags & OPTIONS_TABLE_IS_HOOK != 0))
                || (cmd_get_entry(self_) == &raw mut cmd_show_hooks_entry
                    && (!(*oe).flags & OPTIONS_TABLE_IS_HOOK != 0)))
            {
                oe = oe.add(1);
                continue;
            }

            o = options_get_only(oo, (*oe).name);
            if (o.is_null()) {
                if (!args_has_(args, 'A')) {
                    oe = oe.add(1);
                    continue;
                }
                o = options_get(oo, (*oe).name);
                if (o.is_null()) {
                    oe = oe.add(1);
                    continue;
                }
                parent = 1;
            } else {
                parent = 0;
            }

            if (options_is_array(o) == 0) {
                cmd_show_options_print(self_, item, o, -1, parent);
            } else if ({
                (a = options_array_first(o));
                a.is_null()
            }) {
                if (!args_has_(args, 'v')) {
                    let name = options_name(o);
                    if (parent != 0) {
                        cmdq_print(item, c"%s*".as_ptr(), name);
                    } else {
                        cmdq_print(item, c"%s".as_ptr(), name);
                    }
                }
            } else {
                while (!a.is_null()) {
                    let idx = options_array_item_index(a);
                    cmd_show_options_print(self_, item, o, idx as i32, parent);
                    a = options_array_next(a);
                }
            }

            oe = oe.add(1);
        }
    }
    cmd_retval::CMD_RETURN_NORMAL
}
