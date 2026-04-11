// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
use crate::compat::imsg::{IMSG_HEADER_SIZE, MAX_IMSGSIZE};
use crate::errno;
use crate::libc::{
    BUFSIZ, E2BIG, EBADF, EINVAL, EIO, ENOMEM, O_APPEND, O_CREAT, O_NONBLOCK, O_RDONLY, O_WRONLY,
    STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO, close, dup, fclose, ferror, fopen, fread, fwrite,
    memcpy, open,
};
use crate::*;

pub static FILE_NEXT_STREAM: atomic::AtomicI32 = atomic::AtomicI32::new(3);

pub unsafe fn file_get_path(c: *mut client, file: *const u8) -> NonNull<u8> {
    unsafe {
        if *file == b'/' {
            xstrdup(file)
        } else {
            let base = server_client_get_cwd(c, null_mut());
            NonNull::new(format_nul!(
                "{}/{}",
                base.display(),
                _s(file)
            ))
            .unwrap()
        }
    }
}

pub unsafe fn file_create_with_peer(
    peer: *mut tmuxpeer,
    files: *mut client_files,
    stream: c_int,
    cb: client_file_cb,
    cbdata: *mut c_void,
) -> *mut client_file {
    unsafe {
        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        let cf = Box::new(client_file {
            c: null_mut(),
            references: 1,
            stream,
            buffer,
            cb,
            data: cbdata,
            peer,
            tree: files,
            path: null_mut(),
            event: null_mut(),
            fd: 0,
            error: 0,
            closed: 0,
        });
        (*files).insert(stream, cf);
        let cf_ptr = &mut **(*files).get_mut(&stream).unwrap() as *mut client_file;
        cf_ptr
    }
}

pub unsafe fn file_create_with_client(
    mut c: *mut client,
    stream: c_int,
    cb: client_file_cb,
    cbdata: *mut c_void,
) -> *mut client_file {
    unsafe {
        if !c.is_null() && (*c).flags.intersects(client_flag::ATTACHED) {
            c = null_mut();
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx("out of memory");
        }

        let peer = if !c.is_null() { (*c).peer } else { null_mut() };
        let tree = if !c.is_null() { &raw mut (*c).files } else { null_mut() };

        let cf = Box::new(client_file {
            c,
            references: 1,
            stream,
            buffer,
            cb,
            data: cbdata,
            peer,
            tree,
            path: null_mut(),
            event: null_mut(),
            fd: 0,
            error: 0,
            closed: 0,
        });

        let cf_ptr: *mut client_file;
        if !c.is_null() {
            (*c).files.insert(stream, cf);
            cf_ptr = &mut **(*c).files.get_mut(&stream).unwrap() as *mut client_file;
            (*c).references += 1;
        } else {
            // No tree to insert into; leak the Box and return raw pointer.
            // The caller must ensure cleanup.
            cf_ptr = Box::into_raw(cf);
        }

        cf_ptr
    }
}

pub unsafe fn file_free(cf: *mut client_file) {
    unsafe {
        (*cf).references -= 1;
        if (*cf).references != 0 {
            return;
        }

        evbuffer_free((*cf).buffer);
        free_((*cf).path);

        let c = (*cf).c;
        let tree = (*cf).tree;
        let stream = (*cf).stream;

        if !tree.is_null() {
            // Removing from the BTreeMap drops the Box and frees cf
            (*tree).remove(&stream);
        } else {
            // Not in a tree, was allocated with Box::into_raw
            drop(Box::from_raw(cf));
        }
        if !c.is_null() {
            server_client_unref(c);
        }
    }
}

unsafe fn file_fire_done_cb(cf: *mut client_file) {
    unsafe {
        let c: *mut client = (*cf).c;

        if let Some(cb) = (*cf).cb
            && ((*cf).closed != 0 || c.is_null() || !(*c).flags.intersects(client_flag::DEAD))
        {
            cb(c, (*cf).path, (*cf).error, 1, (*cf).buffer, (*cf).data);
        }
        file_free(cf);
    }
}

