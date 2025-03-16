use libc::strcmp;

use crate::*;

unsafe extern "C" {
    // pub fn notify_hook(_: *mut cmdq_item, _: *const c_char);
    // pub fn notify_client(_: *const c_char, _: *mut client);
    // pub fn notify_session(_: *const c_char, _: *mut session);
    // pub fn notify_winlink(_: *const c_char, _: *mut winlink);
    // pub fn notify_session_window(_: *const c_char, _: *mut session, _: *mut window);
    // pub fn notify_window(_: *const c_char, _: *mut window);
    // pub fn notify_pane(_: *const c_char, _: *mut window_pane);
    // pub fn notify_paste_buffer(_: *const c_char, _: c_int);

    // pub unsafe fn notify_insert_hook(item: *mut cmdq_item, ne: *mut notify_entry);
    // pub unsafe fn notify_insert_one_hook( item: *mut cmdq_item, ne: *mut notify_entry, cmdlist: *mut cmd_list, state: *mut cmdq_state,) -> *mut cmdq_item;
    // pub unsafe fn notify_callback(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval;
    // pub unsafe fn notify_add( name: *const c_char, fs: *mut cmd_find_state, c: *mut client, s: *mut session, w: *mut window, wp: *mut window_pane, pbname: *const c_char,);
}

unsafe impl Zeroable for notify_entry {}
#[repr(C)]
pub struct notify_entry {
    pub name: *mut c_char,
    pub fs: cmd_find_state,
    pub formats: *mut format_tree,

