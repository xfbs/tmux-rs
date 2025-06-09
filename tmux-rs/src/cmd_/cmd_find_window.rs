use crate::*;

#[unsafe(no_mangle)]
static mut cmd_find_window_entry: cmd_entry = cmd_entry {
    name: c"find-window".as_ptr(),
    alias: c"findw".as_ptr(),

    args: args_parse::new(c"CiNrt:TZ", 1, 1, None),
    usage: c"[-CiNrTZ] [-t target-pane] match-string".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_find_window_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_find_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut wp = (*target).wp;
        let mut s = args_string(args, 0);
        let mut suffix = c"".as_ptr();
        let mut star = c"*".as_ptr();

        let mut c = args_has_(args, 'C');
        let mut n = args_has_(args, 'N');
        let mut t = args_has_(args, 'T');

        if args_has(args, b'r') != 0 {
            star = c"".as_ptr();
        }
        if args_has(args, b'r') != 0 && args_has(args, b'i') != 0 {
            suffix = c"/ri".as_ptr();
        } else if (args_has(args, b'r') != 0) {
            suffix = c"/r".as_ptr();
        } else if args_has(args, b'i') != 0 {
            suffix = c"/i".as_ptr();
        }

        if !c && !n && !t {
            c = true;
            n = true;
            t = true;
        }

        let mut filter = xcalloc_::<args_value>(1).as_ptr();
        (*filter).type_ = args_type::ARGS_STRING;

        if c && n && t {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{||:#{C%s:%s},#{||:#{m%s:%s%s%s,#{window_name}},#{m%s:%s%s%s,#{pane_title}}}}"
                    .as_ptr(),
                suffix,
                s,
                suffix,
                star,
                s,
                star,
                suffix,
                star,
                s,
                star,
            );
        } else if c && n {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{||:#{C%s:%s},#{m%s:%s%s%s,#{window_name}}}".as_ptr(),
                suffix,
                s,
                suffix,
                star,
                s,
                star,
            );
        } else if c && t {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{||:#{C%s:%s},#{m%s:%s%s%s,#{pane_title}}}".as_ptr(),
                suffix,
                s,
                suffix,
                star,
                s,
                star,
            );
        } else if n && t {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{||:#{m%s:%s%s%s,#{window_name}},#{m%s:%s%s%s,#{pane_title}}}".as_ptr(),
                suffix,
                star,
                s,
                star,
                suffix,
                star,
                s,
                star,
            );
        } else if c {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{C%s:%s}".as_ptr(),
                suffix,
                s,
            );
        } else if n {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{m%s:%s%s%s,#{window_name}}".as_ptr(),
                suffix,
                star,
                s,
                star,
            );
        } else {
            xasprintf(
                &raw mut (*filter).union_.string,
                c"#{m%s:%s%s%s,#{pane_title}}".as_ptr(),
                suffix,
                star,
                s,
                star,
            );
        }

        let mut new_args: *mut args = args_create();
        if args_has_(args, 'Z') {
            args_set(new_args, b'Z', null_mut(), 0);
        }
        args_set(new_args, b'f', filter, 0);

        window_pane_set_mode(wp, null_mut(), &raw mut window_tree_mode, target, new_args);
        args_free(new_args);

        cmd_retval::CMD_RETURN_NORMAL
    }
}
