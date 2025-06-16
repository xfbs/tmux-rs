unsafe extern "C" {
    pub fn systemd_create_socket(flags: i32, cause: *mut *mut core::ffi::c_char) -> i32;
}

/*
#[no_mangle]
pub extern "C" fn systemd_create_socket(flags: i32, cause: *mut *mut core::ffi::c_char) -> i32 {
    //
    todo!()
}
*/
