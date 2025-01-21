use super::*;
unsafe extern "C" {
    pub unsafe fn check_window_name(_: *mut window);
    pub unsafe fn default_window_name(_: *mut window) -> *mut c_char;
    pub unsafe fn parse_window_name(_: *const c_char) -> *mut c_char;
}
