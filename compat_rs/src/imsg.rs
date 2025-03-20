use core::ffi::{c_int, c_uchar, c_void};
use std::{mem::MaybeUninit, ptr::null_mut};

use libc::{
    __errno_location, CMSG_DATA, CMSG_FIRSTHDR, CMSG_NXTHDR, CMSG_SPACE, EAGAIN, EBADMSG, EINTR, EINVAL, ERANGE,
    SCM_RIGHTS, SOL_SOCKET, c_char, calloc, close, cmsghdr, free, getdtablesize, iovec, memcpy, memmove, msghdr, pid_t,
};

use crate::getdtablecount::getdtablecount;
use crate::imsg_buffer::{
    ibuf_add, ibuf_add_buf, ibuf_close, ibuf_data, ibuf_dynamic, ibuf_fd_avail, ibuf_fd_set, ibuf_free, ibuf_get,
    ibuf_get_ibuf, ibuf_open, ibuf_rewind, ibuf_size, msgbuf_clear, msgbuf_init, msgbuf_write,
};
use crate::queue::{Entry, tailq_entry, tailq_first, tailq_head, tailq_init, tailq_insert_tail, tailq_remove};

// begin imsg.h

pub const MAX_IMSGSIZE: usize = 16384;
const IMSGF_HASFD: u16 = 1; // this needs to be u16, i think, but it's u32 in auto generated header

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ibuf {
    pub entry: tailq_entry<ibuf>,
    pub buf: *mut c_uchar,
    pub size: usize,
    pub max: usize,
    pub wpos: usize,
    pub rpos: usize,
    pub fd: c_int,
}