    pub client: *mut client,
    pub session: *mut session,
    pub window: *mut window,
    pub pane: i32,
    pub pbname: *mut c_char,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_insert_one_hook(
    item: *mut cmdq_item,
    ne: *mut notify_entry,
    cmdlist: *mut cmd_list,
    state: *mut cmdq_state,
) -> *mut cmdq_item {
    let __func__ = c"notify_insert_one_hook".as_ptr();
    unsafe {
        if (cmdlist.is_null()) {
            return (item);
        }
        if (log_get_level() != 0) {
            let s = cmd_list_print(cmdlist, 0);
            log_debug(c"%s: hook %s is: %s".as_ptr(), __func__, (*ne).name, s);
            free_(s);
        }
        let new_item = cmdq_get_command(cmdlist, state);
        cmdq_insert_after(item, new_item)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_insert_hook(mut item: *mut cmdq_item, ne: *mut notify_entry) {
    let __func__ = c"notify_insert_hook".as_ptr();
    unsafe {
        // struct options			*oo;
        // struct cmdq_state		*state;
        // struct options_entry		*o;
        // struct options_array_item	*a;
        // struct cmd_list			*cmdlist;
        // const char			*value;
        // struct cmd_parse_result		*pr;

        log_debug(c"%s: inserting hook %s".as_ptr(), __func__, (*ne).name);

        let mut o: *mut options_entry = null_mut();
        let mut oo: *mut options = null_mut();
        let mut fs: cmd_find_state = zeroed();

        cmd_find_clear_state(&raw mut fs, 0);
        if cmd_find_empty_state(&raw mut (*ne).fs) != 0 || !cmd_find_valid_state(&raw mut (*ne).fs) {
            cmd_find_from_nothing(&raw mut fs, 0);
        } else {
            cmd_find_copy_state(&raw mut fs, &raw mut (*ne).fs);
        }

        if (fs.s.is_null()) {
            oo = global_s_options;
        } else {
            oo = (*fs.s).options;
        }
        o = options_get(oo, (*ne).name);
        if (o.is_null() && !fs.wp.is_null()) {
            oo = (*fs.wp).options;
            o = options_get(oo, (*ne).name);
        }
        if (o.is_null() && !fs.wl.is_null()) {
            oo = (*(*fs.wl).window).options;
            o = options_get(oo, (*ne).name);
        }
        if (o.is_null()) {
            log_debug(c"%s: hook %s not found".as_ptr(), __func__, (*ne).name);
            return;
        }

        let state = cmdq_new_state(&raw mut fs, null_mut(), CMDQ_STATE_NOHOOKS);
        cmdq_add_formats(state, (*ne).formats);

        if (*(*ne).name == b'@' as c_char) {
            let value = options_get_string(oo, (*ne).name);
            let pr = cmd_parse_from_string(value, null_mut());
            match (*pr).status {
                cmd_parse_status::CMD_PARSE_ERROR => {
                    log_debug(
                        c"%s: can't parse hook %s: %s".as_ptr(),
                        __func__,
                        (*ne).name,
                        (*pr).error,
                    );
                    free_((*pr).error);
                }
                cmd_parse_status::CMD_PARSE_SUCCESS => {
                    notify_insert_one_hook(item, ne, (*pr).cmdlist, state);
                }
            }
        } else {
            let mut a = options_array_first(o);
            while (!a.is_null()) {
                let cmdlist = (*options_array_item_value(a)).cmdlist;
                item = notify_insert_one_hook(item, ne, cmdlist, state);
                a = options_array_next(a);
            }
        }

        cmdq_free_state(state);
    }
}

// notify_callback
// notify_add

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_callback(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    let __func__ = c"notify_callback".as_ptr();
    unsafe {
        let mut ne = data as *mut notify_entry;

        log_debug(c"%s: %s".as_ptr(), __func__, (*ne).name);

        if (strcmp((*ne).name, c"pane-mode-changed".as_ptr()) == 0) {
            control_notify_pane_mode_changed((*ne).pane);
        }
        if (strcmp((*ne).name, c"window-layout-changed".as_ptr()) == 0) {
            control_notify_window_layout_changed((*ne).window);
        }
        if (strcmp((*ne).name, c"window-pane-changed".as_ptr()) == 0) {
            control_notify_window_pane_changed((*ne).window);
        }
        if (strcmp((*ne).name, c"window-unlinked".as_ptr()) == 0) {
            control_notify_window_unlinked((*ne).session, (*ne).window);
        }
        if (strcmp((*ne).name, c"window-linked".as_ptr()) == 0) {
            control_notify_window_linked((*ne).session, (*ne).window);
        }
        if (strcmp((*ne).name, c"window-renamed".as_ptr()) == 0) {
            control_notify_window_renamed((*ne).window);
        }
        if (strcmp((*ne).name, c"client-session-changed".as_ptr()) == 0) {
            control_notify_client_session_changed((*ne).client);
        }
        if (strcmp((*ne).name, c"client-detached".as_ptr()) == 0) {
            control_notify_client_detached((*ne).client);
        }
        if (strcmp((*ne).name, c"session-renamed".as_ptr()) == 0) {
            control_notify_session_renamed((*ne).session);
        }
        if (strcmp((*ne).name, c"session-created".as_ptr()) == 0) {
            control_notify_session_created((*ne).session);
        }
        if (strcmp((*ne).name, c"session-closed".as_ptr()) == 0) {
            control_notify_session_closed((*ne).session);
        }
        if (strcmp((*ne).name, c"session-window-changed".as_ptr()) == 0) {
            control_notify_session_window_changed((*ne).session);
        }
        if (strcmp((*ne).name, c"paste-buffer-changed".as_ptr()) == 0) {
            control_notify_paste_buffer_changed((*ne).pbname);
        }
        if (strcmp((*ne).name, c"paste-buffer-deleted".as_ptr()) == 0) {
            control_notify_paste_buffer_deleted((*ne).pbname);
        }

        notify_insert_hook(item, ne);

        if (!(*ne).client.is_null()) {
            server_client_unref((*ne).client);
        }
        if (!(*ne).session.is_null()) {
            session_remove_ref((*ne).session, __func__);
        }
        if (!(*ne).window.is_null()) {
            window_remove_ref((*ne).window, __func__);
        }

        if (!(*ne).fs.s.is_null()) {
            session_remove_ref((*ne).fs.s, __func__);
        }

        format_free((*ne).formats);
        free_((*ne).name);
        free_((*ne).pbname);
        free_(ne);
    }

    cmd_retval::CMD_RETURN_NORMAL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_add(
    name: *const c_char,
    fs: *mut cmd_find_state,
    c: *mut client,
    s: *mut session,
    w: *mut window,
    wp: *mut window_pane,
    pbname: *const c_char,
) {
    let __func__ = c"notify_add".as_ptr();
    unsafe {
        // struct notify_entry *ne;
        // struct cmdq_item *item;

        let item = cmdq_running(null_mut());
        if (!item.is_null() && cmdq_get_flags(item) & CMDQ_STATE_NOHOOKS != 0) {
            return;
        }

        let ne = xcalloc1::<notify_entry>() as *mut notify_entry;
        (*ne).name = xstrdup(name).as_ptr();

        (*ne).client = c;
        (*ne).session = s;
        (*ne).window = w;
        (*ne).pane = (if !wp.is_null() { (*wp).id as i32 } else { -1 });
        (*ne).pbname = (if !pbname.is_null() {
            xstrdup(pbname).as_ptr()
        } else {
            null_mut()
        });

        (*ne).formats = format_create(null_mut(), null_mut(), 0, FORMAT_NOJOBS);
        format_add((*ne).formats, c"hook".as_ptr(), c"%s".as_ptr(), name);
        if (!c.is_null()) {
            format_add((*ne).formats, c"hook_client".as_ptr(), c"%s".as_ptr(), (*c).name);
        }
        if (!s.is_null()) {
            format_add((*ne).formats, c"hook_session".as_ptr(), c"$%u".as_ptr(), (*s).id);
            format_add((*ne).formats, c"hook_session_name".as_ptr(), c"%s".as_ptr(), (*s).name);
        }
        if (!w.is_null()) {
            format_add((*ne).formats, c"hook_window".as_ptr(), c"@%u".as_ptr(), (*w).id);
            format_add((*ne).formats, c"hook_window_name".as_ptr(), c"%s".as_ptr(), (*w).name);
        }
        if (!wp.is_null()) {
            format_add((*ne).formats, c"hook_pane".as_ptr(), c"%%%d".as_ptr(), (*wp).id);
        }
        format_log_debug((*ne).formats, __func__);

        if (!c.is_null()) {
            (*c).references += 1;
        }
        if (!s.is_null()) {
            session_add_ref(s, __func__);
        }
        if (!w.is_null()) {
            window_add_ref(w, __func__);
        }

        cmd_find_copy_state(&raw mut (*ne).fs, fs);
        if (!(*ne).fs.s.is_null()) {
            /* cmd_find_valid_state needs session */
            session_add_ref((*ne).fs.s, __func__);
        }

        cmdq_append(null_mut(), cmdq_get_callback!(notify_callback, ne.cast()).as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_hook(item: *mut cmdq_item, name: *mut c_char) {
    let __func__ = c"notify_hook".as_ptr();
    unsafe {
        let mut target = cmdq_get_target(item);
        let mut ne: notify_entry = zeroed();

        ne.name = name;
        cmd_find_copy_state(&raw mut ne.fs, target);

        ne.client = cmdq_get_client(item);
        ne.session = (*target).s;
        ne.window = (*target).w;
        ne.pane = (if !(*target).wp.is_null() {
            (*(*target).wp).id as i32
        } else {
            -1
        });

        ne.formats = format_create(null_mut(), null_mut(), 0, FORMAT_NOJOBS);
        format_add(ne.formats, c"hook".as_ptr(), c"%s".as_ptr(), name);
        format_log_debug(ne.formats, __func__);

        notify_insert_hook(item, &raw mut ne);
        format_free(ne.formats);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_client(name: *const c_char, c: *mut client) {
    unsafe {
        let mut fs: cmd_find_state = zeroed(); // TODO use uninit

        cmd_find_from_client(&raw mut fs, c, 0);
        notify_add(name, &raw mut fs, c, null_mut(), null_mut(), null_mut(), null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_session(name: *const c_char, s: *mut session) {
    unsafe {
        let mut fs = zeroed(); // TODO use uninit

        if (session_alive(s) != 0) {
            cmd_find_from_session(&raw mut fs, s, 0);
        } else {
            cmd_find_from_nothing(&raw mut fs, 0);
        }
        notify_add(name, &raw mut fs, null_mut(), s, null_mut(), null_mut(), null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_winlink(name: *const c_char, wl: *mut winlink) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_winlink(&raw mut fs, wl, 0);
        notify_add(
            name,
            &raw mut fs,
            null_mut(),
            (*wl).session,
            (*wl).window,
            null_mut(),
            null_mut(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_session_window(name: *const c_char, s: *mut session, w: *mut window) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_session_window(&raw mut fs, s, w, 0);
        notify_add(name, &raw mut fs, null_mut(), s, w, null_mut(), null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_window(name: *const c_char, w: *mut window) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_window(&raw mut fs, w, 0);
        notify_add(name, &raw mut fs, null_mut(), null_mut(), w, null_mut(), null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_pane(name: *const c_char, wp: *mut window_pane) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_from_pane(&raw mut fs, wp, 0);
        notify_add(name, &raw mut fs, null_mut(), null_mut(), null_mut(), wp, null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn notify_paste_buffer(pbname: *const c_char, deleted: i32) {
    unsafe {
        let mut fs: cmd_find_state = zeroed();

        cmd_find_clear_state(&raw mut fs, 0);
        if (deleted != 0) {
            notify_add(
                c"paste-buffer-deleted".as_ptr(),
                &raw mut fs,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                pbname,
            );
        } else {
            notify_add(
                c"paste-buffer-changed".as_ptr(),
                &raw mut fs,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                pbname,
            );
        }
    }
}
