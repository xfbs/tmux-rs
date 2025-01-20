#![allow(dead_code)]
use super::*;

use compat_rs::{
    queue::tailq_head,
    // tree::rb_foreach
};

static mut alerts_list: tailq_head<window> = tailq_head::new();

unsafe extern "C" fn alerts_timer(_fd: i32, _events: u16, arg: *mut c_void) {
    let w = arg as *mut window;

    unsafe {
        log_debug(c"@%u alerts timer expired".as_ptr(), (*w).id);
        alerts_queue(w, WINDOW_SILENCE);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_reset_all() {
    // rb_foreach(&raw mut windows, |w| {});
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_queue(_window: *mut window, _flags: c_int) {
    todo!()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alerts_check_session(_session: *mut session) {
    todo!()
}
