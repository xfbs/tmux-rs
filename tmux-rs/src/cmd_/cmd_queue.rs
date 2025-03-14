use crate::{xmalloc::xcalloc1, *};

unsafe extern "C" {
    // pub unsafe fn cmdq_new_state(_: *mut cmd_find_state, _: *mut key_event, _: c_int) -> *mut cmdq_state;
    // pub unsafe fn cmdq_link_state(_: *mut cmdq_state) -> *mut cmdq_state;
    // pub unsafe fn cmdq_copy_state(_: *mut cmdq_state, _: *mut cmd_find_state) -> *mut cmdq_state;
    // pub unsafe fn cmdq_free_state(_: *mut cmdq_state);
    // pub unsafe fn cmdq_add_format(_: *mut cmdq_state, _: *const c_char, _: *const c_char, ...);
    // pub unsafe fn cmdq_add_formats(_: *mut cmdq_state, _: *mut format_tree);
    // pub unsafe fn cmdq_merge_formats(_: *mut cmdq_item, _: *mut format_tree);
    // pub unsafe fn cmdq_new() -> *mut cmdq_list;
    // pub unsafe fn cmdq_free(_: *mut cmdq_list);
    // pub unsafe fn cmdq_get_name(_: *mut cmdq_item) -> *const c_char;
    // pub unsafe fn cmdq_get_client(_: *mut cmdq_item) -> *mut client;
    // pub unsafe fn cmdq_get_target_client(_: *mut cmdq_item) -> *mut client;
    // pub unsafe fn cmdq_get_state(_: *mut cmdq_item) -> *mut cmdq_state;
    // pub unsafe fn cmdq_get_target(_: *mut cmdq_item) -> *mut cmd_find_state;
    // pub unsafe fn cmdq_get_source(_: *mut cmdq_item) -> *mut cmd_find_state;
    // pub unsafe fn cmdq_get_event(_: *mut cmdq_item) -> *mut key_event;
    // pub unsafe fn cmdq_get_current(_: *mut cmdq_item) -> *mut cmd_find_state;
    // pub unsafe fn cmdq_get_flags(_: *mut cmdq_item) -> c_int;
    // pub unsafe fn cmdq_get_command(_: *mut cmd_list, _: *mut cmdq_state) -> *mut cmdq_item;
    // pub unsafe fn cmdq_get_callback1(_: *const c_char, _: cmdq_cb, _: *mut c_void) -> NonNull<cmdq_item>;
    // pub unsafe fn cmdq_get_error(_: *const c_char) -> NonNull<cmdq_item>;
    pub unsafe fn cmdq_insert_after(_: *mut cmdq_item, _: *mut cmdq_item) -> *mut cmdq_item;
    pub unsafe fn cmdq_append(_: *mut client, _: *mut cmdq_item) -> *mut cmdq_item;
    // pub unsafe fn cmdq_insert_hook(_: *mut session, _: *mut cmdq_item, _: *mut cmd_find_state, _: *const c_char, ...);
    // pub unsafe fn cmdq_continue(_: *mut cmdq_item);
    pub unsafe fn cmdq_next(_: *mut client) -> c_uint;
    // pub unsafe fn cmdq_running(_: *mut client) -> *mut cmdq_item;
    // pub unsafe fn cmdq_guard(_: *mut cmdq_item, _: *const c_char, _: c_int);
    // pub unsafe fn cmdq_print(_: *mut cmdq_item, _: *const c_char, ...);
    // pub unsafe fn cmdq_print_data(_: *mut cmdq_item, _: c_int, _: *mut evbuffer);
    // pub unsafe fn cmdq_error(_: *mut cmdq_item, _: *const c_char, ...);
}

macro_rules! cstringify {
    ($e:expr) => {
        unsafe { ::core::ffi::CStr::from_bytes_with_nul_unchecked(concat!(stringify!($e), "\0").as_bytes()).as_ptr() }
    };
}
use compat_rs::{
    queue::{tailq_empty, tailq_first, tailq_init, tailq_insert_tail, tailq_last, tailq_next, tailq_remove},
    tailq_insert_after,
};
pub(crate) use cstringify;

