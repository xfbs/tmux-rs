use core::ffi::{c_char, c_int};
use libc::{pid_t, termios, winsize};

unsafe extern "C" {
    unsafe fn forkpty(_: *mut c_int, _: *mut c_char, _: *const termios, _: *const winsize) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn getptmfd() -> c_int {
    c_int::MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fdforkpty(
    ptmfd: c_int,
    master: *mut c_int,
    name: *mut c_char,
    tio: *mut termios,
    ws: *mut winsize,
) -> pid_t {
    unsafe { forkpty(master, name, tio, ws) }
}
