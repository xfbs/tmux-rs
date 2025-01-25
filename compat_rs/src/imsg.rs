use core::ffi::{c_int, c_uchar, c_void};
use std::{mem::MaybeUninit, ptr::null_mut};

use libc::{c_char, cmsghdr, iovec, msghdr, pid_t};

use crate::getdtablecount::getdtablecount;
use crate::imsg_buffer::{
    ibuf_add, ibuf_add_buf, ibuf_close, ibuf_data, ibuf_dynamic, ibuf_fd_avail, ibuf_fd_set, ibuf_free, ibuf_get,
    ibuf_get_ibuf, ibuf_open, ibuf_rewind, ibuf_size, msgbuf_clear, msgbuf_init, msgbuf_write,
};
use crate::queue::{tailq_entry, tailq_first, tailq_head, tailq_init, tailq_insert_tail, tailq_remove, Entry};

pub const MAX_IMSGSIZE: u32 = 16384;
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
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<ibuf> {
        unsafe { &raw mut (*this).entry }
    }
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

const IMSG_HEADER_SIZE: usize = size_of::<imsg_hdr>();
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

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct imsg_fd {
    entry: tailq_entry<imsg_fd>,
    fd: i32,
}

impl crate::queue::Entry<imsg_fd> for imsg_fd {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<imsg_fd> {
        unsafe { &raw mut (*this).entry }
    }
}

static imsg_fd_overhead: i32 = 0;

#[no_mangle]
pub unsafe extern "C" fn imsg_init(imsgbuf: *mut imsgbuf, fd: c_int) {
    msgbuf_init(&raw mut (*imsgbuf).w);
    (*imsgbuf).r = core::mem::zeroed();
    (*imsgbuf).fd = fd;
    (*imsgbuf).w.fd = fd;
    (*imsgbuf).pid = libc::getpid();
    tailq_init(&raw mut (*imsgbuf).fds);
}