// #define cmdq_get_callback(cb, data) cmdq_get_callback1(#cb, cb, data)
#[macro_export]
macro_rules! cmdq_get_callback {
    ($cb:ident, $data:expr) => {
        $crate::cmd_::cmd_queue::cmdq_get_callback1(
            const { $crate::cmd_::cmd_queue::cstringify!($cb) },
            Some($cb),
            $data,
        )
    };
}
pub use cmdq_get_callback;
use libc::{getpwuid, getuid, toupper};

/* Command queue flags. */
pub const CMDQ_FIRED: i32 = 0x1;
pub const CMDQ_WAITING: i32 = 0x2;

/* Command queue item type. */
#[repr(i32)]
#[derive(Copy, Clone)]
pub enum cmdq_type {
    CMDQ_COMMAND,
    CMDQ_CALLBACK,
}

// #[derive(compat_rs::TailQEntry)]
compat_rs::impl_tailq_entry!(cmdq_item, entry, tailq_entry<cmdq_item>);
#[repr(C)]
pub struct cmdq_item {
    pub name: *mut c_char,
    pub queue: *mut cmdq_list,
    pub next: *mut cmdq_item,

    pub client: *mut client,
    pub target_client: *mut client,

    pub type_: cmdq_type,
    pub group: u32,

    pub number: u32,
    pub time: time_t,

    pub flags: i32,

    pub state: *mut cmdq_state,
    pub source: cmd_find_state,
    pub target: cmd_find_state,

    pub cmdlist: *mut cmd_list,
    pub cmd: *mut cmd,

    pub cb: cmdq_cb,
    pub data: *mut c_void,

    // #[entry]
    pub entry: tailq_entry<cmdq_item>,
}

pub type cmdq_item_list = tailq_head<cmdq_item>;

#[repr(C)]
pub struct cmdq_state {
    pub references: i32,
    pub flags: i32,

    pub formats: *mut format_tree,

    pub event: key_event,
    pub current: cmd_find_state,
}

