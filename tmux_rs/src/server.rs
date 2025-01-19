use super::*;

#[unsafe(no_mangle)]
pub static mut clients: clients = unsafe { zeroed() };
#[unsafe(no_mangle)]
pub static mut server_proc: *mut tmuxproc = null_mut();
pub static mut server_fd: c_int = -1;
pub static mut server_client_flags: u64 = 0;
pub static mut server_exit: c_int = 0;
pub static mut server_ev_accept: event = unsafe { zeroed() };
pub static mut server_ev_tidy: event = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub static mut marked_pane: cmd_find_state = unsafe { zeroed() };

pub static mut message_next: c_uint = 0;
#[unsafe(no_mangle)]
pub static mut message_log: message_list = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub static mut current_time: time_t = unsafe { zeroed() };

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_set_marked(s: *mut session, wl: *mut winlink, wp: *mut window_pane) {
    unsafe {
        cmd_find_clear_state(&raw mut marked_pane, 0);
        marked_pane.s = s;
        marked_pane.wl = wl;
        marked_pane.w = (*wl).window;
        marked_pane.wp = wp;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_clear_marked() {
    unsafe {
        cmd_find_clear_state(&raw mut marked_pane, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_is_marked(s: *mut session, wl: *mut winlink, wp: *mut window_pane) -> c_int {
    if s.is_null() || wl.is_null() || wp.is_null() {
        return 0;
    }

    unsafe {
        if marked_pane.s != s || marked_pane.wl != wl {
            return 0;
        }
        if marked_pane.wp != wp {
            return 0;
        }
        server_check_marked()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_check_marked() -> c_int {
    unsafe { cmd_find_valid_state(&raw mut marked_pane) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_create_socket(_flags: u64, _cause: *mut *mut c_char) -> c_int {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_start(
    _client: *mut tmuxproc,
    _flags: u64,
    _base: *mut event_base,
    _lockfd: c_int,
    _lockfile: *mut c_char,
) -> c_int {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_update_socket() {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_accept(_timeout: c_int) {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_message(_fmt: *const c_char, _args: ...) {
    todo!()
}
