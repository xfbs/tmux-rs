use core::{
    ffi::{c_int, c_void},
    mem::size_of,
    ops::ControlFlow,
    ptr::null_mut,
};

use bsd_sys::recallocarray;
use libc::{calloc, cmsghdr, free, msghdr};

use super::imsg::{ibuf, msgbuf};
use super::queue::{tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_next, tailq_remove};

const IOV_MAX: usize = 1024; // TODO find where IOV_MAX is defined

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_open(len: usize) -> *mut ibuf {
    if (len == 0) {
        *libc::__errno_location() = libc::EINVAL;
        return null_mut();
    }
    let buf: *mut ibuf = calloc(1, size_of::<ibuf>()) as *mut ibuf;
    if buf.is_null() {
        return null_mut();
    }
    (*buf).buf = calloc(len, 1) as *mut libc::c_uchar;
    if (*buf).buf.is_null() {
        free(buf as *mut libc::c_void);
        return null_mut();
    }

    (*buf).max = len;
    (*buf).size = len;
    (*buf).fd = -1;

    return (buf);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_dynamic(len: usize, max: usize) -> *mut ibuf {
    if (len == 0 || max < len) {
        *libc::__errno_location() = libc::EINVAL;
        return null_mut();
    }
    let buf: *mut ibuf = calloc(1, size_of::<ibuf>()) as *mut ibuf;
    if buf.is_null() {
        return null_mut();
    }
    if len > 0 {
        (*buf).buf = calloc(len, 1) as *mut libc::c_uchar;
        if (*buf).buf.is_null() {
            free(buf as *mut c_void);
            return null_mut();
        }
    }

    (*buf).max = len;
    (*buf).size = len;
    (*buf).fd = -1;

    return (buf);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_realloc(buf: *mut ibuf, len: usize) -> i32 {
    /* on static buffers max is eq size and so the following fails */
    if (len > usize::MAX - (*buf).wpos || (*buf).wpos + len > (*buf).max) {
        *libc::__errno_location() = libc::ERANGE;
        return -1;
    }

    let b = recallocarray((*buf).buf as *mut c_void, (*buf).size, (*buf).wpos + len, 1);
    if b.is_null() {
        return -1;
    }
    (*buf).buf = b as *mut u8;
    (*buf).size = (*buf).wpos + len;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_reserve(buf: *mut ibuf, len: usize) -> *mut c_void {
    if len > usize::MAX - (*buf).wpos || (*buf).max == 0 {
        *libc::__errno_location() = libc::ERANGE;
        return null_mut();
    }

    if (*buf).wpos + len > (*buf).size && ibuf_realloc(buf, len) == -1 {
        return null_mut();
    }

    let b = (*buf).buf.add((*buf).wpos);
    (*buf).wpos += len;
    b as _
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add(buf: *mut ibuf, data: *const c_void, len: usize) -> i32 {
    let b = ibuf_reserve(buf, len);

    if b.is_null() {
        return -1;
    }

    libc::memcpy(b, data, len);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_ibuf(buf: *mut ibuf, from: *const ibuf) -> c_int {
    ibuf_add(buf, ibuf_data(from), ibuf_size(from))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_buf(buf: *mut ibuf, from: *const ibuf) -> c_int {
    ibuf_add_ibuf(buf, from)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_n8(buf: *mut ibuf, value: u64) -> c_int {
    if value > u8::MAX as u64 {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = value;
    ibuf_add(buf, &raw const v as _, size_of::<u8>())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_n16(buf: *mut ibuf, value: u64) -> c_int {
    if value > u16::MAX as u64 {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = (value as u16).to_be();
    ibuf_add(buf, &raw const v as _, size_of::<u16>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_n32(buf: *mut ibuf, value: u64) -> c_int {
    if value > u32::MAX as u64 {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = (value as u32).to_be();
    ibuf_add(buf, &raw const v as _, size_of::<u32>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_n64(buf: *mut ibuf, value: u64) -> c_int {
    let v = value.to_be();
    ibuf_add(buf, &raw const v as _, size_of::<u64>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_h16(buf: *mut ibuf, value: u64) -> c_int {
    if value > u16::MAX as u64 {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = value as u16;
    ibuf_add(buf, &raw const v as _, size_of::<u16>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_h32(buf: *mut ibuf, value: u64) -> c_int {
    if value > u32::MAX as u64 {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = value as u32;
    ibuf_add(buf, &raw const v as _, size_of::<u32>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_h64(buf: *mut ibuf, value: u64) -> c_int {
    ibuf_add(buf, &raw const value as _, size_of::<u64>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_add_zero(buf: *mut ibuf, len: usize) -> c_int {
    let b: *mut c_void = ibuf_reserve(buf, len);
    if b.is_null() {
        return -1;
    }
    libc::memset(b, 0, len);

    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_seek(buf: *mut ibuf, pos: usize, len: usize) -> *mut c_void {
    /* only allow seeking between rpos and wpos */
    if ibuf_size(buf) < pos || usize::MAX - pos < len || ibuf_size(buf) < pos + len {
        *libc::__errno_location() = libc::ERANGE;
        return null_mut();
    }

    (*buf).buf.add((*buf).rpos + pos) as _
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set(buf: *mut ibuf, pos: usize, data: *const c_void, len: usize) -> c_int {
    let b = ibuf_seek(buf, pos, len);
    if b.is_null() {
        return -1;
    }

    libc::memcpy(b, data, len);
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_n8(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    if (value > u8::MAX as u64) {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = value as u8;
    ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u8>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_n16(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    if (value > u16::MAX as u64) {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = u16::to_be(value as u16);
    ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u16>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_n32(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    if (value > u32::MAX as u64) {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = u32::to_be(value as u32);
    ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u32>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_n64(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    let v = u64::to_be(value);
    ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u64>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_h16(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    if (value > u16::MAX as u64) {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = value as u16;
    ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u16>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_h32(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    if (value > u32::MAX as u64) {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    let v = value as u32;
    ibuf_set(buf, pos, &raw const v as *const c_void, size_of::<u32>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_set_h64(buf: *mut ibuf, pos: usize, value: u64) -> c_int {
    ibuf_set(buf, pos, &raw const value as *const c_void, size_of::<u64>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_data(buf: *const ibuf) -> *mut c_void {
    (*buf).buf.add((*buf).rpos) as *mut c_void
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_size(buf: *const ibuf) -> usize {
    (*buf).wpos - (*buf).rpos
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_left(buf: *const ibuf) -> usize {
    if (*buf).max == 0 {
        return 0;
    }
    (*buf).max - (*buf).wpos
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_truncate(buf: *mut ibuf, len: usize) -> c_int {
    if ibuf_size(buf) >= len {
        (*buf).wpos = (*buf).rpos + len;
        return 0;
    }
    if (*buf).max == 0 {
        /* only allow to truncate down */
        *libc::__errno_location() = libc::ERANGE;
        return -1;
    }
    return ibuf_add_zero(buf, len - ibuf_size(buf));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_rewind(buf: *mut ibuf) {
    (*buf).rpos = 0;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_close(msgbuf: *mut msgbuf, buf: *mut ibuf) {
    ibuf_enqueue(msgbuf, buf);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_from_buffer(buf: *mut ibuf, data: *mut c_void, len: usize) {
    libc::memset(buf as _, 0, size_of::<ibuf>());
    (*buf).buf = data as _;
    (*buf).wpos = len;
    (*buf).size = len;
    (*buf).fd = -1;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_from_ibuf(buf: *mut ibuf, from: *const ibuf) {
    ibuf_from_buffer(buf, ibuf_data(from), ibuf_size(from));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get(buf: *mut ibuf, data: *mut c_void, len: usize) -> c_int {
    if ibuf_size(buf) < len {
        *libc::__errno_location() = libc::EBADMSG;
        return -1;
    }

    libc::memcpy(data, ibuf_data(buf), len);
    (*buf).rpos += len;
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_ibuf(buf: *mut ibuf, len: usize, new: *mut ibuf) -> c_int {
    if ibuf_size(buf) < len {
        *libc::__errno_location() = libc::EBADMSG;
        return -1;
    }

    ibuf_from_buffer(new, ibuf_data(buf), len);
    (*buf).rpos += len;
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_n8(buf: *mut ibuf, value: *mut u8) -> c_int {
    ibuf_get(buf, value as _, size_of::<u8>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_n16(buf: *mut ibuf, value: *mut u16) -> c_int {
    let rv = ibuf_get(buf, value as _, size_of::<u16>());
    *value = u16::from_be(*value);
    rv
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_n32(buf: *mut ibuf, value: *mut u32) -> c_int {
    let rv = ibuf_get(buf, value as _, size_of::<u32>());
    *value = u32::from_be(*value);
    rv
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_n64(buf: *mut ibuf, value: *mut u64) -> c_int {
    let rv = ibuf_get(buf, value as _, size_of::<u64>());
    *value = u64::from_be(*value);
    rv
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_h16(buf: *mut ibuf, value: *mut u16) -> c_int {
    ibuf_get(buf, value as _, size_of::<u16>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_h32(buf: *mut ibuf, value: *mut u32) -> c_int {
    ibuf_get(buf, value as _, size_of::<u32>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_get_h64(buf: *mut ibuf, value: *mut u64) -> c_int {
    ibuf_get(buf, value as _, size_of::<u64>())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_skip(buf: *mut ibuf, len: usize) -> c_int {
    if ibuf_size(buf) < len {
        *libc::__errno_location() = libc::EBADMSG;
        return -1;
    }

    (*buf).rpos += len;
    0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_free(buf: *mut ibuf) {
    if buf.is_null() {
        return;
    }
    if (*buf).max == 0 {
        /* if buf lives on the stack */
        libc::abort(); /* abort before causing more harm */
    }
    if ((*buf).fd != -1) {
        libc::close((*buf).fd);
    }
    bsd_sys::freezero((*buf).buf as _, (*buf).size);
    libc::free(buf as *mut libc::c_void);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_fd_avail(buf: *mut ibuf) -> c_int {
    ((*buf).fd != -1) as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_fd_get(buf: *mut ibuf) -> c_int {
    let fd = (*buf).fd;
    (*buf).fd = -1;
    fd
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_fd_set(buf: *mut ibuf, fd: c_int) {
    if (*buf).max == 0 {
        /* if buf lives on the stack */
        libc::abort(); /* abort before causing more harm */
    }
    if (*buf).fd != -1 {
        libc::close((*buf).fd);
    }
    (*buf).fd = fd;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ibuf_write(msgbuf: *mut msgbuf) -> c_int {
    let mut iov: [libc::iovec; IOV_MAX] = std::mem::zeroed();
    let mut i: u32 = 0;
    let mut n: isize = 0;

    tailq_foreach(&raw mut (*msgbuf).bufs, |buf| {
        if i as usize >= IOV_MAX {
            return std::ops::ControlFlow::Break(());
        }
        iov[i as usize].iov_base = ibuf_data(buf);
        iov[i as usize].iov_len = ibuf_size(buf);
        i += 1;

        std::ops::ControlFlow::Continue(())
    });

    loop {
        n = libc::writev((*msgbuf).fd, iov.as_ptr(), i as i32);
        if n == -1 {
            if *libc::__errno_location() == libc::EINTR {
                continue;
            }
            if *libc::__errno_location() == libc::ENOBUFS {
                *libc::__errno_location() = libc::EAGAIN;
            }
            return -1;
        }
    }

    if n == 0 {
        // connection closed
        *libc::__errno_location() = 0;
        return 0;
    }

    msgbuf_drain(msgbuf, n as usize);

    1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msgbuf_init(msgbuf: *mut msgbuf) {
    (*msgbuf).queued = 0;
    (*msgbuf).fd = -1;
    tailq_init(&raw mut (*msgbuf).bufs);
}

unsafe fn msgbuf_drain(msgbuf: *mut msgbuf, mut n: usize) {
    let mut buf = tailq_first(&raw mut (*msgbuf).bufs);

    while !buf.is_null() && n > 0 {
        let next = tailq_next(buf);
        if (n >= ibuf_size(buf)) {
            n -= ibuf_size(buf);
            ibuf_dequeue(msgbuf, buf);
        } else {
            (*buf).rpos += n;
            n = 0;
        }
        buf = next;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn msgbuf_clear(msgbuf: *mut msgbuf) {
    loop {
        let buf = tailq_first(&raw mut (*msgbuf).bufs);
        if buf.is_null() {
            break;
        }
        ibuf_dequeue(msgbuf, buf);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msgbuf_write(msgbuf: *mut msgbuf) -> c_int {
    let mut iov: [libc::iovec; IOV_MAX] = std::mem::zeroed();
    let mut buf: *mut ibuf = null_mut();
    let mut buf0: *mut ibuf = null_mut();
    let mut i: u32 = 0;
    let mut n = 0;
    let mut msg: msghdr = std::mem::zeroed();
    let mut cmsg: cmsghdr = std::mem::zeroed();
    let mut cmsgbuf: cmsgbuf = std::mem::zeroed();
    union cmsgbuf {
        hdr: cmsghdr,
        buf: [u8; unsafe { libc::CMSG_SPACE(size_of::<c_int>() as _) as usize }],
    }

    tailq_foreach(&raw mut (*msgbuf).bufs, |buf| {
        if i as usize >= IOV_MAX {
            return ControlFlow::Break(());
        }
        if i > 0 && (*buf).fd != -1 {
            return ControlFlow::Break(());
        }
        iov[i as usize].iov_base = ibuf_data(buf);
        iov[i as usize].iov_len = ibuf_size(buf);
        i += 1;
        if (*buf).fd != -1 {
            buf0 = buf;
        }
        ControlFlow::Continue(())
    });

    msg.msg_iov = iov.as_mut_ptr();
    msg.msg_iovlen = 1;

    if !buf0.is_null() {
        msg.msg_control = &raw mut cmsgbuf.buf as _;
        msg.msg_controllen = size_of_val(&cmsgbuf.buf);
        cmsg = *libc::CMSG_FIRSTHDR(&raw const msg);
        cmsg.cmsg_len = libc::CMSG_LEN(size_of::<c_int>() as u32) as usize;
        cmsg.cmsg_level = libc::SOL_SOCKET;
        cmsg.cmsg_type = libc::SCM_RIGHTS;
        *(libc::CMSG_DATA(&raw const cmsg) as *mut c_int) = (*buf0).fd;
    }

    loop {
        let n = libc::sendmsg((*msgbuf).fd, &msg, 0);
        if n == -1 {
            if *libc::__errno_location() == libc::EINTR {
                continue;
            }
            if *libc::__errno_location() == libc::ENOBUFS {
                *libc::__errno_location() = libc::EAGAIN;
            }
            return -1;
        }
    }

    if n == 0 {
        *libc::__errno_location() = 0;
        return 0;
    }

    if !buf0.is_null() {
        libc::close((*buf0).fd);
        (*buf0).fd = -1;
    }

    msgbuf_drain(msgbuf, n);

    1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn msgbuf_queuelen(msgbuf: *mut msgbuf) -> u32 {
    (*msgbuf).queued
}

unsafe fn ibuf_enqueue(msgbuf: *mut msgbuf, buf: *mut ibuf) {
    if (*buf).max == 0 {
        /* if buf lives on the stack */
        libc::abort(); /* abort before causing more harm */
    }
    tailq_insert_tail::<_, _>(&raw mut (*msgbuf).bufs, buf);
    (*msgbuf).queued += 1;
}

unsafe fn ibuf_dequeue(msgbuf: *mut msgbuf, buf: *mut ibuf) {
    tailq_remove(&raw mut (*msgbuf).bufs, buf);
    (*msgbuf).queued -= 1;
    ibuf_free(buf);
}
