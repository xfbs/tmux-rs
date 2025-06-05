use crate::*;

use crate::compat::tree::rb_foreach;

#[unsafe(no_mangle)]
static mut cmd_list_sessions_entry: cmd_entry = cmd_entry {
    name: c"list-sessions".as_ptr(),
    alias: c"ls".as_ptr(),

    args: args_parse::new(c"F:f:", 0, 0, None),
    usage: c"[-F format] [-f filter]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_list_sessions_exec),
    ..unsafe { zeroed() }
};

const LIST_SESSIONS_TEMPLATE: *const i8 = c"#{session_name}: #{session_windows} windows (created #{t:session_created})#{?session_grouped, (group ,}#{session_group}#{?session_grouped,),}#{?session_attached, (attached),}".as_ptr();

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_list_sessions_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);

        let mut template = args_get(args, b'F');
        if template.is_null() {
            template = LIST_SESSIONS_TEMPLATE;
        }
        let mut filter = args_get(args, b'f');

        let mut n = 0;
        for s in rb_foreach(&raw mut sessions) {
            let mut ft = format_create(cmdq_get_client(item), item, FORMAT_NONE as i32, format_flags::empty());
            format_add(ft, c"line".as_ptr(), c"%u".as_ptr(), n);
            format_defaults(ft, null_mut(), Some(s), None, None);

            let mut flag = 0;
            if !filter.is_null() {
                let expanded = format_expand(ft, filter);
                flag = format_true(expanded);
                free_(expanded);
            } else {
                flag = 1;
            }
            if (flag != 0) {
                let line = format_expand(ft, template);
                cmdq_print(item, c"%s".as_ptr(), line);
                free_(line);
            }

            format_free(ft);
            n += 1;
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
