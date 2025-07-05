#[cfg(target_os = "linux")]
pub unsafe fn getprogname() -> *const libc::c_char {
    unsafe extern "C" {
        static mut program_invocation_short_name: *mut libc::c_char;
    }

    unsafe { program_invocation_short_name }
}

#[cfg(target_os = "macos")]
pub unsafe fn getprogname() -> *const libc::c_char {
    c"tmux".as_ptr()
}
