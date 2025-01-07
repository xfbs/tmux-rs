use ::core::ffi::c_int;

use tmux_h::{session, window};

use compat_rs::{queue::tailq_head, tree::rb_foreach};

static mut alerts_list: tailq_head<window> = tailq_head::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn alerts_reset_all() {
    // rb_foreach(&raw mut windows, |w| {});
    todo!()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alerts_queue(window: *mut window, flags: c_int) {
    todo!()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn alerts_check_session(session: *mut session) {
    todo!()
}
