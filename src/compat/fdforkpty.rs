use core::ffi::{c_char, c_int};
use libc::{forkpty, pid_t, termios, winsize};

pub extern "C" fn getptmfd() -> c_int {
    c_int::MAX
}

pub unsafe fn fdforkpty(
    _ptmfd: c_int,
    master: *mut c_int,
    name: *mut c_char,
    tio: *mut termios,
    ws: *mut winsize,
) -> pid_t {
    unsafe { forkpty(master, name, tio, ws) }
}
