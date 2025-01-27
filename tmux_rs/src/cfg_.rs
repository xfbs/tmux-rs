use compat_rs::{queue::tailq_first, tree::rb_min};
use libc::{__errno_location, ENOENT, fclose, fopen, strerror};

use crate::*;

#[unsafe(no_mangle)]
pub static mut cfg_client: *mut client = null_mut();
#[unsafe(no_mangle)]
pub static mut cfg_finished: c_int = 0;

static mut cfg_causes: *mut *mut c_char = null_mut();
static mut cfg_ncauses: c_uint = 0;
static mut cfg_item: *mut cmdq_item = null_mut();

#[unsafe(no_mangle)]
pub static mut cfg_quiet: c_int = 1;
#[unsafe(no_mangle)]
pub static mut cfg_files: *mut *mut c_char = null_mut();
#[unsafe(no_mangle)]
pub static mut cfg_nfiles: c_uint = 0;

unsafe extern "C" fn cfg_client_done(_item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    if unsafe { cfg_finished } == 0 {
        cmd_retval::CMD_RETURN_WAIT
    } else {
        cmd_retval::CMD_RETURN_NORMAL
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cfg_done(item: *mut cmdq_item, _data: *mut c_void) -> cmd_retval {
    unsafe {
        if cfg_finished != 0 {
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        cfg_finished = 1;

        cfg_show_causes(null_mut());

        if !cfg_item.is_null() {
            cmdq_continue(cfg_item);
        }

        status_prompt_load_history();

        cmd_retval::CMD_RETURN_NORMAL
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn start_cfg() {
    let mut c: *mut client;
    let mut i: u32;
    let mut flags: i32 = 0;

    //
    // Configuration files are loaded without a client, so commands are run
    // in the global queue with item->client NULL.
    //
    // However, we must block the initial client (but just the initial
    // client) so that its command runs after the configuration is loaded.
    // Because start_cfg() is called so early, we can be sure the client's
    // command queue is currently empty and our callback will be at the
    // front - we need to get in before MSG_COMMAND.

    unsafe {
        c = tailq_first(&raw mut clients);
        cfg_client = c;
        if !c.is_null() {
            cfg_item = cmdq_get_callback!(cfg_client_done, null_mut());
            cmdq_append(c, cfg_item);
        }

        if cfg_quiet != 0 {
            flags = CMD_PARSE_QUIET;
        }

        i = 0;
        while i < cfg_nfiles {
            load_cfg(*cfg_files.add(i as usize), c, null_mut(), null_mut(), flags, null_mut());
            i += 1;
        }

        cmdq_append(null_mut(), cmdq_get_callback!(cfg_done, null_mut()));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_cfg(
    path: *const c_char,
    c: *mut client,
    item: *mut cmdq_item,
    current: *mut cmd_find_state,
    flags: c_int,
    new_item: *mut *mut cmdq_item,
) -> c_int {
    unsafe {
        if !new_item.is_null() {
            *new_item = null_mut();
        }

        log_debug(c"loading %s".as_ptr(), path);
        let mut f = fopen(path, c"rb".as_ptr());
        if f.is_null() {
            if *__errno_location() == ENOENT && flags & CMD_PARSE_QUIET != 0 {
                return 0;
            }
            cfg_add_cause(c"%s: %s".as_ptr(), path, strerror(*__errno_location()));
            return (-1);
        }

        let mut pi: cmd_parse_input = zeroed();
        pi.flags = flags;
        pi.file = path;
        pi.line = 1;
        pi.item = item;
        pi.c = c;

        let mut pr = cmd_parse_from_file(f, &raw mut pi);
        fclose(f);
        if (*pr).status == cmd_parse_status::CMD_PARSE_ERROR {
            cfg_add_cause(c"%s".as_ptr(), (*pr).error);
            free((*pr).error as _);
            return -1;
        }
        if flags & CMD_PARSE_PARSEONLY != 0 {
            cmd_list_free((*pr).cmdlist);
            return 0;
        }

        let mut state = if !item.is_null() {
            cmdq_copy_state(cmdq_get_state(item), current)
        } else {
            cmdq_new_state(null_mut(), null_mut(), 0)
        };
        cmdq_add_format(state, c"current_file".as_ptr(), c"%s".as_ptr(), pi.file);

        let mut new_item0 = cmdq_get_command((*pr).cmdlist, state);
        if !item.is_null() {
            new_item0 = cmdq_insert_after(item, new_item0);
        } else {
            new_item0 = cmdq_append(null_mut(), new_item0);
        }
        cmd_list_free((*pr).cmdlist);
        cmdq_free_state(state);

        if !new_item.is_null() {
            *new_item = new_item0;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_cfg_from_buffer(
    buf: *const c_void,
    len: usize,
    path: *const c_char,
    c: *mut client,
    item: *mut cmdq_item,
    current: *mut cmd_find_state,
    flags: c_int,
    new_item: *mut *mut cmdq_item,
) -> c_int {
    unsafe {
        if !new_item.is_null() {
            *new_item = null_mut();
        }

        log_debug(c"loading %s".as_ptr(), path);

        let mut pi: cmd_parse_input = zeroed();
        pi.flags = flags;
        pi.file = path;
        pi.line = 1;
        pi.item = item;
        pi.c = c;

        let mut pr = cmd_parse_from_buffer(buf, len, &raw mut pi);
        if (*pr).status == cmd_parse_status::CMD_PARSE_ERROR {
            cfg_add_cause(c"%s".as_ptr(), (*pr).error);
            free((*pr).error as _);
            return -1;
        }
        if flags & CMD_PARSE_PARSEONLY != 0 {
            cmd_list_free((*pr).cmdlist);
            return 0;
        }

        let mut state = if !item.is_null() {
            cmdq_copy_state(cmdq_get_state(item), current)
        } else {
            cmdq_new_state(null_mut(), null_mut(), 0)
        };
        cmdq_add_format(state, c"current_file".as_ptr(), c"%s".as_ptr(), pi.file);

        let mut new_item0 = cmdq_get_command((*pr).cmdlist, state);
        if !item.is_null() {
            new_item0 = cmdq_insert_after(item, new_item0);
        } else {
            new_item0 = cmdq_append(null_mut(), new_item0);
        }
        cmd_list_free((*pr).cmdlist);
        cmdq_free_state(state);

        if !new_item.is_null() {
            *new_item = new_item0;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cfg_add_cause(fmt: *const c_char, mut args: ...) {
    unsafe {
        let mut msg: *mut c_char = null_mut();

        xvasprintf(&raw mut msg, fmt, args.as_va_list());

        cfg_ncauses += 1;
        cfg_causes = xreallocarray(cfg_causes as _, cfg_ncauses as usize, size_of::<*mut c_char>())
            .cast()
            .as_ptr();
        *cfg_causes.add(cfg_ncauses as usize - 1) = msg;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cfg_print_causes(item: *mut cmdq_item) {
    unsafe {
        for i in 0..cfg_ncauses {
            cmdq_print(item, c"%s".as_ptr(), *cfg_causes.add(i as usize));
            free(*cfg_causes.add(i as usize) as _);
        }

        free(cfg_causes as _);
        cfg_causes = null_mut();
        cfg_ncauses = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cfg_show_causes(mut s: *mut session) {
    unsafe {
        'out: loop {
            let mut c = tailq_first(&raw mut clients);

            if cfg_ncauses == 0 {
                return;
            }

            if !c.is_null() && (*c).flags & CLIENT_CONTROL != 0 {
                for i in 0..cfg_ncauses {
                    control_write(c, c"%%config-error %s".as_ptr(), *cfg_causes.add(i as usize));
                    free(*cfg_causes.add(i as usize) as _);
                }
                // goto out;
                break 'out;
            }

            if s.is_null() {
                if !c.is_null() && !(*c).session.is_null() {
                    s = (*c).session;
                } else {
                    s = rb_min(&raw mut sessions);
                }
            }
            if s.is_null() || (*s).attached == 0 {
                return;
            }
            let wp = (*(*(*s).curw).window).active;

            let wme: *mut window_mode_entry = tailq_first(&raw mut (*wp).modes);
            if wme.is_null() || (*wme).mode != &raw mut window_view_mode {
                window_pane_set_mode(wp, null_mut(), &raw mut window_view_mode, null_mut(), null_mut());
            }
            for i in 0..cfg_ncauses {
                window_copy_add(wp, 0, c"%s".as_ptr(), *cfg_causes.add(i as usize));
                free(*cfg_causes.add(i as usize) as _);
            }
            break;
        }
        // out:
        free(cfg_causes as _);
        cfg_causes = null_mut();
        cfg_ncauses = 0;
    }
}
