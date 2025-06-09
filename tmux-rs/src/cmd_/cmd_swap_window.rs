use crate::*;

use crate::compat::queue::{tailq_insert_tail, tailq_remove};

#[unsafe(no_mangle)]
static mut cmd_swap_window_entry: cmd_entry = cmd_entry {
    name: c"swap-window".as_ptr(),
    alias: c"swapw".as_ptr(),

    args: args_parse::new(c"ds:t:", 0, 0, None),
    usage: c"[-d] [-s src-window] [-t dst-window]".as_ptr(),

    source: cmd_entry_flag::new(
        b's',
        cmd_find_type::CMD_FIND_WINDOW,
        CMD_FIND_DEFAULT_MARKED,
    ),
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_WINDOW, 0),

    flags: cmd_flag::empty(),
    exec: Some(cmd_swap_window_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_swap_window_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut source = cmdq_get_source(item);
        let mut target = cmdq_get_target(item);
        let mut src = (*source).s;
        let mut dst = (*target).s;
        let mut wl_src = (*source).wl;
        let mut wl_dst = (*target).wl;

        let mut sg_src = session_group_contains(src);
        let mut sg_dst = session_group_contains(dst);

        if (src != dst && !sg_src.is_null() && !sg_dst.is_null() && sg_src == sg_dst) {
            cmdq_error(item, c"can't move window, sessions are grouped".as_ptr());
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if (*wl_dst).window == (*wl_src).window {
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut w_dst = (*wl_dst).window;
        tailq_remove::<_, discr_wentry>(&raw mut (*w_dst).winlinks, wl_dst);
        let mut w_src = (*wl_src).window;
        tailq_remove::<_, discr_wentry>(&raw mut (*w_src).winlinks, wl_src);

        (*wl_dst).window = w_src;
        tailq_insert_tail::<_, discr_wentry>(&raw mut (*w_src).winlinks, wl_dst);
        (*wl_src).window = w_dst;
        tailq_insert_tail::<_, discr_wentry>(&raw mut (*w_dst).winlinks, wl_src);

        if args_has(args, b'd') != 0 {
            session_select(dst, (*wl_dst).idx);
            if src != dst {
                session_select(src, (*wl_src).idx);
            }
        }
        session_group_synchronize_from(src);
        server_redraw_session_group(src);
        if (src != dst) {
            session_group_synchronize_from(dst);
            server_redraw_session_group(dst);
        }
        recalculate_sizes();

        cmd_retval::CMD_RETURN_NORMAL
    }
}
