use libc::{O_APPEND, O_TRUNC};

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_save_buffer_entry: cmd_entry = cmd_entry {
    name: c"save-buffer".as_ptr(),
    alias: c"saveb".as_ptr(),

    args: args_parse::new(c"ab:", 1, 1, None),
    usage: c"[-a] [-b buffer-name] path".as_ptr(),

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_save_buffer_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_show_buffer_entry: cmd_entry = cmd_entry {
    name: c"show-buffer".as_ptr(),
    alias: c"showb".as_ptr(),

    args: args_parse::new(c"b:", 0, 0, None),
    usage: c"[-b buffer-name]".as_ptr(),

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_save_buffer_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_save_buffer_done(
    _c: *mut client,
    path: *mut c_char,
    error: i32,
    closed: i32,
    _buffer: *mut evbuffer,
    data: *mut c_void,
) {
    let mut item = data as *mut cmdq_item;

    if closed == 0 {
        return;
    }

    unsafe {
        if error != 0 {
            cmdq_error(item, c"%s: %s".as_ptr(), path, strerror(error));
        }
        cmdq_continue(item);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_save_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut c = cmdq_get_client(item);
        let mut pb;
        let mut flags = 0;
        let mut bufname = args_get_(args, 'b');
        let mut bufsize: usize = 0;
        let mut path = null_mut();
        let mut evb = null_mut();

        if bufname.is_null() {
            pb = paste_get_top(null_mut());
            if pb.is_null() {
                cmdq_error(item, c"no buffers".as_ptr());
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        } else {
            pb = paste_get_name(bufname);
            if pb.is_null() {
                cmdq_error(item, c"no buffer %s".as_ptr(), bufname);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }
        let mut bufdata = paste_buffer_data(pb, &raw mut bufsize);

        if (cmd_get_entry(self_) == &raw mut cmd_show_buffer_entry) {
            if (!(*c).session.is_null() || ((*c).flags & CLIENT_CONTROL) != 0) {
                evb = evbuffer_new();
                if evb.is_null() {
                    fatalx(c"out of memory".as_ptr());
                }
                evbuffer_add(evb, bufdata as _, bufsize);
                cmdq_print_data(item, 1, evb);
                evbuffer_free(evb);
                return (cmd_retval::CMD_RETURN_NORMAL);
            }
            path = xstrdup_(c"-").as_ptr();
        } else {
            path = format_single_from_target(item, args_string(args, 0));
        }
        if (args_has_(args, 'a')) {
            flags = O_APPEND;
        } else {
            flags = O_TRUNC;
        }
        file_write(
            cmdq_get_client(item),
            path,
            flags,
            bufdata as _,
            bufsize,
            Some(cmd_save_buffer_done),
            item as _,
        );
        free_(path);

        cmd_retval::CMD_RETURN_WAIT
    }
}
