#![allow(dead_code)]
use super::*;

use compat_rs::{
    queue::tailq_head,
    // tree::rb_foreach
};

static mut alerts_list: tailq_head<window> = tailq_head::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn alerts_reset_all() {
    // rb_foreach(&raw mut windows, |w| {});
    todo!()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alerts_queue(_window: *mut window, _flags: c_int) {
    todo!()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alerts_check_session(_session: *mut session) {
    todo!()
}
