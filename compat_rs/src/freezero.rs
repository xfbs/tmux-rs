#[no_mangle]
pub unsafe extern "C" fn freezero(ptr: *mut libc::c_void, size: usize) {
    if !ptr.is_null() {
        libc::memset(ptr, 0i32, size);
        libc::free(ptr);
    }
}
