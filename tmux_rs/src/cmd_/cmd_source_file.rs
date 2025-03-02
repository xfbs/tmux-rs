use crate::*;

use compat_rs::VIS_GLOB;
use libc::{EINVAL, ENOENT, ENOMEM, GLOB_NOMATCH, GLOB_NOSPACE, glob, glob_t, globfree, strcmp};

#[unsafe(no_mangle)]
static mut cmd_source_file_entry: cmd_entry = cmd_entry {
    name: c"source-file".as_ptr(),
    alias: c"source".as_ptr(),

    args: args_parse::new(c"t:Fnqv", 1, -1, None),
    usage: c"[-Fnqv] [-t target-pane] path ...".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, CMD_FIND_CANFAIL),

    flags: 0,
    exec: Some(cmd_source_file_exec),
    ..unsafe { zeroed() }
};

#[repr(C)]
pub struct cmd_source_file_data {
    pub item: *mut cmdq_item,
    pub flags: i32,

    pub after: *mut cmdq_item,
    pub retval: cmd_retval,

    pub current: u32,
    pub files: *mut *mut c_char,
    pub nfiles: u32,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_source_file_complete_cb(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    unsafe {
        cfg_print_causes(item);
        cmd_retval::CMD_RETURN_NORMAL
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_source_file_complete(c: *mut client, cdata: *mut cmd_source_file_data) {
    unsafe {
        if cfg_finished != 0 {
            if ((*cdata).retval == cmd_retval::CMD_RETURN_ERROR && !c.is_null() && (*c).session.is_null()) {
                (*c).retval = 1;
            }
            let new_item = cmdq_get_callback!(cmd_source_file_complete_cb, null_mut()).as_ptr();
            cmdq_insert_after((*cdata).after, new_item);
        }

        for i in 0..(*cdata).nfiles {
            free_(*(*cdata).files.add(i as usize));
        }
        free_((*cdata).files);
        free_(cdata);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_source_file_done(
    c: *mut client,
    path: *mut c_char,
    error: i32,
    closed: i32,
    buffer: *mut evbuffer,
    data: *mut c_void,
) {
    unsafe {
        let cdata = data as *mut cmd_source_file_data;
        let mut item = (*cdata).item;
        let mut bdata = EVBUFFER_DATA(buffer);
        let bsize = EVBUFFER_LENGTH(buffer);
        let mut new_item: *mut cmdq_item = null_mut();
        let mut target = cmdq_get_target(item);

        if closed == 0 {
            return;
        }

        if (error != 0) {
            cmdq_error(item, c"%s: %s".as_ptr(), path, strerror(error));
        } else if (bsize != 0) {
            if (load_cfg_from_buffer(
                bdata.cast(),
                bsize,
                path,
                c,
                (*cdata).after,
                target,
                (*cdata).flags,
                &raw mut new_item,
            ) < 0)
            {
                (*cdata).retval = cmd_retval::CMD_RETURN_ERROR;
            } else if !new_item.is_null() {
                (*cdata).after = new_item;
            }
        }

        (*cdata).current += 1;
        let n = (*cdata).current;
        if n < (*cdata).nfiles {
            file_read(
                c,
                *(*cdata).files.add(n as usize),
                Some(cmd_source_file_done),
                cdata.cast(),
            );
        } else {
            cmd_source_file_complete(c, cdata);
            cmdq_continue(item);
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_source_file_add(cdata: *mut cmd_source_file_data, path: *const c_char) {
    unsafe {
        let mut __func__ = c"cmd_source_file_add".as_ptr();
        log_debug(c"%s: %s".as_ptr(), __func__, path);
        (*cdata).files = xreallocarray_((*cdata).files, ((*cdata).nfiles + 1) as usize).as_ptr();
        *(*cdata).files.add((*cdata).nfiles as usize) = xstrdup(path).as_ptr();
        (*cdata).nfiles += 1;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_source_file_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    let __func__ = c"cmd_source_file_exec".as_ptr();

    unsafe {
        let mut args = cmd_get_args(self_);
        let mut c = cmdq_get_client(item);
        let mut retval = cmd_retval::CMD_RETURN_NORMAL;
        let mut pattern: *mut c_char = null_mut();
        let mut cwd = null_mut();
        let mut expanded: *mut c_char = null_mut();
        let mut path: *mut c_char = null_mut();
        let mut error: *mut c_char = null_mut();
        let mut g = MaybeUninit::<glob_t>::uninit();
        let mut result = 0i32;

        let cdata = xcalloc_::<cmd_source_file_data>(1).as_ptr();
        (*cdata).item = item;

        if (args_has_(args, 'q')) {
            (*cdata).flags |= CMD_PARSE_QUIET;
        }
        if (args_has_(args, 'n')) {
            (*cdata).flags |= CMD_PARSE_PARSEONLY;
        }
        if (args_has_(args, 'v')) {
            (*cdata).flags |= CMD_PARSE_VERBOSE;
        }

        utf8_stravis(&raw mut cwd, server_client_get_cwd(c, null_mut()), VIS_GLOB as i32);

        for i in 0..args_count(args) {
            let mut path = args_string(args, i);
            if args_has_(args, 'F') {
                free_(expanded);
                expanded = format_single_from_target(item, path);
                path = expanded;
            }
            if strcmp(path, c"-".as_ptr()) == 0 {
                cmd_source_file_add(cdata, c"-".as_ptr());
                continue;
            }

            if (*path == b'/' as c_char) {
                pattern = xstrdup(path).as_ptr();
            } else {
                xasprintf(&raw mut pattern, c"%s/%s".as_ptr(), cwd, path);
            }
            log_debug(c"%s: %s".as_ptr(), __func__, pattern);

            result = glob(pattern, 0, None, g.as_mut_ptr());
            if result != 0 {
                if result != GLOB_NOMATCH || !(*cdata).flags & CMD_PARSE_QUIET != 0 {
                    if (result == GLOB_NOMATCH) {
                        error = strerror(ENOENT);
                    } else if result == GLOB_NOSPACE {
                        error = strerror(ENOMEM);
                    } else {
                        error = strerror(EINVAL);
                    }
                    cmdq_error(item, c"%s: %s".as_ptr(), path, error);
                    retval = cmd_retval::CMD_RETURN_ERROR;
                }
                globfree(g.as_mut_ptr());
                free_(pattern);
                continue;
            }
            free_(pattern);

            for j in 0..(*g.as_ptr()).gl_pathc {
                cmd_source_file_add(cdata, *(*g.as_ptr()).gl_pathv.add(j));
            }
            globfree(g.as_mut_ptr());
        }
        free_(expanded);

        (*cdata).after = item;
        (*cdata).retval = retval;

        if ((*cdata).nfiles != 0) {
            file_read(c, *(*cdata).files, Some(cmd_source_file_done), cdata as _);
            retval = cmd_retval::CMD_RETURN_WAIT;
        } else {
            cmd_source_file_complete(c, cdata);
        }

        free_(cwd);
        retval
    }
}
