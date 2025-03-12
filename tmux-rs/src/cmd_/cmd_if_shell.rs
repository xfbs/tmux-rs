use libc::{WEXITSTATUS, WIFEXITED, toupper};

use crate::*;

#[unsafe(no_mangle)]
static mut cmd_if_shell_entry: cmd_entry = cmd_entry {
    name: c"if-shell".as_ptr(),
    alias: c"if".as_ptr(),

    args: args_parse::new(c"bFt:", 2, 3, Some(cmd_if_shell_args_parse)),
    usage: c"[-bF] [-t target-pane] shell-command command [command]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: 0,
    exec: Some(cmd_if_shell_exec),
    ..unsafe { zeroed() }
};

#[repr(C)]
pub struct cmd_if_shell_data {
    pub cmd_if: *mut args_command_state,
    pub cmd_else: *mut args_command_state,

    pub client: *mut client,
    pub item: *mut cmdq_item,
}

unsafe extern "C" fn cmd_if_shell_args_parse(_: *mut args, idx: u32, _: *mut *mut c_char) -> args_parse_type {
    if idx == 1 || idx == 2 {
        args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING
    } else {
        args_parse_type::ARGS_PARSE_STRING
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_if_shell_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut tc = cmdq_get_target_client(item);
        let mut s = (*target).s;
        let mut count = args_count(args);
        let mut wait = !args_has(args, b'b');

        let shellcmd = format_single_from_target(item, args_string(args, 0));
        if (args_has_(args, 'F')) {
            let cmdlist = if (*shellcmd != b'0' as _ && *shellcmd != b'\0' as _) {
                args_make_commands_now(self_, item, 1, 0)
            } else if (count == 3) {
                args_make_commands_now(self_, item, 2, 0)
            } else {
                free_(shellcmd);
                return (cmd_retval::CMD_RETURN_NORMAL);
            };
            free_(shellcmd);
            if (cmdlist.is_null()) {
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            let new_item = cmdq_get_command(cmdlist, cmdq_get_state(item));
            cmdq_insert_after(item, new_item);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        let cdata = xcalloc_::<cmd_if_shell_data>(1).as_ptr();

        (*cdata).cmd_if = args_make_commands_prepare(self_, item, 1, null_mut(), wait, 0);
        if (count == 3) {
            (*cdata).cmd_else = args_make_commands_prepare(self_, item, 2, null_mut(), wait, 0);
        }

        if (wait != 0) {
            (*cdata).client = cmdq_get_client(item);
            (*cdata).item = item;
        } else {
            (*cdata).client = tc;
        }
        if (!(*cdata).client.is_null()) {
            (*(*cdata).client).references += 1;
        }

        if (job_run(
            shellcmd,
            0,
            null_mut(),
            null_mut(),
            s,
            server_client_get_cwd(cmdq_get_client(item), s),
            None,
            Some(cmd_if_shell_callback),
            Some(cmd_if_shell_free),
            cdata as _,
            0,
            -1,
            -1,
        )
        .is_null())
        {
            cmdq_error(item, c"failed to run command: %s".as_ptr(), shellcmd);
            free_(shellcmd);
            free_(cdata);
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        free_(shellcmd);

        if (wait == 0) {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        return (cmd_retval::CMD_RETURN_WAIT);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_if_shell_callback(job: *mut job) {
    unsafe {
        let mut cdata = job_get_data(job) as *mut cmd_if_shell_data;
        let mut c = (*cdata).client;
        let mut item = (*cdata).item;
        let mut error: *mut c_char = null_mut();

        let mut state: *mut args_command_state = null_mut();
        let status = job_get_status(job);

        'out: {
            if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
                state = (*cdata).cmd_else;
            } else {
                state = (*cdata).cmd_if;
            }
            if (state.is_null()) {
                break 'out;
            }

            let cmdlist = args_make_commands(state, 0, null_mut(), &raw mut error);
            if (cmdlist.is_null()) {
                if ((*cdata).item.is_null()) {
                    *error = toupper(*error as i32) as i8;
                    status_message_set(c, -1, 1, 0, c"%s".as_ptr(), error);
                } else {
                    cmdq_error((*cdata).item, c"%s".as_ptr(), error);
                }
                free_(error);
            } else if (item.is_null()) {
                let new_item = cmdq_get_command(cmdlist, null_mut());
                cmdq_append(c, new_item);
            } else {
                let new_item = cmdq_get_command(cmdlist, cmdq_get_state(item));
                cmdq_insert_after(item, new_item);
            }
        }

        // out:
        if (!(*cdata).item.is_null()) {
            cmdq_continue((*cdata).item);
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_if_shell_free(data: *mut c_void) {
    unsafe {
        let mut cdata = data as *mut cmd_if_shell_data;

        if !(*cdata).client.is_null() {
            server_client_unref((*cdata).client);
        }

        if !(*cdata).cmd_else.is_null() {
            args_make_commands_free((*cdata).cmd_else);
        }
        args_make_commands_free((*cdata).cmd_if);

        free_(cdata);
    }
}