impl Entry<ibuf> for ibuf {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<ibuf> { unsafe { &raw mut (*this).entry } }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct msgbuf {
    pub bufs: tailq_head<ibuf>,
    pub queued: u32,
    pub fd: c_int,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ibuf_read {
    pub buf: [c_uchar; 65535usize],
    pub rptr: *mut c_uchar,
    pub wpos: usize,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct imsgbuf {
    pub fds: tailq_head<imsg_fd>,
    pub r: ibuf_read,
    pub w: msgbuf,
    pub fd: c_int,
    pub pid: pid_t,
}

pub const IMSG_HEADER_SIZE: usize = size_of::<imsg_hdr>();
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct imsg_hdr {
    pub type_: u32,
    pub len: u16,
    pub flags: u16,
    pub peerid: u32,
    pub pid: u32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct imsg {
    pub hdr: imsg_hdr,
    pub fd: c_int,
    pub data: *mut c_void,
    pub buf: *mut ibuf,
}

impl crate::queue::Entry<imsg_fd> for imsg_fd {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<imsg_fd> { unsafe { &raw mut (*this).entry } }
}

// end imsg.h
// begin imsg.c

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct imsg_fd {
    entry: tailq_entry<imsg_fd>,
    fd: i32,
}

static mut imsg_fd_overhead: i32 = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_init(imsgbuf: *mut imsgbuf, fd: c_int) {
    unsafe {
        msgbuf_init(&raw mut (*imsgbuf).w);
        (*imsgbuf).r = core::mem::zeroed();
        (*imsgbuf).fd = fd;
        (*imsgbuf).w.fd = fd;
        (*imsgbuf).pid = libc::getpid();
        tailq_init(&raw mut (*imsgbuf).fds);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_read(imsgbuf: *mut imsgbuf) -> isize {
    const BUFSIZE: usize = unsafe { CMSG_SPACE(size_of::<c_int>() as u32) } as usize;
    union cmsgbuf {
        hdr: cmsghdr,
        buf: [c_uchar; BUFSIZE],
    };

    unsafe {
        let mut n: isize = -1;
        let fd: i32;

        let mut msg: msghdr = core::mem::zeroed();
        let mut cmsgbuf: cmsgbuf = core::mem::zeroed();

        let mut iov: iovec = iovec {
            iov_base: (*imsgbuf).r.buf.as_mut_ptr().add((*imsgbuf).r.wpos) as *mut c_void,
            iov_len: BUFSIZE - (*imsgbuf).r.wpos,
        };
        msg.msg_iov = &raw mut iov;
        msg.msg_iovlen = 1;
        msg.msg_control = &raw mut cmsgbuf.buf as *mut c_void;
        msg.msg_controllen = BUFSIZE;

        let mut ifd: *mut imsg_fd = calloc(1, size_of::<imsg_fd>()) as *mut imsg_fd;
        if ifd.is_null() {
            return -1;
        }

        // this extra labeled block isn't necessary, but makes the breaks more semantic
        // goto fail => break 'fail
        // goto again => continue 'again
        'fail: {
            'again: loop {
                if getdtablecount()
                    + imsg_fd_overhead
                    + ((CMSG_SPACE(size_of::<libc::c_int>() as u32) - CMSG_SPACE(0)) as i32 / size_of::<c_int>() as i32)
                    >= getdtablesize()
                {
                    *__errno_location() = EAGAIN;
                    free(ifd as *mut c_void);
                    return -1;
                }

                n = libc::recvmsg((*imsgbuf).fd, &raw mut msg, 0);
                if n == -1 {
                    if (*__errno_location() == EINTR) {
                        continue 'again;
                    }
                    break 'fail;
                }

                (*imsgbuf).r.wpos += n as usize;

                // really?
                let mut cmsg: *mut cmsghdr = CMSG_FIRSTHDR(&raw const msg);
                while !cmsg.is_null() {
                    if (*cmsg).cmsg_level == SOL_SOCKET && (*cmsg).cmsg_type == SCM_RIGHTS {
                        let mut i: c_int;

                        let mut j: c_int = (((cmsg as *mut c_char).add((*cmsg).cmsg_len).addr()
                            - CMSG_DATA(cmsg).addr())
                            / size_of::<c_int>()) as i32;
                        for i in 0..j {
                            let fd = *(CMSG_DATA(cmsg) as *mut c_int).add(i as usize);
                            if !ifd.is_null() {
                                (*ifd).fd = fd;
                                tailq_insert_tail(&raw mut (*imsgbuf).fds, ifd);
                                ifd = null_mut();
                            } else {
                                close(fd);
                            }
                        }
                    }
                    cmsg = CMSG_NXTHDR(&raw const msg, cmsg);
                }

                break; // no looping on success
            }
        }

        // fail:
        free(ifd as *mut c_void);
        n
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get(imsgbuf: *mut imsgbuf, imsg: *mut imsg) -> isize {
    unsafe {
        let mut m = MaybeUninit::<imsg>::uninit();
        let m = m.as_mut_ptr();
        let av: usize = (*imsgbuf).r.wpos;

        if IMSG_HEADER_SIZE > av {
            return 0;
        }

        memcpy(
            &raw mut (*m).hdr as *mut c_void,
            (*imsgbuf).r.buf.as_ptr() as *const c_void,
            size_of::<imsg_hdr>(),
        );
        if ((*m).hdr.len as usize) < IMSG_HEADER_SIZE || ((*m).hdr.len as usize) > MAX_IMSGSIZE {
            *__errno_location() = ERANGE;
            return -1;
        }
        if ((*m).hdr.len as usize) > av {
            return 0;
        }

        (*m).fd = -1;
        (*m).buf = null_mut();
        (*m).data = null_mut();

        let datalen = (*m).hdr.len as usize - IMSG_HEADER_SIZE;
        (*imsgbuf).r.rptr = (*imsgbuf).r.buf.as_mut_ptr().add(IMSG_HEADER_SIZE);
        if datalen != 0 {
            (*m).buf = ibuf_open(datalen);
            if (*m).buf.is_null() {
                return -1;
            }
            if ibuf_add((*m).buf, (*imsgbuf).r.rptr as *mut c_void, datalen) == -1 {
                /* this should never fail */
                ibuf_free((*m).buf);
                return -1;
            }
            (*m).data = ibuf_data((*m).buf);
        }

        if (*m).hdr.flags & IMSGF_HASFD != 0 {
            (*m).fd = imsg_dequeue_fd(imsgbuf);
        }

        if ((*m).hdr.len as usize) < av {
            let left = av - (*m).hdr.len as usize;
            memmove(
                &raw mut (*imsgbuf).r.buf as *mut c_void,
                (*imsgbuf).r.buf.as_ptr().add((*m).hdr.len as usize) as *const c_void,
                left,
            );
            (*imsgbuf).r.wpos = left;
        } else {
            (*imsgbuf).r.wpos = 0;
        }

        core::ptr::copy_nonoverlapping(m, imsg, 1);

        (datalen + IMSG_HEADER_SIZE) as isize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_ibuf(imsg: *mut imsg, ibuf: *mut ibuf) -> c_int {
    unsafe {
        if (*imsg).buf.is_null() {
            *__errno_location() = EBADMSG;
            return -1;
        }
        ibuf_get_ibuf((*imsg).buf, ibuf_size((*imsg).buf), ibuf)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_data(imsg: *mut imsg, data: *mut c_void, len: usize) -> i32 {
    unsafe {
        if len == 0 {
            *__errno_location() = EINVAL;
            return -1;
        }
        if (*imsg).buf.is_null() || ibuf_size((*imsg).buf) != len {
            *__errno_location() = EBADMSG;
            return -1;
        }
        ibuf_get((*imsg).buf, data, len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_fd(imsg: *mut imsg) -> i32 { unsafe { std::ptr::replace(&raw mut (*imsg).fd, -1) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_id(imsg: *const imsg) -> u32 { unsafe { (*imsg).hdr.peerid } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_len(imsg: *const imsg) -> usize {
    unsafe {
        if (*imsg).buf.is_null() {
            return 0;
        }
        ibuf_size((*imsg).buf)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_pid(imsg: *const imsg) -> pid_t { unsafe { (*imsg).hdr.pid as pid_t } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_get_type(imsg: *const imsg) -> u32 { unsafe { (*imsg).hdr.type_ } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_compose(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    fd: c_int,
    data: *const c_void,
    datalen: usize,
) -> i32 {
    unsafe {
        let wbuf = imsg_create(imsgbuf, type_, id, pid, datalen);
        if wbuf.is_null() {
            return -1;
        }

        if imsg_add(wbuf, data, datalen) == -1 {
            return -1;
        }

        ibuf_fd_set(wbuf, fd);
        imsg_close(imsgbuf, wbuf);

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_composev(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    fd: c_int,
    iov: *const iovec,
    iovcnt: c_int,
) -> c_int {
    unsafe {
        let mut datalen: usize = 0;

        for i in 0..iovcnt {
            datalen += (*iov.add(i as usize)).iov_len;
        }

        let wbuf = imsg_create(imsgbuf, type_, id, pid, datalen);
        if wbuf.is_null() {
            return -1;
        }

        for i in 0..iovcnt {
            if imsg_add(wbuf, (*iov.add(i as usize)).iov_base, (*iov.add(i as usize)).iov_len) == -1 {
                return -1;
            }
        }

        ibuf_fd_set(wbuf, fd);
        imsg_close(imsgbuf, wbuf);

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_compose_ibuf(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    buf: *mut ibuf,
) -> c_int {
    unsafe {
        let hdrbuf: *mut ibuf = null_mut();
        let mut hdr: imsg_hdr = std::mem::zeroed();

        let fail = || {
            // TODO is this equivalent to the goto fail;
            // TODO is the old value of the pointer captured?
            let save_errno = *__errno_location();
            ibuf_free(buf);
            ibuf_free(hdrbuf);
            *__errno_location() = save_errno;
            -1
        };

        if ibuf_size(buf) + IMSG_HEADER_SIZE > MAX_IMSGSIZE as usize {
            *__errno_location() = ERANGE;
            return fail();
        }

        hdr.type_ = type_;
        hdr.len = (ibuf_size(buf) + IMSG_HEADER_SIZE) as u16;
        hdr.flags = 0;
        hdr.peerid = id;

        hdr.pid = pid as u32;
        if hdr.pid == 0 {
            hdr.pid = (*imsgbuf).pid as u32;
        }

        let hdrbuf = ibuf_open(IMSG_HEADER_SIZE);
        if hdrbuf.is_null() {
            return fail();
        }
        if imsg_add(hdrbuf, &raw mut hdr as *mut c_void, size_of::<imsg_hdr>()) == -1 {
            return fail();
        }

        ibuf_close(&raw mut (*imsgbuf).w, hdrbuf);
        ibuf_close(&raw mut (*imsgbuf).w, buf);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_forward(imsgbuf: *mut imsgbuf, msg: *mut imsg) -> c_int {
    unsafe {
        let mut len = 0;

        if (*msg).fd != -1 {
            close((*msg).fd);
            (*msg).fd = -1;
        }

        if !(*msg).buf.is_null() {
            ibuf_rewind((*msg).buf);
            len = ibuf_size((*msg).buf);
        }

        let wbuf = imsg_create(
            imsgbuf,
            (*msg).hdr.type_,
            (*msg).hdr.peerid,
            (*msg).hdr.pid as pid_t,
            len,
        );
        if wbuf.is_null() {
            return -1;
        }

        if !(*msg).buf.is_null() {
            if ibuf_add_buf(wbuf, (*msg).buf) == -1 {
                ibuf_free(wbuf);
                return -1;
            }
        }

        imsg_close(imsgbuf, wbuf);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_create(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    mut datalen: usize,
) -> *mut ibuf {
    unsafe {
        let mut hdr: imsg_hdr = std::mem::zeroed();

        datalen += IMSG_HEADER_SIZE;
        if datalen > MAX_IMSGSIZE {
            *__errno_location() = ERANGE;
            return null_mut();
        }

        hdr.type_ = type_;
        hdr.flags = 0;
        hdr.peerid = id;

        hdr.pid = pid as _;
        if hdr.pid == 0 {
            hdr.pid = (*imsgbuf).pid as _;
        }

        let wbuf = ibuf_dynamic(datalen, MAX_IMSGSIZE);
        if wbuf.is_null() {
            return null_mut();
        }
        if imsg_add(wbuf, &raw mut hdr as *mut c_void, size_of::<imsg_hdr>()) == -1 {
            return null_mut();
        }

        wbuf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_add(msg: *mut ibuf, data: *const c_void, datalen: usize) -> i32 {
    unsafe {
        if datalen != 0 {
            if ibuf_add(msg, data, datalen) == -1 {
                ibuf_free(msg);
                return -1;
            }
        }
        datalen as i32
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_close(imsgbuf: *mut imsgbuf, msg: *mut ibuf) {
    unsafe {
        let hdr: *mut imsg_hdr = (*msg).buf as *mut imsg_hdr;

        (*hdr).flags &= !IMSGF_HASFD;
        if ibuf_fd_avail(msg) != 0 {
            (*hdr).flags |= IMSGF_HASFD;
        }
        (*hdr).len = ibuf_size(msg) as u16;

        ibuf_close(&raw mut (*imsgbuf).w, msg)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_free(imsg: *mut imsg) { unsafe { ibuf_free((*imsg).buf) } }

#[unsafe(no_mangle)]
unsafe extern "C" fn imsg_dequeue_fd(imsgbuf: *mut imsgbuf) -> i32 {
    unsafe {
        let ifd = tailq_first(&raw mut (*imsgbuf).fds);

        if ifd.is_null() {
            return -1;
        }

        let fd = (*ifd).fd;
        tailq_remove(&raw mut (*imsgbuf).fds, ifd);
        free(ifd as *mut c_void);

        fd
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_flush(imsgbuf: *mut imsgbuf) -> c_int {
    unsafe {
        while (*imsgbuf).w.queued != 0 {
            if msgbuf_write(&raw mut (*imsgbuf).w) <= 0 {
                return -1;
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn imsg_clear(imsgbuf: *mut imsgbuf) {
    unsafe {
        msgbuf_clear(&raw mut (*imsgbuf).w);

        loop {
            let fd = imsg_dequeue_fd(imsgbuf);

            if fd == -1 {
                break;
            }

            close(fd);
        }
    }
}
