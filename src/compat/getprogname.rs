unsafe extern "C" {
    static mut program_invocation_short_name: *mut libc::c_char;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getprogname() -> *const libc::c_char {
    unsafe { program_invocation_short_name }
}
