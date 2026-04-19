//! Internal copies of the BSD-style `libc` helpers used by the imsg
//! layer. Duplicated (rather than shared via another workspace crate)
//! because the footprint is small and stable; the originals live in
//! `src/compat/` and are used by other parts of tmux-rs.

use core::ffi::c_void;
use core::ptr::null_mut;

// ---------------------------------------------------------------------
// errno — platform-specific accessor. Matches the main crate's
// `crate::errno!()` macro.
// ---------------------------------------------------------------------

#[cfg(target_os = "linux")]
macro_rules! errno {
    () => {
        *::libc::__errno_location()
    };
}
#[cfg(target_os = "macos")]
macro_rules! errno {
    () => {
        *::libc::__error()
    };
}
pub(crate) use errno;

// ---------------------------------------------------------------------
// getdtablecount — number of open file descriptors for this process.
// OpenBSD has a syscall; on Linux we count `/proc/self/fd` entries.
// ---------------------------------------------------------------------

pub(crate) fn getdtablecount() -> i32 {
    if let Ok(read_dir) = std::fs::read_dir("/proc/self/fd") {
        read_dir.count() as i32
    } else {
        0
    }
}

// ---------------------------------------------------------------------
// freezero — scrub-then-free, for memory that may hold sensitive data.
// ---------------------------------------------------------------------

pub(crate) unsafe fn freezero(ptr: *mut c_void, size: usize) {
    unsafe {
        if !ptr.is_null() {
            libc::memset(ptr, 0, size);
            libc::free(ptr);
        }
    }
}

// ---------------------------------------------------------------------
// recallocarray — realloc with zero-fill on grow; scrub-on-shrink.
// OpenBSD extension; ported here for the ibuf grow path.
// ---------------------------------------------------------------------

pub(crate) unsafe fn recallocarray(
    ptr: *mut c_void,
    oldnmemb: usize,
    newnmemb: usize,
    size: usize,
) -> *mut c_void {
    const MUL_NO_OVERFLOW: usize = 1usize << (size_of::<usize>() * 4);

    unsafe extern "C" {
        fn getpagesize() -> i32;
    }

    unsafe {
        if ptr.is_null() {
            return libc::calloc(newnmemb, size);
        }

        if (newnmemb >= MUL_NO_OVERFLOW || size >= MUL_NO_OVERFLOW)
            && newnmemb > 0
            && usize::MAX / newnmemb < size
        {
            errno!() = libc::ENOMEM;
            return null_mut();
        }
        let newsize = newnmemb * size;

        if (oldnmemb >= MUL_NO_OVERFLOW || size >= MUL_NO_OVERFLOW)
            && oldnmemb > 0
            && usize::MAX / oldnmemb < size
        {
            errno!() = libc::EINVAL;
            return null_mut();
        }
        let oldsize = oldnmemb * size;

        // Don't bother too much if we're shrinking just a bit.
        if newsize <= oldsize {
            let d = oldsize - newsize;
            if d < oldsize / 2 && d < getpagesize() as usize {
                libc::memset((ptr as *mut u8).add(newsize).cast(), 0, d);
                return ptr;
            }
        }

        let newptr = libc::malloc(newsize);
        if newptr.is_null() {
            return null_mut();
        }

        if newsize > oldsize {
            libc::memcpy(newptr, ptr, oldsize);
            libc::memset(
                (newptr as *mut u8).add(oldsize).cast(),
                0,
                newsize - oldsize,
            );
        } else {
            libc::memcpy(newptr, ptr, newsize);
        }

        libc::memset(ptr, 0, oldsize);
        libc::free(ptr);

        newptr
    }
}
