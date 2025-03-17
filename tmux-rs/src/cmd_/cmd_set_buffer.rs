use crate::*;

#[unsafe(no_mangle)]
static mut cmd_set_buffer_entry: cmd_entry = cmd_entry {
    name: c"set-buffer".as_ptr(),
    alias: c"setb".as_ptr(),

    args: args_parse::new(c"ab:t:n:w", 0, 1, None),
    usage: c"[-aw] [-b buffer-name] [-n new-buffer-name] [-t target-client] data".as_ptr(),

    flags: CMD_AFTERHOOK | CMD_CLIENT_TFLAG | CMD_CLIENT_CANFAIL,
    exec: Some(cmd_set_buffer_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_delete_buffer_entry: cmd_entry = cmd_entry {
    name: c"delete-buffer".as_ptr(),
    alias: c"deleteb".as_ptr(),

    args: args_parse::new(c"b:", 0, 0, None),
    usage: CMD_BUFFER_USAGE.as_ptr(),

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_set_buffer_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_set_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut tc = cmdq_get_target_client(item);
        let mut pb;
        let mut cause = null_mut();
        let mut olddata;
        let mut newsize: usize;

        let mut bufname = args_get_(args, 'b');
        if bufname.is_null() {
            pb = null_mut();
        } else {
            pb = paste_get_name(bufname);
        }

        if (cmd_get_entry(self_) == &raw mut cmd_delete_buffer_entry) {
            if (pb.is_null()) {
                if !bufname.is_null() {
                    cmdq_error(item, c"unknown buffer: %s".as_ptr(), bufname);
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
                pb = paste_get_top(&raw mut bufname);
            }
            if (pb.is_null()) {
                cmdq_error(item, c"no buffer".as_ptr());
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            paste_free(NonNull::new_unchecked(pb));
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        if (args_has_(args, 'n')) {
            if (pb.is_null()) {
                if (!bufname.is_null()) {
                    cmdq_error(item, c"unknown buffer: %s".as_ptr(), bufname);
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
                pb = paste_get_top(&raw mut bufname);
            }
            if pb.is_null() {
                cmdq_error(item, c"no buffer".as_ptr());
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            if (paste_rename(bufname, args_get_(args, 'n'), &raw mut cause) != 0) {
                cmdq_error(item, c"%s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        if (args_count(args) != 1) {
            cmdq_error(item, c"no data specified".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        let newsize = strlen(args_string(args, 0));
        if newsize == 0 {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        let mut bufsize = 0;
        let mut bufdata = null_mut();

        if let Some(pb_non_null) = NonNull::new(pb)
            && args_has_(args, 'a')
        {
            olddata = paste_buffer_data_(pb_non_null, &mut bufsize);
            bufdata = xmalloc(bufsize).as_ptr().cast();
            memcpy_(bufdata, olddata, bufsize);
        }

        bufdata = xrealloc_(bufdata, bufsize + newsize).as_ptr();
        memcpy_(bufdata.add(bufsize), args_string(args, 0), newsize);
        bufsize += newsize;

        if (paste_set(bufdata, bufsize, bufname, &raw mut cause) != 0) {
            cmdq_error(item, c"%s".as_ptr(), cause);
            free_(bufdata);
            free_(cause);
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        if (args_has_(args, 'w') && !tc.is_null()) {
            tty_set_selection(&raw mut (*tc).tty, c"".as_ptr(), bufdata, bufsize);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
