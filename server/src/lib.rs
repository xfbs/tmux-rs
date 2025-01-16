#![feature(extern_types)]
#![feature(c_variadic)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(non_upper_case_globals)]

use tmux_h::*;

pub static mut clients: clients = unsafe { zeroed() };
pub static mut server_proc: *mut tmuxproc = null_mut();
pub static mut server_fd: c_int = -1;
pub static mut server_client_flags: u64 = 0;
pub static mut server_exit: c_int = 0;
pub static mut server_ev_accept: event = unsafe { zeroed() };
pub static mut server_ev_tidy: event = unsafe { zeroed() };

pub static mut marked_pane: cmd_find_state = unsafe { zeroed() };

pub static mut message_next: c_uint = 0;
pub static mut message_log: message_list = unsafe { zeroed() };

pub static mut current_time: time_t = unsafe { zeroed() };

unsafe extern "C" {
    fn cmd_find_clear_state(...);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_set_marked(
    s: *mut session,
    wl: *mut winlink,
    wp: *mut window_pane,
) {
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
pub unsafe extern "C" fn server_is_marked(
    s: *mut session,
    wl: *mut winlink,
    wp: *mut window_pane,
) -> c_int {
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
pub unsafe extern "C" fn server_create_socket(flags: u64, cause: *mut *mut c_char) -> c_int {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_start(
    client: *mut tmuxproc,
    flags: u64,
    base: *mut event_base,
    lockfd: c_int,
    lockfile: *mut c_char,
) -> c_int {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_update_socket() {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_accept(timeout: c_int) {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_add_message(fmt: *const c_char, args: ...) {
    todo!()
}
