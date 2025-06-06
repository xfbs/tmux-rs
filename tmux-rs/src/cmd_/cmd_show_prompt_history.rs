use crate::*;

#[unsafe(no_mangle)]
static mut cmd_show_prompt_history_entry: cmd_entry = cmd_entry {
    name: c"show-prompt-history".as_ptr(),
    alias: c"showphist".as_ptr(),

    args: args_parse::new(c"T:", 0, 0, None),
    usage: c"[-T type]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_prompt_history_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_clear_prompt_history_entry: cmd_entry = cmd_entry {
    name: c"clear-prompt-history".as_ptr(),
    alias: c"clearphist".as_ptr(),

    args: args_parse::new(c"T:", 0, 0, None),
    usage: c"[-T type]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_show_prompt_history_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_prompt_history_exec(
    self_: *mut cmd,
    item: *mut cmdq_item,
) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut typestr = args_get(args, b'T');
        let mut type_: prompt_type;

        if (cmd_get_entry(self_) == &raw mut cmd_clear_prompt_history_entry) {
            if (typestr.is_null()) {
                for tidx in 0..PROMPT_NTYPES {
                    free_(status_prompt_hlist[tidx as usize]);
                    status_prompt_hlist[tidx as usize] = null_mut();
                    status_prompt_hsize[tidx as usize] = 0;
                }
            } else {
                type_ = status_prompt_type(typestr);
                if (type_ == prompt_type::PROMPT_TYPE_INVALID) {
                    cmdq_error(item, c"invalid type: %s".as_ptr(), typestr);
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
                free_(status_prompt_hlist[type_ as usize]);
                status_prompt_hlist[type_ as usize] = null_mut();
                status_prompt_hsize[type_ as usize] = 0;
            }

            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        if (typestr.is_null()) {
            for tidx in 0..PROMPT_NTYPES {
                cmdq_print(
                    item,
                    c"History for %s:\n".as_ptr(),
                    status_prompt_type_string(tidx),
                );
                for hidx in 0u32..status_prompt_hsize[tidx as usize] {
                    cmdq_print(
                        item,
                        c"%d: %s".as_ptr(),
                        hidx + 1,
                        *status_prompt_hlist[tidx as usize].add(hidx as usize),
                    );
                }
                cmdq_print(item, c"%s".as_ptr(), c"".as_ptr());
            }
        } else {
            type_ = status_prompt_type(typestr);
            if (type_ == prompt_type::PROMPT_TYPE_INVALID) {
                cmdq_error(item, c"invalid type: %s".as_ptr(), typestr);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            cmdq_print(
                item,
                c"History for %s:\n".as_ptr(),
                status_prompt_type_string(type_ as u32),
            );
            for hidx in 0u32..status_prompt_hsize[type_ as usize] {
                cmdq_print(
                    item,
                    c"%d: %s".as_ptr(),
                    hidx + 1,
                    *status_prompt_hlist[type_ as usize].add(hidx as usize),
                );
            }
            cmdq_print(item, c"%s".as_ptr(), c"".as_ptr());
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
