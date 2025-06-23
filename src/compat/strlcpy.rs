use core::ffi::c_char;

/// The strlcpy() function copies up to size - 1 characters from the NUL-terminated string src to dst,
/// NUL-terminating the result.
pub unsafe fn strlcpy(dst: *mut c_char, src: *const c_char, siz: usize) -> usize {
    unsafe {
        let len = libc::strnlen(src, siz);
        core::ptr::copy_nonoverlapping(src, dst, len);
        *dst.add(len) = b'\0' as i8;

        len
    }
}