pub unsafe fn file_fire_done(cf: *mut client_file) {
    defer(Box::new(move || unsafe { file_fire_done_cb(cf) }));
}

pub unsafe fn file_fire_read(cf: *mut client_file) {
    unsafe {
        if let Some(cb) = (*cf).cb {
            cb(
                (*cf).c,
                (*cf).path,
                (*cf).error,
                0,
                (*cf).buffer,
                (*cf).data,
            );
        }
    }
}

pub unsafe fn file_can_print(c: *mut client) -> bool {
    unsafe {
        !(c.is_null()
            || (*c).flags.intersects(client_flag::ATTACHED)
            || (*c).flags.intersects(client_flag::CONTROL))
    }
}

macro_rules! file_print {
   ($client:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::file::file_vprint($client, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use file_print;

pub unsafe fn file_vprint(c: *mut client, args: std::fmt::Arguments) {
    unsafe {
        if !file_can_print(c) {
            return;
        }

        if let Some(cf) = (*c).files.get_mut(&1) {
            let cf = &mut **cf as *mut client_file;
            evbuffer_add_vprintf((*cf).buffer, args);
            file_push(cf);
        } else {
            let cf = file_create_with_client(c, 1, None, null_mut());
            (*cf).path = xstrdup(c!("-")).as_ptr();

            // TODO
            evbuffer_add_vprintf((*cf).buffer, args);

            let mut msg: msg_write_open = zeroed();
            msg.stream = 1;
            msg.fd = STDOUT_FILENO;
            msg.flags = 0;
            proc_send(
                (*c).peer,
                msgtype::MSG_WRITE_OPEN,
                -1,
                &raw mut msg as _,
                size_of::<msg_write_open>(),
            );
        }
    }
}

pub unsafe fn file_print_buffer(c: *mut client, data: *mut c_void, size: usize) {
    unsafe {
        if !file_can_print(c) {
            return;
        }

        if let Some(cf) = (*c).files.get_mut(&1) {
            let cf = &mut **cf as *mut client_file;
            evbuffer_add((*cf).buffer, data, size);
            file_push(cf);
        } else {
            let cf = file_create_with_client(c, 1, None, null_mut());
            (*cf).path = xstrdup(c!("-")).as_ptr();

            evbuffer_add((*cf).buffer, data, size);

            let mut msg: msg_write_open = zeroed();
            msg.stream = 1;
            msg.fd = STDOUT_FILENO;
            msg.flags = 0;
            proc_send(
                (*c).peer,
                msgtype::MSG_WRITE_OPEN,
                -1,
                &raw mut msg as _,
                size_of::<msg_write_open>(),
            );
        }
    }
}

macro_rules! file_error {
   ($client:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::file::file_error_($client, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use file_error;
pub unsafe fn file_error_(c: *mut client, args: std::fmt::Arguments) {
    unsafe {
        if !file_can_print(c) {
            return;
        }

        if let Some(cf) = (*c).files.get_mut(&2) {
            let cf = &mut **cf as *mut client_file;
            evbuffer_add_vprintf((*cf).buffer, args);
            file_push(cf);
        } else {
            let cf = file_create_with_client(c, 2, None, null_mut());
            (*cf).path = xstrdup(c!("-")).as_ptr();

            evbuffer_add_vprintf((*cf).buffer, args);

            let mut msg: msg_write_open = zeroed();
            msg.stream = 2;
            msg.fd = STDERR_FILENO;
            msg.flags = 0;
            proc_send(
                (*c).peer,
                msgtype::MSG_WRITE_OPEN,
                -1,
                &raw mut msg as _,
                size_of::<msg_write_open>(),
            );
        }
    }
}

pub unsafe fn file_write(
    c: *mut client,
    path: *const u8,
    flags: c_int,
    bdata: *const c_void,
    bsize: usize,
    cb: client_file_cb,
    cbdata: *mut c_void,
) {
    unsafe {
        let cf: *mut client_file;
        let msg: *mut msg_write_open;
        let msglen: usize;
        let mut fd = -1;
        let stream: u32 = FILE_NEXT_STREAM.fetch_add(1, atomic::Ordering::Relaxed) as u32;
        let f: *mut FILE;
        let mode: *const u8;

        'done: {
            'skip: {
                if streq_(path, "-") {
                    cf = file_create_with_client(c, stream as i32, cb, cbdata);
                    (*cf).path = xstrdup_(c"-").as_ptr();

                    fd = STDOUT_FILENO;
                    if c.is_null()
                        || ((*c).flags.intersects(client_flag::ATTACHED))
                        || ((*c).flags.intersects(client_flag::CONTROL))
                    {
                        (*cf).error = EBADF;
                        break 'done;
                    }
                    break 'skip;
                }

                cf = file_create_with_client(c, stream as i32, cb, cbdata);
                (*cf).path = file_get_path(c, path).as_ptr();

                if c.is_null() || (*c).flags.intersects(client_flag::ATTACHED) {
                    if flags & O_APPEND != 0 {
                        mode = c!("ab");
                    } else {
                        mode = c!("wb");
                    }
                    f = fopen((*cf).path, mode);
                    if f.is_null() {
                        (*cf).error = errno!();
                        break 'done;
                    }
                    if fwrite(bdata, 1, bsize, f) != bsize {
                        fclose(f);
                        (*cf).error = EIO;
                        break 'done;
                    }
                    fclose(f);
                    break 'done;
                }
            }

            // skip:
            evbuffer_add((*cf).buffer, bdata, bsize);

            msglen = strlen((*cf).path) + 1 + size_of::<msg_write_open>();
            if msglen > MAX_IMSGSIZE - IMSG_HEADER_SIZE {
                (*cf).error = E2BIG;
                break 'done;
            }
            msg = xmalloc(msglen).as_ptr().cast();
            (*msg).stream = (*cf).stream;
            (*msg).fd = fd;
            (*msg).flags = flags;
            memcpy(
                msg.add(1).cast(),
                (*cf).path.cast(),
                msglen - size_of::<msg_write_open>(),
            );
            if proc_send((*cf).peer, msgtype::MSG_WRITE_OPEN, -1, msg.cast(), msglen) != 0 {
                free_(msg);
                (*cf).error = EINVAL;
                break 'done;
            }
            free_(msg);
            return;
        }

        // done:
        file_fire_done(cf);
    }
}

pub unsafe fn file_read(
    c: *mut client,
    path: *const u8,
    cb: client_file_cb,
    cbdata: *mut c_void,
) -> *mut client_file {
    unsafe {
        let cf;
        let msg: *mut msg_read_open;
        let msglen: usize;
        let mut fd: i32 = -1;
        let stream: u32 = FILE_NEXT_STREAM.fetch_add(1, atomic::Ordering::Relaxed) as u32;
        let f: *mut FILE;
        let mut size: usize;
        let mut buffer = MaybeUninit::<[u8; BUFSIZ as usize]>::uninit();
        'done: {
            'skip: {
                if streq_(path, "-") {
                    cf = file_create_with_client(c, stream as i32, cb, cbdata);
                    (*cf).path = xstrdup_(c"-").as_ptr();

                    fd = STDIN_FILENO;
                    if c.is_null()
                        || ((*c).flags.intersects(client_flag::ATTACHED))
                        || ((*c).flags.intersects(client_flag::CONTROL))
                    {
                        (*cf).error = EBADF;
                        break 'done;
                    }
                    break 'skip;
                }

                cf = file_create_with_client(c, stream as i32, cb, cbdata);
                (*cf).path = file_get_path(c, path).as_ptr();

                if c.is_null() || (*c).flags.intersects(client_flag::ATTACHED) {
                    f = fopen((*cf).path, c!("rb"));
                    if f.is_null() {
                        (*cf).error = errno!();
                        break 'done;
                    }
                    loop {
                        size = fread(buffer.as_mut_ptr().cast(), 1, BUFSIZ as usize, f);
                        if evbuffer_add((*cf).buffer, buffer.as_ptr().cast(), size) != 0 {
                            (*cf).error = ENOMEM;
                            break 'done;
                        }
                        if size != BUFSIZ as usize {
                            break;
                        }
                    }
                    if ferror(f) != 0 {
                        (*cf).error = EIO;
                        break 'done;
                    }
                    fclose(f);
                    break 'done;
                }
            }

            // skip:
            msglen = strlen((*cf).path) + 1 + size_of::<msg_read_open>();
            if msglen > MAX_IMSGSIZE - IMSG_HEADER_SIZE {
                (*cf).error = E2BIG;
                break 'done;
            }
            msg = xmalloc(msglen).as_ptr().cast();
            (*msg).stream = (*cf).stream;
            (*msg).fd = fd;
            memcpy(
                msg.add(1).cast(),
                (*cf).path.cast(),
                msglen - size_of::<msg_read_open>(),
            );
            if proc_send((*cf).peer, msgtype::MSG_READ_OPEN, -1, msg.cast(), msglen) != 0 {
                free_(msg);
                (*cf).error = EINVAL;
                break 'done;
            }
            free_(msg);
            return cf;
        }

        // done:
        file_fire_done(cf);
        null_mut()
    }
}

pub unsafe fn file_cancel(cf: *mut client_file) {
    unsafe {
        log_debug!("read cancel file {}", (*cf).stream);

        if (*cf).closed != 0 {
            return;
        }
        (*cf).closed = 1;

        let msg: msg_read_cancel = msg_read_cancel {
            stream: (*cf).stream,
        };
        proc_send(
            (*cf).peer,
            msgtype::MSG_READ_CANCEL,
            -1,
            &raw const msg as *const c_void,
            size_of::<msg_read_cancel>(),
        );
    }
}

unsafe fn file_push_cb(cf: *mut client_file) {

    unsafe {
        if (*cf).c.is_null() || !(*(*cf).c).flags.intersects(client_flag::DEAD) {
            file_push(cf);
        }
        file_free(cf);
    }
}

pub unsafe fn file_push(cf: *mut client_file) {
    unsafe {
        let mut msglen: usize;
        let mut sent: usize;

        let mut msg: Vec<u8> = Vec::with_capacity(size_of::<msg_write_data>());
        let mut left = EVBUFFER_LENGTH((*cf).buffer);
        while left != 0 {
            sent = left;
            if sent > MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_write_data>() {
                sent = MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_write_data>();
            }

            msglen = size_of::<msg_write_data>() + sent;
            msg.clear();
            msg.reserve(msglen);
            let msg_header = msg_write_data { stream: (*cf).stream };
            msg.extend_from_slice(std::slice::from_raw_parts(
                &raw const msg_header as *const u8,
                size_of::<msg_write_data>(),
            ));
            msg.extend_from_slice(std::slice::from_raw_parts(
                EVBUFFER_DATA((*cf).buffer).cast(),
                sent,
            ));
            if proc_send(
                (*cf).peer,
                msgtype::MSG_WRITE,
                -1,
                msg.as_ptr().cast(),
                msglen,
            ) != 0
            {
                break;
            }
            evbuffer_drain((*cf).buffer, sent);

            left = EVBUFFER_LENGTH((*cf).buffer);
            log_debug!("file {} sent {}, left {}", (*cf).stream, sent, left);
        }
        if left != 0 {
            (*cf).references += 1;
            defer(Box::new(move || unsafe { file_push_cb(cf) }));
        } else if (*cf).stream > 2 {
            let close: msg_write_close = msg_write_close {
                stream: (*cf).stream,
            };
            proc_send(
                (*cf).peer,
                msgtype::MSG_WRITE_CLOSE,
                -1,
                &raw const close as *const c_void,
                size_of::<msg_write_close>(),
            );
            file_fire_done(cf);
        }
    }
}

pub unsafe fn file_write_left(files: *mut client_files) -> c_int {
    let mut left;
    let mut waiting: i32 = 0;

    unsafe {
        for cf in (*files).values() {
            if cf.event.is_null() {
                continue;
            }
            left = EVBUFFER_LENGTH((*cf.event).output);
            if left != 0 {
                waiting += 1;
                log_debug!("file {} {} bytes left", cf.stream, left);
            }
        }
    }

    (waiting != 0) as i32
}

pub unsafe extern "C-unwind" fn file_write_error_callback(
    _bev: *mut bufferevent,
    _what: i16,
    arg: *mut c_void,
) {
    unsafe {
        let cf = arg as *mut client_file;

        log_debug!("write error file {}", (*cf).stream);

        bufferevent_free((*cf).event);
        (*cf).event = null_mut();

        close((*cf).fd);
        (*cf).fd = -1;

        if let Some(cb) = (*cf).cb {
            cb(null_mut(), null_mut(), 0, -1, null_mut(), (*cf).data);
        }
    }
}

pub unsafe extern "C-unwind" fn file_write_callback(_bev: *mut bufferevent, arg: *mut c_void) {
    unsafe {
        let cf = arg as *mut client_file;

        log_debug!("write check file {}", (*cf).stream);

        if let Some(cb) = (*cf).cb {
            cb(null_mut(), null_mut(), 0, -1, null_mut(), (*cf).data);
        }

        if (*cf).closed != 0 && EVBUFFER_LENGTH((*(*cf).event).output) == 0 {
            bufferevent_free((*cf).event);
            close((*cf).fd);
            file_free(cf);
        }
    }
}

pub unsafe fn file_write_open(
    files: *mut client_files,
    peer: *mut tmuxpeer,
    imsg: *mut imsg,
    allow_streams: i32,
    close_received: i32,
    cb: client_file_cb,
    cbdata: *mut c_void,
) {
    unsafe {
        let msg = (*imsg).data as *mut msg_write_open;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let path: *const u8;
        let flags = O_NONBLOCK | O_WRONLY | O_CREAT;
        let mut error: i32 = 0;
        'reply: {
            if msglen < size_of::<msg_write_open>() {
                fatalx("bad MSG_WRITE_OPEN size");
            }
            if msglen == size_of::<msg_write_open>() {
                path = c!("-");
            } else {
                path = msg.add(1).cast();
            }
            log_debug!("open write file {} {}", (*msg).stream, _s(path));

            if (*files).contains_key(&(*msg).stream) {
                error = EBADF;
                break 'reply;
            }
            let cf = file_create_with_peer(peer, files, (*msg).stream, cb, cbdata);
            if (*cf).closed != 0 {
                error = EBADF;
                break 'reply;
            }

            (*cf).fd = -1;
            if (*msg).fd == -1 {
                (*cf).fd = open(path, (*msg).flags | flags, 0o644);
            } else if allow_streams != 0 {
                if (*msg).fd != STDOUT_FILENO && (*msg).fd != STDERR_FILENO {
                    errno!() = EBADF;
                } else {
                    (*cf).fd = dup((*msg).fd);
                    if close_received != 0 {
                        close((*msg).fd);
                    } /* can only be used once */
                }
            } else {
                errno!() = EBADF;
            }
            if (*cf).fd == -1 {
                error = errno!();
                break 'reply;
            }

            (*cf).event = bufferevent_new(
                (*cf).fd,
                None,
                Some(file_write_callback),
                Some(file_write_error_callback),
                cf.cast(),
            );
            if (*cf).event.is_null() {
                fatalx("out of memory");
            }
            bufferevent_enable((*cf).event, EV_WRITE);
            break 'reply;
        }
        // reply:
        let reply: msg_write_ready = msg_write_ready {
            stream: (*msg).stream,
            error,
        };

        proc_send(
            peer,
            msgtype::MSG_WRITE_READY,
            -1,
            &raw const reply as _,
            size_of::<msg_write_ready>(),
        );
    }
}

pub unsafe fn file_write_data(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_write_data;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let size = msglen - size_of::<msg_write_data>();

        if msglen < size_of::<msg_write_data>() {
            fatalx("bad MSG_WRITE size");
        }
        let cf = (*files)
            .get_mut(&(*msg).stream)
            .map_or(null_mut(), |cf| &mut **cf as *mut client_file);
        if cf.is_null() {
            fatalx("unknown stream number");
        }
        log_debug!("write {} to file {}", size, (*cf).stream);

        if !(*cf).event.is_null() {
            bufferevent_write((*cf).event, msg.add(1).cast(), size);
        }
    }
}