#[no_mangle]
pub unsafe extern "C" fn imsg_read(imsgbuf: *mut imsgbuf) -> isize {
    union cmsgbuf {
        hdr: cmsghdr,
        buf: [c_uchar; unsafe { libc::CMSG_SPACE(size_of::<c_int>() as u32) } as usize],
    };
    let n: isize = -1;
    let fd: i32;

    let mut msg: msghdr = core::mem::zeroed();
    let mut cmsgbuf: cmsgbuf = core::mem::zeroed();

    let mut iov: iovec = iovec {
        iov_base: (*imsgbuf).r.buf.as_mut_ptr().add((*imsgbuf).r.wpos) as *mut c_void,
        iov_len: size_of_val(&(*imsgbuf).r.buf) - (*imsgbuf).r.wpos,
    };
    msg.msg_iov = &raw mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsgbuf.buf.as_mut_ptr() as *mut c_void;
    msg.msg_controllen = size_of_val(&cmsgbuf.buf);

    let mut ifd: *mut imsg_fd = libc::calloc(1, size_of::<imsg_fd>()) as _;
    if ifd.is_null() {
        return -1;
    }

    loop {
        if (getdtablecount()
            + imsg_fd_overhead
            + ((libc::CMSG_SPACE(size_of::<libc::c_int>() as u32) - libc::CMSG_SPACE(0)) as i32
                / size_of::<c_int>() as i32)
            >= libc::getdtablesize())
        {
            *libc::__errno_location() = libc::EAGAIN;
            libc::free(ifd as *mut c_void);
            return -1;
        }

        let n = libc::recvmsg((*imsgbuf).fd, &raw mut msg, 0);
        if n == -1 {
            if (*libc::__errno_location() == libc::EINTR) {
                continue;
            }
            break;
        }

        (*imsgbuf).r.wpos += n as usize;

        let mut cmsg: *mut cmsghdr = libc::CMSG_FIRSTHDR(&raw mut msg);
        while !cmsg.is_null() {
            if (*cmsg).cmsg_level == libc::SOL_SOCKET && (*cmsg).cmsg_type == libc::SCM_RIGHTS {
                let mut i: c_int;

                let mut j: c_int = (((cmsg as *mut c_char).add((*cmsg).cmsg_len) as isize
                    - libc::CMSG_DATA(cmsg) as isize)
                    / size_of::<c_int>() as isize) as i32;
                for i in 0..j {
                    let fd = *(libc::CMSG_DATA(cmsg) as *mut c_int).add(i as usize);
                    if !ifd.is_null() {
                        (*ifd).fd = fd;
                        tailq_insert_tail::<_, ()>(&raw mut (*imsgbuf).fds, ifd);
                        ifd = null_mut();
                    } else {
                        libc::close(fd);
                    }
                }
                cmsg = libc::CMSG_NXTHDR(&msg, cmsg);
            }
        }
    }

    libc::free(ifd as _);
    n
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get(imsgbuf: *mut imsgbuf, imsg: *mut imsg) -> isize {
    let mut m: imsg = std::mem::zeroed();
    let av: usize = (*imsgbuf).r.wpos;

    if IMSG_HEADER_SIZE > av {
        return 0;
    }

    libc::memcpy(
        &raw mut m.hdr as *mut c_void,
        (*imsgbuf).r.buf.as_ptr() as *const c_void,
        size_of::<imsg_hdr>(),
    );
    if (m.hdr.len as usize) < IMSG_HEADER_SIZE || (m.hdr.len as u32) > MAX_IMSGSIZE {
        *libc::__errno_location() = libc::ERANGE;
        return -1;
    }
    if (m.hdr.len as usize) > av {
        return 0;
    }

    m.fd = -1;
    m.buf = null_mut();
    m.data = null_mut();

    let datalen = m.hdr.len as usize - IMSG_HEADER_SIZE;
    (*imsgbuf).r.rptr = (*imsgbuf).r.buf.as_mut_ptr().add(IMSG_HEADER_SIZE);
    if datalen != 0 {
        m.buf = ibuf_open(datalen);
        if m.buf.is_null() {
            return -1;
        }
        if ibuf_add(m.buf, (*imsgbuf).r.rptr as *mut c_void, datalen) == -1 {
            /* this should never fail */
            ibuf_free(m.buf);
            return -1;
        }
        m.data = ibuf_data(m.buf);
    }

    if m.hdr.flags & IMSGF_HASFD != 0 {
        m.fd = imsg_dequeue_fd(imsgbuf);
    }

    if (m.hdr.len as usize) < av {
        let left = av - m.hdr.len as usize;
        libc::memmove(
            &raw mut (*imsgbuf).r.buf as *mut c_void,
            (*imsgbuf).r.buf.as_ptr().add(m.hdr.len as usize) as *const c_void,
            left,
        );
        (*imsgbuf).r.wpos = left;
    } else {
        (*imsgbuf).r.wpos = 0;
    }

    *imsg = m;

    (datalen + IMSG_HEADER_SIZE) as isize
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_ibuf(imsg: *mut imsg, ibuf: *mut ibuf) -> c_int {
    if (*imsg).buf.is_null() {
        *libc::__errno_location() = libc::EBADMSG;
        return -1;
    }
    ibuf_get_ibuf((*imsg).buf, ibuf_size((*imsg).buf), ibuf)
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_data(imsg: *mut imsg, data: *mut c_void, len: usize) -> c_int {
    if len == 0 {
        *libc::__errno_location() = libc::EINVAL;
        return -1;
    }
    if (*imsg).buf.is_null() || ibuf_size((*imsg).buf) != len {
        *libc::__errno_location() = libc::EBADMSG;
        return -1;
    }
    ibuf_get((*imsg).buf, data, len)
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_fd(imsg: *mut imsg) -> c_int {
    let fd = (*imsg).fd;

    (*imsg).fd = -1;
    fd
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_id(imsg: *mut imsg) -> u32 {
    (*imsg).hdr.peerid
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_len(imsg: *mut imsg) -> usize {
    if (*imsg).buf.is_null() {
        return 0;
    }
    ibuf_size((*imsg).buf)
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_pid(imsg: *mut imsg) -> pid_t {
    (*imsg).hdr.pid as pid_t
}

#[no_mangle]
pub unsafe extern "C" fn imsg_get_type(imsg: *mut imsg) -> u32 {
    (*imsg).hdr.type_
}

#[no_mangle]
pub unsafe extern "C" fn imsg_compose(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    fd: c_int,
    data: *const c_void,
    datalen: usize,
) -> c_int {
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

#[no_mangle]
pub unsafe extern "C" fn imsg_composev(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    fd: c_int,
    iov: *const iovec,
    iovcnt: c_int,
) -> c_int {
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

#[no_mangle]
pub unsafe extern "C" fn imsg_compose_ibuf(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    buf: *mut ibuf,
) -> c_int {
    let hdrbuf: *mut ibuf = null_mut();
    let mut hdr: imsg_hdr = std::mem::zeroed();

    let fail = || {
        // TODO is this equivalent to the goto fail;
        // TODO is the old value of the pointer captured?
        let save_errno = *libc::__errno_location();
        ibuf_free(buf);
        ibuf_free(hdrbuf);
        *libc::__errno_location() = save_errno;
        -1
    };

    if ibuf_size(buf) + IMSG_HEADER_SIZE > MAX_IMSGSIZE as usize {
        *libc::__errno_location() = libc::ERANGE;
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
    return 1;
}

#[no_mangle]
pub unsafe extern "C" fn imsg_forward(imsgbuf: *mut imsgbuf, msg: *mut imsg) -> c_int {
    let mut len = 0;

    if (*msg).fd != -1 {
        libc::close((*msg).fd);
        (*msg).fd = -1;
    }

    if !(*msg).buf.is_null() {
        ibuf_rewind((*msg).buf);
        len = ibuf_size((*msg).buf);
    }

    let wbuf = imsg_create(imsgbuf, (*msg).hdr.type_, (*msg).hdr.peerid, (*msg).hdr.pid as _, len);
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

#[no_mangle]
pub unsafe extern "C" fn imsg_create(
    imsgbuf: *mut imsgbuf,
    type_: u32,
    id: u32,
    pid: pid_t,
    mut datalen: usize,
) -> *mut ibuf {
    let mut hdr: imsg_hdr = std::mem::zeroed();

    datalen += IMSG_HEADER_SIZE;
    if datalen > MAX_IMSGSIZE as usize {
        *libc::__errno_location() = libc::ERANGE;
        return null_mut();
    }

    hdr.type_ = type_;
    hdr.flags = 0;
    hdr.peerid = id;

    hdr.pid = pid as _;
    if hdr.pid == 0 {
        hdr.pid = (*imsgbuf).pid as _;
    }

    let wbuf = ibuf_dynamic(datalen, MAX_IMSGSIZE as usize);
    if wbuf.is_null() {
        return null_mut();
    }
    if imsg_add(wbuf, &raw mut hdr as *mut c_void, size_of::<imsg_hdr>()) == -1 {
        return null_mut();
    }

    wbuf
}

#[no_mangle]
pub unsafe extern "C" fn imsg_add(msg: *mut ibuf, data: *const c_void, datalen: usize) -> c_int {
    if datalen != 0 {
        if ibuf_add(msg, data, datalen) == -1 {
            ibuf_free(msg);
            return -1;
        }
    }
    datalen as _
}
#[no_mangle]
pub unsafe extern "C" fn imsg_close(imsgbuf: *mut imsgbuf, msg: *mut ibuf) {
    let hdr: *mut imsg_hdr = (*msg).buf as *mut imsg_hdr;

    (*hdr).flags &= !IMSGF_HASFD;
    if ibuf_fd_avail(msg) != 0 {
        (*hdr).flags |= !IMSGF_HASFD;
    }
    (*hdr).len = ibuf_size(msg).try_into().expect("buf size too large");

    ibuf_close(&raw mut (*imsgbuf).w, msg)
}

#[no_mangle]
pub unsafe extern "C" fn imsg_free(imsg: *mut imsg) {
    ibuf_free((*imsg).buf)
}

#[no_mangle]
unsafe extern "C" fn imsg_dequeue_fd(imsgbuf: *mut imsgbuf) -> i32 {
    let ifd = tailq_first(&raw mut (*imsgbuf).fds);

    if ifd.is_null() {
        return -1;
    }

    let fd = (*ifd).fd;
    tailq_remove(&raw mut (*imsgbuf).fds, ifd);
    libc::free(ifd as *mut c_void);

    fd
}

#[no_mangle]
pub unsafe extern "C" fn imsg_flush(imsgbuf: *mut imsgbuf) -> c_int {
    while (*imsgbuf).w.queued != 0 {
        if msgbuf_write(&raw mut (*imsgbuf).w) <= 0 {
            return -1;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn imsg_clear(imsgbuf: *mut imsgbuf) {
    msgbuf_clear(&raw mut (*imsgbuf).w);

    loop {
        let fd = imsg_dequeue_fd(imsgbuf);

        if fd == -1 {
            break;
        }

        libc::close(fd);
    }
}
