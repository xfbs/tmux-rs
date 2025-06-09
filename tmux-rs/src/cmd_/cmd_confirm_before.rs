use crate::*;

#[unsafe(no_mangle)]
static mut cmd_confirm_before_entry: cmd_entry = cmd_entry {
    name: c"confirm-before".as_ptr(),
    alias: c"confirm".as_ptr(),

    args: args_parse::new(c"bc:p:t:y", 1, 1, Some(cmd_confirm_before_args_parse)),
    usage: c"[-by] [-c confirm_key] [-p prompt] [-t target-pane] command".as_ptr(),

    flags: cmd_flag::CMD_CLIENT_TFLAG,
    exec: Some(cmd_confirm_before_exec),
    ..unsafe { zeroed() }
};

pub struct cmd_confirm_before_data {
    item: *mut cmdq_item,
    cmdlist: *mut cmd_list,
    confirm_key: c_uchar,
    default_yes: i32,
}

unsafe extern "C" fn cmd_confirm_before_args_parse(
    _: *mut args,
    _: u32,
    _: *mut *mut c_char,
) -> args_parse_type {
    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_confirm_before_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut tc = cmdq_get_target_client(item);
        let mut target = cmdq_get_target(item);
        let mut new_prompt = null_mut();
        let mut wait = !args_has(args, b'b');

        let mut cdata = xcalloc_::<cmd_confirm_before_data>(1).as_ptr();
        (*cdata).cmdlist = args_make_commands_now(self_, item, 0, 1);
        if ((*cdata).cmdlist.is_null()) {
            free_(cdata);
            return (cmd_retval::CMD_RETURN_ERROR);
        }

        if wait != 0 {
            (*cdata).item = item;
        }

        (*cdata).default_yes = args_has(args, b'y');
        let mut confirm_key = args_get(args, b'c');
        if !confirm_key.is_null() {
            if (*confirm_key.add(1) == b'\0' as _ && *confirm_key > 31 && *confirm_key < 127) {
                (*cdata).confirm_key = *confirm_key as _;
            } else {
                cmdq_error(item, c"invalid confirm key".as_ptr());
                free_(cdata);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        } else {
            (*cdata).confirm_key = b'y';
        }

        let mut prompt = args_get(args, b'p');
        if !prompt.is_null() {
            xasprintf(&raw mut new_prompt, c"%s ".as_ptr(), prompt);
        } else {
            let cmd = (*cmd_get_entry(cmd_list_first((*cdata).cmdlist))).name;
            xasprintf(
                &raw mut new_prompt,
                c"Confirm '%s'? (%c/n) ".as_ptr(),
                cmd,
                (*cdata).confirm_key as u32,
            );
        }

        status_prompt_set(
            tc,
            target,
            new_prompt,
            null_mut(),
            Some(cmd_confirm_before_callback),
            Some(cmd_confirm_before_free),
            cdata as _,
            PROMPT_SINGLE,
            prompt_type::PROMPT_TYPE_COMMAND,
        );
        free_(new_prompt);

        if wait == 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_confirm_before_callback(
    c: *mut client,
    data: NonNull<c_void>,
    s: *const c_char,
    _done: i32,
) -> i32 {
    unsafe {
        let mut cdata: NonNull<cmd_confirm_before_data> = data.cast();
        let mut item = (*cdata.as_ptr()).item;
        let mut retcode: i32 = 1;

        'out: {
            if (*c).flags.intersects(client_flag::DEAD) {
                break 'out;
            }

            if s.is_null() {
                break 'out;
            }
            if *s != (*cdata.as_ptr()).confirm_key as _
                && (*s != b'\0' as _ || (*cdata.as_ptr()).default_yes == 0)
            {
                break 'out;
            }
            retcode = 0;

            let mut new_item = null_mut();
            if item.is_null() {
                new_item = cmdq_get_command((*cdata.as_ptr()).cmdlist, null_mut());
                cmdq_append(c, new_item);
            } else {
                new_item = cmdq_get_command((*cdata.as_ptr()).cmdlist, cmdq_get_state(item));
                cmdq_insert_after(item, new_item);
            }
        }

        // out:
        if !item.is_null() {
            if !cmdq_get_client(item).is_null() && (*cmdq_get_client(item)).session.is_null() {
                (*cmdq_get_client(item)).retval = retcode;
            }
            cmdq_continue(item);
        }
        0
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_confirm_before_free(data: NonNull<c_void>) {
    unsafe {
        let mut cdata: NonNull<cmd_confirm_before_data> = data.cast();
        cmd_list_free((*cdata.as_ptr()).cmdlist);
        free_(cdata.as_ptr());
    }
}