pub unsafe fn file_write_close(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_write_close;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;

        if msglen != size_of::<msg_write_close>() {
            fatalx("bad MSG_WRITE_CLOSE size");
        }
        let cf = (*files)
            .get_mut(&(*msg).stream)
            .map_or(null_mut(), |cf| &mut **cf as *mut client_file);
        if cf.is_null() {
            fatalx("unknown stream number");
        }
        log_debug!("close file {}", (*cf).stream);

        if (*cf).event.is_null() || EVBUFFER_LENGTH((*(*cf).event).output) == 0 {
            if !(*cf).event.is_null() {
                bufferevent_free((*cf).event);
            }
            if (*cf).fd != -1 {
                close((*cf).fd);
            }
            file_free(cf);
        }
    }
}

pub unsafe extern "C-unwind" fn file_read_error_callback(
    _bev: *mut bufferevent,
    _what: i16,
    arg: *mut c_void,
) {
    unsafe {
        let cf = arg as *mut client_file;

        log_debug!("read error file {}", (*cf).stream);

        let msg: msg_read_done = msg_read_done {
            stream: (*cf).stream,
            error: 0,
        };
        proc_send(
            (*cf).peer,
            msgtype::MSG_READ_DONE,
            -1,
            &raw const msg as *const c_void,
            size_of::<msg_read_done>(),
        );

        bufferevent_free((*cf).event);
        close((*cf).fd);
        file_free(cf);
    }
}

