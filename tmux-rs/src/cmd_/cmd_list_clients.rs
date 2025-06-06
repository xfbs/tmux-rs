use crate::*;

use crate::compat::queue::tailq_foreach;

const LIST_CLIENTS_TEMPLATE: &CStr = c"#{client_name}: #{session_name} [#{client_width}x#{client_height} #{client_termname}] #{?#{!=:#{client_uid},#{uid}},[user #{?client_user,#{client_user},#{client_uid},}] ,}#{?client_flags,(,}#{client_flags}#{?client_flags,),}";

#[unsafe(no_mangle)]
static mut cmd_list_clients_entry: cmd_entry = cmd_entry {
    name: c"list-clients".as_ptr(),
    alias: c"lsc".as_ptr(),

    args: args_parse::new(c"F:f:t:", 0, 0, None),
    usage: c"[-F format] [-f filter] [-t target-session]".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_SESSION, 0),

    flags: cmd_flag::CMD_READONLY.union(cmd_flag::CMD_AFTERHOOK),
    exec: Some(cmd_list_clients_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_list_clients_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);

        let mut s = if (args_has(args, b't') != 0) {
            (*target).s
        } else {
            null_mut()
        };

        let mut template = args_get(args, b'F');
        if (template.is_null()) {
            template = LIST_CLIENTS_TEMPLATE.as_ptr();
        }
        let mut filter = args_get(args, b'f');

        let mut idx = 0;
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if ((*c).session.is_null() || (!s.is_null() && s != (*c).session)) {
                continue;
            }

            let mut ft = format_create(
                cmdq_get_client(item),
                item,
                FORMAT_NONE,
                format_flags::empty(),
            );
            format_add(ft, c"line".as_ptr(), c"%u".as_ptr(), idx);
            format_defaults(ft, c, None, None, None);

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

            idx += 1;
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
