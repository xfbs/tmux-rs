use core::ffi::{c_char, c_void};

unsafe extern "C" {
    pub fn __b64_ntop(
        src: *const u8,
        srclength: usize,
        target: *mut c_char,
        targsize: usize,
    ) -> i32;
    pub fn __b64_pton(src: *const c_char, target: *mut u8, targsize: usize) -> i32;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn b64_ntop(
    src: *const u8,
    srclength: usize,
    target: *mut c_char,
    targsize: usize,
) -> i32 {
    unsafe { __b64_ntop(src, srclength, target, targsize) }
}

// https://www.rfc-editor.org/rfc/rfc4648

// skips all whitespace anywhere.
// converts characters, four at a time, starting at (or after)
// src from base - 64 numbers into three 8 bit bytes in the target area.
// it returns the number of data bytes stored at the target, or -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn b64_pton(src: *const c_char, target: *mut u8, targsize: usize) -> i32 {
    unsafe { __b64_pton(src, target, targsize) }
}