pub unsafe extern "C-unwind" fn file_read_callback(_bev: *mut bufferevent, arg: *mut c_void) {
    let cf = arg as *mut client_file;
    unsafe {
        let mut msg: Vec<u8> = Vec::with_capacity(size_of::<msg_read_data>());

        loop {
            let bdata = EVBUFFER_DATA((*(*cf).event).input);
            let mut bsize = EVBUFFER_LENGTH((*(*cf).event).input);

            if bsize == 0 {
                break;
            }
            if bsize > MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_read_data>() {
                bsize = MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_read_data>();
            }
            log_debug!("read {} from file {}", bsize, (*cf).stream);

            let msglen = size_of::<msg_read_data>() + bsize;
            msg.clear();
            msg.reserve(msglen);
            msg.extend_from_slice(
                std::slice::from_raw_parts(
                &raw const (*cf).stream as *const u8,
                size_of::<msg_read_data>()
                )
            );
            msg.extend_from_slice(
                std::slice::from_raw_parts(
                    bdata,
                    bsize
                )
            );
            proc_send(
                (*cf).peer,
                msgtype::MSG_READ,
                -1,
                msg.as_ptr().cast(),
                msglen,
            );

            evbuffer_drain((*(*cf).event).input, bsize);
        }
    }
}

pub unsafe fn file_read_open(
    files: *mut client_files,
    peer: *mut tmuxpeer,
    imsg: *mut imsg,
    allow_streams: c_int,
    close_received: c_int,
    cb: client_file_cb,
    cbdata: *mut c_void,
) {
    unsafe {
        let msg = (*imsg).data as *mut msg_read_open;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let path: *const u8;
        let cf: *mut client_file;
        let flags = O_NONBLOCK | O_RDONLY;
        let error;

        'reply: {
            if msglen < size_of::<msg_read_done>() {
                fatalx("bad MSG_READ_OPEN size");
            }
            if msglen == size_of::<msg_read_done>() {
                path = c!("-");
            } else {
                path = msg.add(1).cast();
            }
            log_debug!("open read file {} {}", (*msg).stream, _s(path));

            if (*files).contains_key(&(*msg).stream) {
                error = EBADF;
                break 'reply;
            }
            cf = file_create_with_peer(peer, files, (*msg).stream, cb, cbdata);
            if (*cf).closed != 0 {
                error = EBADF;
                break 'reply;
            }

            (*cf).fd = -1;
            if (*msg).fd == -1 {
                (*cf).fd = open(path, flags, 0);
            } else if allow_streams != 0 {
                if (*msg).fd != STDIN_FILENO {
                    errno!() = EBADF;
                } else {
                    (*cf).fd = dup((*msg).fd);
                    if close_received != 0 {
                        close((*msg).fd);
                    }
                }
            } else {
                errno!() = EBADF;
            }
            if (*cf).fd == -1 {
                error = errno!();
                break 'reply;
            }

            (*cf).event = bufferevent_new(
                (*cf).fd,
                Some(file_read_callback),
                None,
                Some(file_read_error_callback),
                cf.cast(),
            );
            if (*cf).event.is_null() {
                fatalx("out of memory");
            }
            bufferevent_enable((*cf).event, EV_READ);
            return;
        }
        // reply:
        let reply = msg_read_done {
            stream: (*msg).stream,
            error,
        };
        proc_send(
            peer,
            msgtype::MSG_READ_DONE,
            -1,
            &raw const reply as _,
            size_of::<msg_read_done>(),
        );
    }
}

