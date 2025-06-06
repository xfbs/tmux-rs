use crate::*;

#[unsafe(no_mangle)]
static mut cmd_list_buffers_entry: cmd_entry = cmd_entry {
    name: c"list-buffers".as_ptr(),
    alias: c"lsb".as_ptr(),

    args: args_parse::new(c"F:f:", 0, 0, None),
    usage: c"[-F format] [-f filter]".as_ptr(),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_list_buffers_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_list_buffers_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut flag = 0;

        let mut template: *const c_char = args_get(args, b'F');
        if template.is_null() {
            template = c"#{buffer_name}: #{buffer_size} bytes: \"#{buffer_sample}\"".as_ptr();
        }
        let mut filter = args_get(args, b'f');

        let mut pb = null_mut();
        while ({
            pb = paste_walk(pb);
            !pb.is_null()
        }) {
            let ft = format_create(
                cmdq_get_client(item),
                item,
                FORMAT_NONE,
                format_flags::empty(),
            );
            format_defaults_paste_buffer(ft, pb);

            if !filter.is_null() {
                let expanded = format_expand(ft, filter);
                flag = format_true(expanded);
                free_(expanded);
            } else {
                flag = 1;
            }
            if flag != 0 {
                let line = format_expand(ft, template);
                cmdq_print(item, c"%s".as_ptr(), line);
                free_(line);
            }

            format_free(ft);
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
