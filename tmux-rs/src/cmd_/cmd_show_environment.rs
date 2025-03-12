use crate::*;

#[unsafe(no_mangle)]
static mut cmd_show_environment_entry: cmd_entry = cmd_entry {
    name: c"show-environment".as_ptr(),
    alias: c"showenv".as_ptr(),

    args: args_parse::new(c"hgst:", 0, 1, None),
    usage: c"[-hgs] [-t target-session] [name]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, CMD_FIND_CANFAIL),

    flags: CMD_AFTERHOOK,
    exec: Some(cmd_show_environment_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_environment_escape(envent: *mut environ_entry) -> *mut c_char {
    unsafe {
        let mut value = transmute_ptr((*envent).value);
        let mut ret: *mut i8 = xmalloc(strlen(value) * 2 + 1).as_ptr().cast(); /* at most twice the size */
        let mut out = ret;

        let mut c = 0;
        while ({
            c = *value;
            value = value.add(1);
            c != b'\0' as c_char
        }) {
            /* POSIX interprets $ ` " and \ in double quotes. */
            if (c == b'$' as _ || c == b'`' as _ || c == b'"' as _ || c == b'\\' as _) {
                *out = b'\\' as _;
                out = out.add(1);
            }
            *out = c;
            out = out.add(1);
        }
        *out = b'\0' as c_char;

        ret
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_environment_print(self_: *mut cmd, item: *mut cmdq_item, envent: *mut environ_entry) {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut escaped = null_mut();

        if (!args_has_(args, 'h') && ((*envent).flags & ENVIRON_HIDDEN != 0)) {
            return;
        }
        if (args_has_(args, 'h') && (!(*envent).flags & ENVIRON_HIDDEN != 0)) {
            return;
        }

        if (!args_has_(args, 's')) {
            if let Some(value) = (*envent).value {
                cmdq_print(item, c"%s=%s".as_ptr(), (*envent).name, value);
            } else {
                cmdq_print(item, c"-%s".as_ptr(), (*envent).name);
            }
            return;
        }

        if (*envent).value.is_some() {
            escaped = cmd_show_environment_escape(envent);
            cmdq_print(
                item,
                c"%s=\"%s\"; export %s;".as_ptr(),
                (*envent).name,
                escaped,
                (*envent).name,
            );
            free_(escaped);
        } else {
            cmdq_print(item, c"unset %s;".as_ptr(), (*envent).name);
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_show_environment_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut env: *mut environ = null_mut();
        let mut name = args_string(args, 0);

        let mut tflag = args_get_(args, 't');
        if !tflag.is_null() {
            if ((*target).s.is_null()) {
                cmdq_error(item, c"no such session: %s".as_ptr(), tflag);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        if (args_has_(args, 'g')) {
            env = global_environ;
        } else {
            if ((*target).s.is_null()) {
                tflag = args_get_(args, 't');
                if (!tflag.is_null()) {
                    cmdq_error(item, c"no such session: %s".as_ptr(), tflag);
                } else {
                    cmdq_error(item, c"no current session".as_ptr());
                }
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            env = (*(*target).s).environ;
        }

        let mut envent;
        if (!name.is_null()) {
            envent = environ_find(env, name);
            if (envent.is_null()) {
                cmdq_error(item, c"unknown variable: %s".as_ptr(), name);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            cmd_show_environment_print(self_, item, envent);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        envent = environ_first(env);
        while (!envent.is_null()) {
            cmd_show_environment_print(self_, item, envent);
            envent = environ_next(envent);
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}