pub unsafe fn file_read_cancel(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_read_cancel;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;

        if msglen != size_of::<msg_read_cancel>() {
            fatalx("bad MSG_READ_CANCEL size");
        }
        let cf = (*files)
            .get_mut(&(*msg).stream)
            .map_or(null_mut(), |cf| &mut **cf as *mut client_file);
        if cf.is_null() {
            fatalx("unknown stream number");
        }
        log_debug!("cancel file {}", (*cf).stream);

        file_read_error_callback(null_mut(), 0, cf.cast());
    }
}

pub unsafe fn file_write_ready(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_write_ready;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;

        if msglen != size_of::<msg_write_ready>() {
            fatalx("bad MSG_WRITE_READY size");
        }
        let cf = (*files)
            .get_mut(&(*msg).stream)
            .map_or(null_mut(), |cf| &mut **cf as *mut client_file);
        if cf.is_null() {
            return;
        }
        if (*msg).error != 0 {
            (*cf).error = (*msg).error;
            file_fire_done(cf);
        } else {
            file_push(cf);
        }
    }
}

pub unsafe fn file_read_data(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_read_data;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let bdata: *mut c_void = msg.add(1).cast();
        let bsize = msglen - size_of::<msg_read_data>();

        if msglen < size_of::<msg_read_data>() {
            fatalx("bad MSG_READ_DATA size");
        }
        let cf = (*files)
            .get_mut(&(*msg).stream)
            .map_or(null_mut(), |cf| &mut **cf as *mut client_file);
        if cf.is_null() {
            return;
        }

        log_debug!("file {} read {} bytes", (*cf).stream, bsize);
        if (*cf).error == 0 && (*cf).closed == 0 {
            if evbuffer_add((*cf).buffer, bdata, bsize) != 0 {
                (*cf).error = ENOMEM;
                file_fire_done(cf);
            } else {
                file_fire_read(cf);
            }
        }
    }
}

pub unsafe fn file_read_done(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_read_done;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;

        if msglen != size_of::<msg_read_done>() {
            fatalx("bad MSG_READ_DONE size");
        }
        let cf = (*files)
            .get_mut(&(*msg).stream)
            .map_or(null_mut(), |cf| &mut **cf as *mut client_file);
        if cf.is_null() {
            return;
        }

        log_debug!("file {} read done", (*cf).stream);
        (*cf).error = (*msg).error;
        file_fire_done(cf);
    }
}
