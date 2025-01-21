use crate::*;

unsafe extern "C" {
    pub unsafe static mut global_options: *mut options;
    pub unsafe static mut global_s_options: *mut options;
    pub unsafe static mut global_w_options: *mut options;
    pub unsafe static mut global_environ: *mut environ;
    pub unsafe static mut start_time: timeval;
    pub unsafe static mut socket_path: *mut c_char;
    pub unsafe static mut ptm_fd: c_int;
    pub unsafe static mut shell_command: *mut c_char;

    pub unsafe fn checkshell(_: *mut c_char) -> c_int;
    pub unsafe fn setblocking(_: c_int, _: c_int);
    pub unsafe fn shell_argv0(_: *mut c_char, _: c_int) -> *mut c_char;
    pub unsafe fn get_timer() -> u64;
    pub unsafe fn sig2name(_: i32) -> *mut c_char;
    pub unsafe fn find_cwd() -> *mut c_char;
    pub unsafe fn find_home() -> *mut c_char;
    pub unsafe fn getversion() -> *mut c_char;
}