#[repr(C)]
pub struct cmdq_list {
    pub item: *mut cmdq_item,
    pub list: cmdq_item_list,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_name(c: *const client) -> *const c_char {
    static mut buf: [c_char; 256] = [0; 256];
    let s = &raw mut buf as *mut i8;

    if c.is_null() {
        return c"<global>".as_ptr();
    }

    unsafe {
        if !(*c).name.is_null() {
            xsnprintf(s, 256, c"<%s>".as_ptr(), (*c).name);
        } else {
            xsnprintf(s, 256, c"<%p>".as_ptr(), c);
        }
    }

    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get(c: *mut client) -> *mut cmdq_list {
    static mut global_queue: *mut cmdq_list = null_mut();

    unsafe {
        if (c.is_null()) {
            if global_queue.is_null() {
                global_queue = cmdq_new().as_ptr();
            }
            return global_queue;
        }

        (*c).queue
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_new() -> NonNull<cmdq_list> {
    unsafe {
        let queue = NonNull::from(xcalloc1::<cmdq_list>());
        tailq_init(&raw mut (*queue.as_ptr()).list);
        queue
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_free(queue: *mut cmdq_list) {
    unsafe {
        if !tailq_empty(&raw mut (*queue).list) {
            fatalx(c"queue not empty".as_ptr());
        }
        free_(queue);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn cmdq_get_name(item: *mut cmdq_item) -> *mut c_char { unsafe { (*item).name } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_client(item: *mut cmdq_item) -> *mut client { unsafe { (*item).client } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_target_client(item: *mut cmdq_item) -> *mut client {
    unsafe { (*item).target_client }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_state(item: *mut cmdq_item) -> *mut cmdq_state { unsafe { (*item).state } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_target(item: *mut cmdq_item) -> *mut cmd_find_state {
    unsafe { &raw mut (*item).target }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_source(item: *mut cmdq_item) -> *mut cmd_find_state {
    unsafe { &raw mut (*item).source }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_event(item: *mut cmdq_item) -> *mut key_event {
    unsafe { &raw mut (*(*item).state).event }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_current(item: *mut cmdq_item) -> *mut cmd_find_state {
    unsafe { &raw mut (*(*item).state).current }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_flags(item: *mut cmdq_item) -> i32 { unsafe { (*(*item).state).flags } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_new_state(
    current: *mut cmd_find_state,
    event: *mut key_event,
    flags: i32,
) -> *mut cmdq_state {
    unsafe {
        let state: *mut cmdq_state = xcalloc1::<cmdq_state>();
        (*state).references = 1;
        (*state).flags = flags;

        if (!event.is_null()) {
            memcpy__(&raw mut (*state).event, event);
        } else {
            (*state).event.key = KEYC_NONE;
        }
        if !current.is_null() && cmd_find_valid_state(current) != 0 {
            cmd_find_copy_state(&raw mut (*state).current, current);
        } else {
            cmd_find_clear_state(&raw mut (*state).current, 0);
        }

        state
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_link_state(state: *mut cmdq_state) -> *mut cmdq_state {
    unsafe {
        (*state).references += 1;
    }
    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_copy_state(state: *mut cmdq_state, current: *mut cmd_find_state) -> *mut cmdq_state {
    unsafe {
        if !current.is_null() {
            return cmdq_new_state(current, &raw mut (*state).event, (*state).flags);
        }

        cmdq_new_state(&raw mut (*state).current, &raw mut (*state).event, (*state).flags)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_free_state(state: *mut cmdq_state) {
    unsafe {
        (*state).references -= 1;
        if (*state).references != 0 {
            return;
        }

        if !(*state).formats.is_null() {
            format_free((*state).formats);
        }
        free_(state);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_add_format(
    state: *mut cmdq_state,
    key: *const c_char,
    fmt: *const c_char,
    mut args: ...
) {
    let mut value = null_mut();
    unsafe {
        xvasprintf(&raw mut value, fmt, args.as_va_list());

        if ((*state).formats.is_null()) {
            (*state).formats = format_create(null_mut(), null_mut(), FORMAT_NONE, 0);
        }
        format_add((*state).formats, key, c"%s".as_ptr(), value);

        free_(value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_add_formats(state: *mut cmdq_state, ft: *mut format_tree) {
    unsafe {
        if ((*state).formats.is_null()) {
            (*state).formats = format_create(null_mut(), null_mut(), FORMAT_NONE, 0);
        }
        format_merge((*state).formats, ft);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_merge_formats(item: *mut cmdq_item, ft: *mut format_tree) {
    unsafe {
        if !(*item).cmd.is_null() {
            let entry = cmd_get_entry((*item).cmd);
            format_add(ft, c"command".as_ptr(), c"%s".as_ptr(), (*entry).name);
        }

        if !(*(*item).state).formats.is_null() {
            format_merge(ft, (*(*item).state).formats);
        }
    }
}

// TODO: this function is broken, likely due to tailq_insert_tail or tailq_last being incorrect
/*
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_append(c: *mut client, mut item: *mut cmdq_item) -> *mut cmdq_item {
    let func = "cmdq_append".as_ptr();

    unsafe {
        let mut queue = cmdq_get(c);
        let mut next = null_mut();

        loop {
            next = (*item).next;
            (*item).next = null_mut();

            if !c.is_null() {
                (*c).references += 1;
            }
            (*item).client = c;

            (*item).queue = queue;
            tailq_insert_tail::<_, ()>(&raw mut (*queue).list, item);
            log_debug(c"%s %s: %s".as_ptr(), func, cmdq_name(c), (*item).name);

            item = next;
            if item.is_null() {
                break;
            }
        }
        tailq_last(&raw mut (*queue).list)
    }
}
*/

// TODO crashes with this one
/*
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_insert_after(mut after: *mut cmdq_item, mut item: *mut cmdq_item) -> *mut cmdq_item {
    unsafe {
        let c = (*after).client;
        let queue = (*after).queue;

        loop {
            let mut next = (*item).next;
            (*item).next = (*after).next;
            (*after).next = item;

            if (!c.is_null()) {
                (*c).references += 1;
            }
            (*item).client = c;

            (*item).queue = queue;
            tailq_insert_after!(&raw mut (*queue).list, after, item, entry);
            log_debug(
                c"%s %s: %s after %s".as_ptr(),
                c"cmdq_insert_after".as_ptr(),
                cmdq_name(c),
                (*item).name,
                (*after).name,
            );

            after = item;
            item = next;
            if item.is_null() {
                break;
            }
        }
        after
    }
}
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_insert_hook(
    s: *mut session,
    mut item: *mut cmdq_item,
    current: *mut cmd_find_state,
    fmt: *const c_char,
    mut ap: ...
) {
    unsafe {
        let mut state = (*item).state;
        let mut cmd = (*item).cmd;
        let mut args = cmd_get_args(cmd);
        let mut ae: *mut args_entry = null_mut();
        let mut flag: c_uchar = 0;
        let mut name: *mut c_char = null_mut();
        const sizeof_tmp: usize = 32;
        let mut buf: [c_char; 32] = zeroed();
        let tmp = &raw mut buf as *mut c_char;

        if (*(*item).state).flags & CMDQ_STATE_NOHOOKS != 0 {
            return;
        }
        let oo = if s.is_null() { global_s_options } else { (*s).options };

        xvasprintf(&raw mut name, fmt, ap.as_va_list());

        let o = options_get(oo, name);
        if o.is_null() {
            free_(name);
            return;
        }
        log_debug(c"running hook %s (parent %p)".as_ptr(), name, item);

        /*
         * The hooks get a new state because they should not update the current
         * target or formats for any subsequent commands.
         */
        let new_state = cmdq_new_state(current, &raw mut (*state).event, CMDQ_STATE_NOHOOKS);
        cmdq_add_format(new_state, c"hook".as_ptr(), c"%s".as_ptr(), name);

        let arguments = args_print(args);
        cmdq_add_format(new_state, c"hook_arguments".as_ptr(), c"%s".as_ptr(), arguments);
        free_(arguments);

        for i in 0..args_count(args) {
            xsnprintf(tmp, sizeof_tmp, c"hook_argument_%d".as_ptr(), i);
            cmdq_add_format(new_state, tmp, c"%s".as_ptr(), args_string(args, i));
        }
        flag = args_first(args, &raw mut ae);
        while flag != 0 {
            let value = args_get(args, flag);
            if (value.is_null()) {
                xsnprintf(tmp, sizeof_tmp, c"hook_flag_%c".as_ptr(), flag as u32);
                cmdq_add_format(new_state, tmp, c"1".as_ptr());
            } else {
                xsnprintf(tmp, sizeof_tmp, c"hook_flag_%c".as_ptr(), flag as u32);
                cmdq_add_format(new_state, tmp, c"%s".as_ptr(), value);
            }

            let mut i = 0;
            let mut av = args_first_value(args, flag);
            while !av.is_null() {
                xsnprintf(tmp, sizeof_tmp, c"hook_flag_%c_%d".as_ptr(), flag as u32, i);
                cmdq_add_format(new_state, tmp, c"%s".as_ptr(), (*av).union_.string);
                i += 1;
                av = args_next_value(av);
            }

            flag = args_next(&raw mut ae);
        }

        let mut a = options_array_first(o);
        while !a.is_null() {
            let cmdlist = (*options_array_item_value(a)).cmdlist;
            if (!cmdlist.is_null()) {
                let new_item = cmdq_get_command(cmdlist, new_state);
                if (!item.is_null()) {
                    item = cmdq_insert_after(item, new_item);
                } else {
                    item = cmdq_append(null_mut(), new_item);
                }
            }
            a = options_array_next(a);
        }

        cmdq_free_state(new_state);
        free_(name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_continue(item: *mut cmdq_item) {
    unsafe {
        (*item).flags &= !CMDQ_WAITING;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_remove(item: *mut cmdq_item) {
    unsafe {
        if !(*item).client.is_null() {
            server_client_unref((*item).client);
        }
        if !(*item).cmdlist.is_null() {
            cmd_list_free((*item).cmdlist);
        }
        cmdq_free_state((*item).state);

        tailq_remove(&raw mut (*(*item).queue).list, item);

        free_((*item).name);
        free_(item);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_remove_group(item: *mut cmdq_item) {
    unsafe {
        if ((*item).group == 0) {
            return;
        }
        let mut this = tailq_next(item);
        while !this.is_null() {
            let next = tailq_next(this);
            if ((*this).group == (*item).group) {
                cmdq_remove(this);
            }
            this = next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_empty_command(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_command(cmdlist: *mut cmd_list, mut state: *mut cmdq_state) -> *mut cmdq_item {
    unsafe {
        let mut first: *mut cmdq_item = null_mut();
        let mut last: *mut cmdq_item = null_mut();
        let mut created = false;

        let mut cmd = cmd_list_first(cmdlist);
        if cmd.is_null() {
            return cmdq_get_callback!(cmdq_empty_command, null_mut()).as_ptr();
        }

        if (state.is_null()) {
            state = cmdq_new_state(null_mut(), null_mut(), 0);
            created = true;
        }

        while !cmd.is_null() {
            let entry = cmd_get_entry(cmd);

            let mut item = xcalloc1::<cmdq_item>() as *mut cmdq_item;
            xasprintf(&raw mut (*item).name, c"[%s/%p]".as_ptr(), (*entry).name, item);
            (*item).type_ = cmdq_type::CMDQ_COMMAND;

            (*item).group = cmd_get_group(cmd);
            (*item).state = cmdq_link_state(state);

            (*item).cmdlist = cmdlist;
            (*item).cmd = cmd;

            (*cmdlist).references += 1;
            log_debug(
                c"%s: %s group %u".as_ptr(),
                "cmdq_get_command".as_ptr(),
                (*item).name,
                (*item).group,
            );

            if first.is_null() {
                first = item;
            }
            if !last.is_null() {
                (*last).next = item;
            }
            last = item;

            cmd = cmd_list_next(cmd);
        }

        if created {
            cmdq_free_state(state);
        }
        first
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_find_flag(
    item: *mut cmdq_item,
    fs: *mut cmd_find_state,
    flag: *mut cmd_entry_flag,
) -> cmd_retval {
    unsafe {
        if ((*flag).flag == 0) {
            cmd_find_from_client(fs, (*item).target_client, 0);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let value = args_get(cmd_get_args((*item).cmd), (*flag).flag as u8);
        if (cmd_find_target(fs, item, value, (*flag).type_, (*flag).flags) != 0) {
            cmd_find_clear_state(fs, 0);
            return cmd_retval::CMD_RETURN_ERROR;
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_add_message(item: *mut cmdq_item) {
    unsafe {
        let mut c = (*item).client;
        let mut state = (*item).state;
        let mut user = null_mut();

        let tmp = cmd_print((*item).cmd);
        if !c.is_null() {
            let uid = proc_get_peer_uid((*c).peer);
            if uid != -1i32 as uid_t && uid != getuid() {
                let pw = getpwuid(uid);
                if !pw.is_null() {
                    xasprintf(&raw mut user, c"[%s]".as_ptr(), (*pw).pw_name);
                } else {
                    user = xstrdup(c"[unknown]".as_ptr()).as_ptr();
                }
            } else {
                user = xstrdup(c"".as_ptr()).as_ptr();
            }
            if !(*c).session.is_null() && (*state).event.key != KEYC_NONE {
                let key = key_string_lookup_key((*state).event.key, 0);
                server_add_message(c"%s%s key %s: %s".as_ptr(), (*c).name, user, key, tmp);
            } else {
                server_add_message(c"%s%s command: %s".as_ptr(), (*c).name, user, tmp);
            }
            free_(user);
        } else {
            server_add_message(c"command: %s".as_ptr(), tmp);
        }
        free_(tmp);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_fire_command(item: *mut cmdq_item) -> cmd_retval {
    let __func__ = c"cmdq_fire_command".as_ptr();

    unsafe {
        let mut name = cmdq_name((*item).client);
        let mut state = (*item).state;
        let mut cmd = (*item).cmd;
        let mut args = cmd_get_args(cmd);
        let mut entry = cmd_get_entry(cmd);
        let mut tc = null_mut();
        let mut saved = (*item).client;
        let mut retval;
        let mut fs: cmd_find_state = zeroed();
        let mut fsp: *mut cmd_find_state = null_mut();
        let mut flags = 0;
        let mut quiet = 0;

        'out: {
            if cfg_finished != 0 {
                cmdq_add_message(item);
            }
            if log_get_level() > 1 {
                let tmp = cmd_print(cmd);
                log_debug(c"%s %s: (%u) %s".as_ptr(), __func__, name, (*item).group, tmp);
                free_(tmp);
            }

            flags = !!((*state).flags & CMDQ_STATE_CONTROL);
            cmdq_guard(item, c"begin".as_ptr(), flags);

            if ((*item).client.is_null()) {
                (*item).client = cmd_find_client(item, null_mut(), 1);
            }

            if (*entry).flags & CMD_CLIENT_CANFAIL != 0 {
                quiet = 1;
            }
            if (*entry).flags & CMD_CLIENT_CFLAG != 0 {
                tc = cmd_find_client(item, args_get_(args, 'c'), quiet);
                if (tc.is_null() && quiet == 0) {
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
            } else if ((*entry).flags & CMD_CLIENT_TFLAG != 0) {
                tc = cmd_find_client(item, args_get_(args, 't'), quiet);
                if (tc.is_null() && quiet == 0) {
                    retval = cmd_retval::CMD_RETURN_ERROR;
                    break 'out;
                }
            } else {
                tc = cmd_find_client(item, null_mut(), 1);
            }
            (*item).target_client = tc;

            retval = cmdq_find_flag(item, &raw mut (*item).source, &raw mut (*entry).source);
            if (retval == cmd_retval::CMD_RETURN_ERROR) {
                break 'out;
            }
            retval = cmdq_find_flag(item, &raw mut (*item).target, &raw mut (*entry).target);
            if (retval == cmd_retval::CMD_RETURN_ERROR) {
                break 'out;
            }

            retval = ((*entry).exec.unwrap())(cmd, item);
            if (retval == cmd_retval::CMD_RETURN_ERROR) {
                break 'out;
            }

            if ((*entry).flags & CMD_AFTERHOOK != 0) {
                fsp = if cmd_find_valid_state(&raw mut (*item).target) != 0 {
                    &raw mut (*item).target
                } else if cmd_find_valid_state(&raw mut (*(*item).state).current) != 0 {
                    &raw mut (*(*item).state).current
                } else if cmd_find_from_client(&raw mut fs, (*item).client, 0) == 0 {
                    &raw mut fs
                } else {
                    break 'out;
                };
                cmdq_insert_hook((*fsp).s, item, fsp, c"after-%s".as_ptr(), (*entry).name);
            }
        }

        (*item).client = saved;
        if (retval == cmd_retval::CMD_RETURN_ERROR) {
            fsp = null_mut();
            if cmd_find_valid_state(&raw mut (*item).target) != 0 {
                fsp = &raw mut (*item).target;
            } else if cmd_find_valid_state(&raw mut (*(*item).state).current) != 0 {
                fsp = &raw mut (*(*item).state).current;
            } else if (cmd_find_from_client(&raw mut fs, (*item).client, 0) == 0) {
                fsp = &raw mut fs;
            }
            cmdq_insert_hook(
                if !fsp.is_null() { (*fsp).s } else { null_mut() },
                item,
                fsp,
                c"command-error".as_ptr(),
            );
            cmdq_guard(item, c"error".as_ptr(), flags);
        } else {
            cmdq_guard(item, c"end".as_ptr(), flags);
        }
        retval
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_callback1(name: *const c_char, cb: cmdq_cb, data: *mut c_char) -> NonNull<cmdq_item> {
    let item = xcalloc_::<cmdq_item>(1).as_ptr();

    unsafe {
        xasprintf(&raw mut (*item).name, c"[%s/%p]".as_ptr(), name, item);
        (*item).type_ = cmdq_type::CMDQ_CALLBACK;

        (*item).group = 0;
        (*item).state = cmdq_new_state(null_mut(), null_mut(), 0);

        (*item).cb = cb;
        (*item).data = data as _;

        NonNull::new_unchecked(item)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_error_callback(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    let error = data as *mut c_char;

    unsafe {
        cmdq_error(item, c"%s".as_ptr(), error);
        free_(error);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_get_error(error: *const c_char) -> NonNull<cmdq_item> {
    unsafe { cmdq_get_callback!(cmdq_error_callback, xstrdup(error).as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_fire_callback(item: *mut cmdq_item) -> cmd_retval {
    unsafe { ((*item).cb.unwrap())(item, (*item).data) }
}

// TODO this translation is broken
/*
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_next(c: *mut client) -> u32 {
    let __func__ = c"cmdq_next".as_ptr();
    static mut number: u32 = 0;
    let mut items = 0;
    let mut retval: cmd_retval = cmd_retval::CMD_RETURN_NORMAL;

    unsafe {
        let mut queue = cmdq_get(c);
        let mut name = cmdq_name(c);

        'waiting: {
            if tailq_empty(&raw mut (*queue).list) {
                log_debug(c"%s %s: empty".as_ptr(), __func__, name);
                return 0;
            }
            if (*tailq_first(&raw mut (*queue).list)).flags & CMDQ_WAITING != 0 {
                log_debug(c"%s %s: waiting".as_ptr(), __func__, name);
                return 0;
            }

            log_debug(c"%s %s: enter".as_ptr(), __func__, name);
            loop {
                (*queue).item = tailq_first(&raw mut (*queue).list);
                let item = (*queue).item;
                if item.is_null() {
                    break;
                }
                log_debug(
                    c"%s %s: %s (%d), flags %x".as_ptr(),
                    __func__,
                    name,
                    (*item).name,
                    (*item).type_,
                    (*item).flags,
                );

                if ((*item).flags & CMDQ_WAITING != 0) {
                    break 'waiting;
                }

                if (!(*item).flags & CMDQ_FIRED != 0) {
                    (*item).time = libc::time(null_mut());
                    number += 1;
                    (*item).number = number;

                    match (*item).type_ {
                        cmdq_type::CMDQ_COMMAND => {
                            retval = cmdq_fire_command(item);

                            if (retval == cmd_retval::CMD_RETURN_ERROR) {
                                cmdq_remove_group(item);
                            }
                            break;
                        }
                        cmdq_type::CMDQ_CALLBACK => {
                            retval = cmdq_fire_callback(item);
                        }
                        _ => {
                            retval = cmd_retval::CMD_RETURN_ERROR;
                        }
                    }
                    (*item).flags |= CMDQ_FIRED;

                    if (retval == cmd_retval::CMD_RETURN_WAIT) {
                        (*item).flags |= CMDQ_WAITING;
                        break 'waiting;
                    }
                    items += 1;
                }
                cmdq_remove(item);
            }
            (*queue).item = null_mut();

            log_debug(c"%s %s: exit (empty)".as_ptr(), __func__, name);
            return items;
        } // 'waiting
        //waiting:
        log_debug(c"%s %s: exit (wait)".as_ptr(), __func__, name);
        items
    }
}
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_running(c: *mut client) -> *mut cmdq_item {
    unsafe {
        let queue = cmdq_get(c);

        if (*queue).item.is_null() {
            return null_mut();
        }
        if (*(*queue).item).flags & CMDQ_WAITING != 0 {
            return null_mut();
        }
        (*queue).item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_guard(item: *mut cmdq_item, guard: *const c_char, flags: i32) {
    unsafe {
        let mut c = (*item).client;
        let t = (*item).time;
        let number = (*item).number;

        if !c.is_null() && (*c).flags.intersects(client_flag::CONTROL) {
            control_write(c, c"%%%s %ld %u %d".as_ptr(), guard, t, number, flags);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_print_data(item: *mut cmdq_item, parse: i32, evb: *mut evbuffer) {
    unsafe {
        server_client_print((*item).client, parse, evb);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_print(item: *mut cmdq_item, fmt: *const c_char, mut args: ...) {
    unsafe {
        let evb = evbuffer_new();
        if (evb.is_null()) {
            fatalx(c"out of memory".as_ptr());
        }

        evbuffer_add_vprintf(evb, fmt, core::mem::transmute(args.as_va_list()));

        cmdq_print_data(item, 0, evb);
        evbuffer_free(evb);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmdq_error(item: *mut cmdq_item, fmt: *const c_char, mut args: ...) {
    let __func__ = c"cmdq_error".as_ptr();
    unsafe {
        let mut c = (*item).client;
        let mut cmd = (*item).cmd;
        let mut msg = null_mut();
        let mut tmp = null_mut();
        let mut file = null();
        let mut line = 0u32;

        xvasprintf(&raw mut msg, fmt, args.as_va_list());

        log_debug(c"%s: %s".as_ptr(), __func__, msg);

        if c.is_null() {
            cmd_get_source(cmd, &raw mut file, &raw mut line);
            cfg_add_cause(c"%s:%u: %s".as_ptr(), file, line, msg);
        } else if ((*c).session.is_null() || (*c).flags.intersects(client_flag::CONTROL)) {
            server_add_message(c"%s message: %s".as_ptr(), (*c).name, msg);
            if (!(*c).flags.intersects(client_flag::UTF8)) {
                tmp = msg;
                msg = utf8_sanitize(tmp);
                free_(tmp);
            }
            if ((*c).flags.intersects(client_flag::CONTROL)) {
                control_write(c, c"%s".as_ptr(), msg);
            } else {
                file_error(c, c"%s\n".as_ptr(), msg);
            }
            (*c).retval = 1;
        } else {
            *msg = toupper((*msg) as i32) as _;
            status_message_set(c, -1, 1, 0, c"%s".as_ptr(), msg);
        }

        free_(msg);
    }
}
