use crate::*;

#[unsafe(no_mangle)]
static mut cmd_paste_buffer_entry: cmd_entry = cmd_entry {
    name: c"paste-buffer".as_ptr(),
    alias: c"pasteb".as_ptr(),

    args: args_parse::new(c"db:prs:t:", 0, 0, None),
    usage: c"[-dpr] [-s separator] [-b buffer-name] [-t target-pane]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_paste_buffer_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_paste_buffer_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut wp = (*target).wp;
        let mut bracket = args_has(args, b'p') != 0;

        if (window_pane_exited(wp) != 0) {
            cmdq_error(item, c"target pane has exited".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        let mut bufname = null();
        if (args_has(args, b'b') != 0) {
            bufname = args_get(args, b'b');
        }

        let mut pb = null_mut();
        if (bufname.is_null()) {
            pb = paste_get_top(null_mut());
        } else {
            pb = paste_get_name(bufname);
            if (pb.is_null()) {
                cmdq_error(item, c"no buffer %s".as_ptr(), bufname);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        if (!pb.is_null() && !(*wp).flags.intersects(window_pane_flags::PANE_INPUTOFF)) {
            let mut sepstr = args_get(args, b's');
            if (sepstr.is_null()) {
                if (args_has(args, b'r') != 0) {
                    sepstr = c"\n".as_ptr();
                } else {
                    sepstr = c"\r".as_ptr();
                }
            }
            let seplen = strlen(sepstr);

            if (bracket && (*(*wp).screen).mode & MODE_BRACKETPASTE != 0) {
                bufferevent_write((*wp).event, c"\x1b[200~".as_ptr().cast(), 6);
            }

            let mut bufsize: usize = 0;
            let mut bufdata = paste_buffer_data(pb, &raw mut bufsize);
            let mut bufend = bufdata.add(bufsize);

            loop {
                let mut line: *mut c_char =
                    libc::memchr(bufdata as _, b'\n' as i32, bufend.addr() - bufdata.addr()).cast();
                if line.is_null() {
                    break;
                }

                bufferevent_write((*wp).event, bufdata.cast(), line.addr() - bufdata.addr());
                bufferevent_write((*wp).event, sepstr.cast(), seplen);

                bufdata = line.add(1);
            }
            if (bufdata != bufend) {
                bufferevent_write((*wp).event, bufdata.cast(), bufend.addr() - bufdata.addr());
            }

            if bracket && (*(*wp).screen).mode & MODE_BRACKETPASTE != 0 {
                bufferevent_write((*wp).event, c"\x1b[201~".as_ptr().cast(), 6);
            }
        }

        if !pb.is_null() && args_has(args, b'd') != 0 {
            paste_free(pb);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
