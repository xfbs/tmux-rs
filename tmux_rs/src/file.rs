use compat_rs::{
    imsg::{IMSG_HEADER_SIZE, MAX_IMSGSIZE},
    tree::{rb_find, rb_foreach, rb_insert, rb_remove},
};
use libc::{
    __errno_location, BUFSIZ, E2BIG, EBADF, EINVAL, EIO, ENOMEM, O_APPEND, O_CREAT, O_NONBLOCK, O_RDONLY, O_WRONLY,
    STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO, close, dup, fclose, ferror, fopen, fread, fwrite, memcpy, open, strcmp,
};
use libevent_sys::{
    EV_READ, EV_TIMEOUT, EV_WRITE, bufferevent_enable, bufferevent_free, bufferevent_new, bufferevent_write,
    evbuffer_add, evbuffer_add_vprintf, evbuffer_drain, evbuffer_free, evbuffer_new, event_once,
};

use crate::log::fatalx_;

use super::*;

unsafe extern "C" {
    // pub fn file_cmp(_: *mut client_file, _: *mut client_file) -> c_int;
    pub fn client_files_RB_INSERT_COLOR(_: *mut client_files, _: *mut client_file);
    pub fn client_files_RB_REMOVE_COLOR(_: *mut client_files, _: *mut client_file, _: *mut client_file);
    pub fn client_files_RB_REMOVE(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn client_files_RB_INSERT(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn client_files_RB_FIND(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    pub fn client_files_RB_NFIND(_: *mut client_files, _: *mut client_file) -> *mut client_file;
    // pub fn file_create_with_peer( _: *mut tmuxpeer, _: *mut client_files, _: c_int, _: client_file_cb, _: *mut c_void,) -> *mut client_file;
    // pub fn file_create_with_client(_: *mut client, _: c_int, _: client_file_cb, _: *mut c_void) -> *mut client_file;
    // pub fn file_free(_: *mut client_file);
    // pub fn file_fire_done(_: *mut client_file);
    // pub fn file_fire_read(_: *mut client_file);
    // pub fn file_can_print(_: *mut client) -> c_int;
    // pub fn file_print(_: *mut client, _: *const c_char, ...);
    // pub fn file_vprint(_: *mut client, _: *const c_char, _: VaList);
    // pub fn file_print_buffer(_: *mut client, _: *mut c_void, _: usize);
    // pub fn file_error(_: *mut client, _: *const c_char, ...);
    // pub fn file_write( _: *mut client, _: *const c_char, _: c_int, _: *const c_void, _: usize, _: client_file_cb, _: *mut c_void,);
    // pub fn file_read(_: *mut client, _: *const c_char, _: client_file_cb, _: *const c_void) -> *mut client_file;
    // pub fn file_cancel(_: *mut client_file);
    // pub fn file_push(_: *mut client_file);
    // pub fn file_write_left(_: *mut client_files) -> c_int;
    // pub fn file_write_open( _: *mut client_files, _: *mut tmuxpeer, _: *mut imsg, _: c_int, _: c_int, _: client_file_cb, _: *mut c_void,);
    // pub fn file_write_data(_: *mut client_files, _: *mut imsg);
    // pub fn file_write_close(_: *mut client_files, _: *mut imsg);
    // pub fn file_read_open( _: *mut client_files, _: *mut tmuxpeer, _: *mut imsg, _: c_int, _: c_int, _: client_file_cb, _: *mut c_void,);
    // pub fn file_write_ready(_: *mut client_files, _: *mut imsg);
    // pub fn file_read_data(_: *mut client_files, _: *mut imsg);
    // pub fn file_read_done(_: *mut client_files, _: *mut imsg);
    // pub fn file_read_cancel(_: *mut client_files, _: *mut imsg);
}

#[unsafe(no_mangle)]
pub static mut file_next_stream: i32 = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_get_path(c: *mut client, file: *const c_char) -> NonNull<c_char> {
    unsafe {
        if *file == b'/' as c_char {
            xstrdup(file)
        } else {
            xasprintf_(c"%s/%s", server_client_get_cwd(c, null_mut()), file)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_cmp(cf1: *const client_file, cf2: *const client_file) -> c_int {
    // TODO this can be more consise, just subtraction
    unsafe {
        if (*cf1).stream < (*cf2).stream {
            -1
        } else if (*cf1).stream > (*cf2).stream {
            1
        } else {
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_create_with_peer(
    peer: *mut tmuxpeer,
    files: *mut client_files,
    stream: c_int,
    cb: client_file_cb,
    cbdata: *mut c_void,
) -> *mut client_file {
    unsafe {
        let mut cf = xcalloc_::<client_file>(1).as_ptr();
        (*cf).c = null_mut();
        (*cf).references = 1;
        (*cf).stream = stream;

        (*cf).buffer = evbuffer_new();
        if ((*cf).buffer.is_null()) {
            fatalx_(format_args!("out of memory"));
        }

        (*cf).cb = cb;
        (*cf).data = cbdata;

        (*cf).peer = peer;
        (*cf).tree = files;
        rb_insert::<client_file, _>(files, cf);

        cf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_create_with_client(
    mut c: *mut client,
    stream: c_int,
    cb: client_file_cb,
    cbdata: *mut c_void,
) -> *mut client_file {
    unsafe {
        if !c.is_null() && (*c).flags & CLIENT_ATTACHED != 0 {
            c = null_mut();
        }

        let mut cf: *mut client_file = xcalloc_::<client_file>(1).as_ptr();
        (*cf).c = c;
        (*cf).references = 1;
        (*cf).stream = stream;

        (*cf).buffer = evbuffer_new();
        if (*cf).buffer.is_null() {
            fatalx_(format_args!("out of memory"));
        }

        (*cf).cb = cb;
        (*cf).data = cbdata;

        if !(*cf).c.is_null() {
            (*cf).peer = (*(*cf).c).peer;
            (*cf).tree = &raw mut (*(*cf).c).files;
            rb_insert(&raw mut (*(*cf).c).files, cf);
            (*(*cf).c).references += 1;
        }

        cf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_free(cf: *mut client_file) {
    unsafe {
        (*cf).references -= 1;
        if (*cf).references != 0 {
            return;
        }

        evbuffer_free((*cf).buffer);
        free_((*cf).path);

        if !(*cf).tree.is_null() {
            rb_remove((*cf).tree, cf);
        }
        if !(*cf).c.is_null() {
            server_client_unref((*cf).c);
        }

        free_(cf);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_fire_done_cb(_fd: i32, _events: i16, arg: *mut c_void) {
    unsafe {
        let mut cf: *mut client_file = arg as _;
        let mut c: *mut client = (*cf).c;

        if let Some(cb) = (*cf).cb {
            if ((*cf).closed != 0 || c.is_null() || !(*c).flags & CLIENT_DEAD != 0) {
                cb(c, (*cf).path, (*cf).error, 1, (*cf).buffer, (*cf).data);
            }
        }
        file_free(cf);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_fire_done(cf: *mut client_file) {
    unsafe {
        event_once(-1, EV_TIMEOUT as i16, Some(file_fire_done_cb), cf as _, null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_fire_read(cf: *mut client_file) {
    unsafe {
        if let Some(cb) = (*cf).cb {
            cb((*cf).c, (*cf).path, (*cf).error, 0, (*cf).buffer, (*cf).data);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_can_print(c: *mut client) -> c_int {
    unsafe {
        if c.is_null() || (*c).flags & CLIENT_ATTACHED != 0 || (*c).flags & CLIENT_CONTROL != 0 {
            0
        } else {
            1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_print(c: *mut client, fmt: *const c_char, mut args: ...) {
    unsafe {
        file_vprint(c, fmt, args.as_va_list());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_vprint(c: *mut client, fmt: *const c_char, mut ap: VaList) {
    unsafe {
        let mut cf: *mut client_file = null_mut();
        let mut find: client_file = zeroed();
        let mut msg: msg_write_open = zeroed();

        if file_can_print(c) == 0 {
            return;
        }

        find.stream = 1;
        let mut cf = rb_find(&raw mut (*c).files, &raw mut find);
        if cf.is_null() {
            cf = file_create_with_client(c, 1, None, null_mut());
            (*cf).path = xstrdup(c"-".as_ptr()).as_ptr();

            // TODO
            evbuffer_add_vprintf((*cf).buffer, fmt, core::mem::transmute(ap.clone().as_va_list()));

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
        } else {
            evbuffer_add_vprintf((*cf).buffer, fmt, core::mem::transmute(ap.as_va_list()));
            file_push(cf);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_print_buffer(c: *mut client, data: *mut c_void, size: usize) {
    unsafe {
        let mut cf: *mut client_file = null_mut();
        let mut find: client_file = zeroed();
        let mut msg: msg_write_open = zeroed();

        if file_can_print(c) == 0 {
            return;
        }

        find.stream = 1;

        let mut cf = rb_find(&raw mut (*c).files, &raw mut find);
        if cf.is_null() {
            cf = file_create_with_client(c, 1, None, null_mut());
            (*cf).path = xstrdup(c"-".as_ptr()).as_ptr();

            evbuffer_add((*cf).buffer, data, size);

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
        } else {
            evbuffer_add((*cf).buffer, data, size);
            file_push(cf);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_error(c: *mut client, fmt: *const c_char, mut args: ...) {
    unsafe {
        let mut cf: *mut client_file = null_mut();
        let mut find: client_file = zeroed();
        let mut msg: msg_write_open = zeroed();

        if file_can_print(c) == 0 {
            return;
        }

        let mut ap = args.clone();
        let mut ap = ap.as_va_list();

        find.stream = 2;
        cf = rb_find(&raw mut (*c).files, &raw mut find);
        if cf.is_null() {
            cf = file_create_with_client(c, 2, None, null_mut());
            (*cf).path = xstrdup(c"-".as_ptr()).as_ptr();

            evbuffer_add_vprintf((*cf).buffer, fmt, core::mem::transmute(ap));

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
        } else {
            evbuffer_add_vprintf((*cf).buffer, fmt, core::mem::transmute(args.as_va_list()));
            file_push(cf);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write(
    c: *mut client,
    path: *const c_char,
    flags: c_int,
    bdata: *const c_void,
    bsize: usize,
    cb: client_file_cb,
    cbdata: *mut c_void,
) {
    unsafe {
        let mut cf: *mut client_file = null_mut();
        let mut msg: *mut msg_write_open = null_mut();
        let mut msglen: usize = 0;
        let mut fd = -1;
        let mut stream: u32 = file_next_stream as u32;
        file_next_stream += 1;
        let mut f: *mut FILE = null_mut();
        let mut mode: *const c_char = null();

        'done: {
            'skip: {
                if strcmp(path, c"-".as_ptr()) == 0 {
                    cf = file_create_with_client(c, stream as i32, cb, cbdata);
                    (*cf).path = xstrdup_(c"-").as_ptr();

                    fd = STDOUT_FILENO;
                    if (c.is_null() || ((*c).flags & CLIENT_ATTACHED != 0) || ((*c).flags & CLIENT_CONTROL != 0)) {
                        (*cf).error = EBADF;
                        break 'done;
                    }
                    break 'skip;
                }

                cf = file_create_with_client(c, stream as i32, cb, cbdata);
                (*cf).path = file_get_path(c, path).as_ptr();

                if c.is_null() || (*c).flags & CLIENT_ATTACHED != 0 {
                    if (flags & O_APPEND != 0) {
                        mode = c"ab".as_ptr();
                    } else {
                        mode = c"wb".as_ptr();
                    }
                    f = fopen((*cf).path, mode);
                    if (f.is_null()) {
                        (*cf).error = *__errno_location();
                        break 'done;
                    }
                    if (fwrite(bdata, 1, bsize, f) != bsize) {
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
            if msglen > MAX_IMSGSIZE as usize - IMSG_HEADER_SIZE {
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
            if (proc_send((*cf).peer, msgtype::MSG_WRITE_OPEN, -1, msg.cast(), msglen) != 0) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read(
    c: *mut client,
    path: *const c_char,
    cb: client_file_cb,
    cbdata: *mut c_void,
) -> *mut client_file {
    unsafe {
        let mut cf;
        let mut msg: *mut msg_read_open = null_mut();
        let mut msglen: usize = 0;
        let mut fd: i32 = -1;
        let mut stream: u32 = file_next_stream as u32;
        file_next_stream += 1;
        let mut f: *mut FILE = null_mut();
        let mut size: usize = 0;
        let mut buffer = MaybeUninit::<[c_char; BUFSIZ as usize]>::uninit();
        'done: {
            'skip: {
                if (strcmp(path, c"-".as_ptr()) == 0) {
                    cf = file_create_with_client(c, stream as i32, cb, cbdata);
                    (*cf).path = xstrdup_(c"-").as_ptr();

                    fd = STDIN_FILENO;
                    if (c.is_null() || ((*c).flags & CLIENT_ATTACHED != 0) || ((*c).flags & CLIENT_CONTROL != 0)) {
                        (*cf).error = EBADF;
                        break 'done;
                    }
                    break 'skip;
                }

                cf = file_create_with_client(c, stream as i32, cb, cbdata);
                (*cf).path = file_get_path(c, path).as_ptr();

                if (c.is_null() || (*c).flags & CLIENT_ATTACHED != 0) {
                    f = fopen((*cf).path, c"rb".as_ptr());
                    if (f.is_null()) {
                        (*cf).error = *__errno_location();
                        break 'done;
                    }
                    loop {
                        size = fread(buffer.as_mut_ptr().cast(), 1, BUFSIZ as usize, f);
                        if (evbuffer_add((*cf).buffer, buffer.as_ptr().cast(), size) != 0) {
                            (*cf).error = ENOMEM;
                            break 'done;
                        }
                        if (size != BUFSIZ as usize) {
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
            if (msglen > MAX_IMSGSIZE as usize - IMSG_HEADER_SIZE) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_cancel(cf: *mut client_file) {
    unsafe {
        log_debug(c"read cancel file %d".as_ptr(), (*cf).stream);

        if ((*cf).closed != 0) {
            return;
        }
        (*cf).closed = 1;

        let msg: msg_read_cancel = msg_read_cancel { stream: (*cf).stream };
        proc_send(
            (*cf).peer,
            msgtype::MSG_READ_CANCEL,
            -1,
            &raw const msg as *const c_void,
            size_of::<msg_read_cancel>(),
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_push_cb(_fd: i32, _events: i16, arg: *mut c_void) {
    let mut cf = arg as *mut client_file;

    unsafe {
        if (*cf).c.is_null() || !(*(*cf).c).flags & CLIENT_DEAD != 0 {
            file_push(cf);
        }
        file_free(cf);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_push(cf: *mut client_file) {
    unsafe {
        let mut msglen: usize = 0;
        let mut sent: usize = 0;

        let mut msg = xmalloc_::<msg_write_data>();
        let mut left = EVBUFFER_LENGTH((*cf).buffer);
        while (left != 0) {
            sent = left;
            if sent > MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_write_data>() {
                sent = MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_write_data>();
            }

            msglen = size_of::<msg_write_data>() + sent;
            msg = xrealloc_(msg.as_ptr(), msglen);
            (*msg.as_ptr()).stream = (*cf).stream;
            memcpy(msg.as_ptr().add(1).cast(), EVBUFFER_DATA((*cf).buffer).cast(), sent);
            if (proc_send((*cf).peer, msgtype::MSG_WRITE, -1, msg.as_ptr().cast(), msglen) != 0) {
                break;
            }
            evbuffer_drain((*cf).buffer, sent);

            left = EVBUFFER_LENGTH((*cf).buffer);
            log_debug(c"file %d sent %zu, left %zuc".as_ptr(), (*cf).stream, sent, left);
        }
        if (left != 0) {
            (*cf).references += 1;
            event_once(-1, EV_TIMEOUT as i16, Some(file_push_cb), cf.cast(), null());
        } else if ((*cf).stream > 2) {
            let mut close: msg_write_close = msg_write_close { stream: (*cf).stream };
            proc_send(
                (*cf).peer,
                msgtype::MSG_WRITE_CLOSE,
                -1,
                &raw const close as *const c_void,
                size_of::<msg_write_close>(),
            );
            file_fire_done(cf);
        }
        free_(msg.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_left(files: *mut client_files) -> c_int {
    let mut left = 0;
    let mut waiting: i32 = 0;

    unsafe {
        rb_foreach(files, |cf| {
            if ((*cf).event.is_null()) {
                return ControlFlow::<(), ()>::Continue(());
            }
            left = EVBUFFER_LENGTH((*(*cf).event).output);
            if (left != 0) {
                waiting += 1;
                log_debug(c"file %u %zu bytes left".as_ptr(), (*cf).stream, left);
            }
            ControlFlow::<(), ()>::Continue(())
        });
    }

    (waiting != 0) as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_error_callback(bev: *mut bufferevent, what: i16, arg: *mut c_void) {
    unsafe {
        let cf = arg as *mut client_file;

        log_debug(c"write error file %d".as_ptr(), (*cf).stream);

        bufferevent_free((*cf).event);
        (*cf).event = null_mut();

        close((*cf).fd);
        (*cf).fd = -1;

        if let Some(cb) = (*cf).cb {
            cb(null_mut(), null_mut(), 0, -1, null_mut(), (*cf).data);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_callback(bev: *mut bufferevent, arg: *mut c_void) {
    unsafe {
        let cf = arg as *mut client_file;

        log_debug(c"write check file %d".as_ptr(), (*cf).stream);

        if let Some(cb) = (*cf).cb {
            cb(null_mut(), null_mut(), 0, -1, null_mut(), (*cf).data);
        }

        if ((*cf).closed != 0 && EVBUFFER_LENGTH((*(*cf).event).output) == 0) {
            bufferevent_free((*cf).event);
            close((*cf).fd);
            rb_remove((*cf).tree, cf);
            file_free(cf);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_open(
    files: *mut client_files,
    peer: *mut tmuxpeer,
    imsg: *mut imsg,
    allow_streams: i32,
    close_received: i32,
    cb: client_file_cb,
    cbdata: *mut c_void,
) {
    unsafe {
        let mut msg = (*imsg).data as *mut msg_write_open;
        let mut msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let mut path: *const c_char = null();
        let mut find: client_file = zeroed();
        let flags = O_NONBLOCK | O_WRONLY | O_CREAT;
        let mut error: i32 = 0;
        'reply: {
            if msglen < size_of::<msg_write_open>() {
                fatalx(c"bad MSG_WRITE_OPEN size".as_ptr());
            }
            if msglen == size_of::<msg_write_open>() {
                path = c"-".as_ptr();
            } else {
                path = msg.add(1).cast();
            }
            log_debug(c"open write file %d %s".as_ptr(), (*msg).stream, path);

            find.stream = (*msg).stream;
            if !rb_find(files, &raw mut find).is_null() {
                error = EBADF;
                break 'reply;
            }
            let cf = file_create_with_peer(peer, files, (*msg).stream, cb, cbdata);
            if ((*cf).closed != 0) {
                error = EBADF;
                break 'reply;
            }

            (*cf).fd = -1;
            if ((*msg).fd == -1) {
                (*cf).fd = open(path, (*msg).flags | flags, 0o644);
            } else if (allow_streams != 0) {
                if ((*msg).fd != STDOUT_FILENO && (*msg).fd != STDERR_FILENO) {
                    *__errno_location() = EBADF;
                } else {
                    (*cf).fd = dup((*msg).fd);
                    if (close_received != 0) {
                        close((*msg).fd); /* can only be used once */
                    }
                }
            } else {
                *__errno_location() = EBADF;
            }
            if ((*cf).fd == -1) {
                error = *__errno_location();
                break 'reply;
            }

            (*cf).event = bufferevent_new(
                (*cf).fd,
                None,
                Some(file_write_callback),
                Some(file_write_error_callback),
                cf.cast(),
            );
            if ((*cf).event.is_null()) {
                fatalx(c"out of memory".as_ptr());
            }
            bufferevent_enable((*cf).event, EV_WRITE as i16);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_data(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_write_data;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let mut find: client_file = zeroed(); // TODO use uninit
        let size = msglen - size_of::<msg_write_data>();

        if msglen < size_of::<msg_write_data>() {
            fatalx(c"bad MSG_WRITE size".as_ptr());
        }
        find.stream = (*msg).stream;
        let cf = rb_find(files, &raw mut find);
        if cf.is_null() {
            fatalx(c"unknown stream number".as_ptr());
        }
        log_debug(c"write %zu to file %d".as_ptr(), size, (*cf).stream);

        if (!(*cf).event.is_null()) {
            bufferevent_write((*cf).event, msg.add(1).cast(), size);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_close(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let mut msg = (*imsg).data as *mut msg_write_close;
        let mut msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let mut find: client_file = zeroed(); // TODO uninit
        // struct client_file find, *cf;

        if msglen != size_of::<msg_write_close>() {
            fatalx(c"bad MSG_WRITE_CLOSE size".as_ptr());
        }
        find.stream = (*msg).stream;
        let cf = rb_find(files, &raw mut find);
        if (cf.is_null()) {
            fatalx(c"unknown stream number".as_ptr());
        }
        log_debug(c"close file %d".as_ptr(), (*cf).stream);

        if ((*cf).event.is_null() || EVBUFFER_LENGTH((*(*cf).event).output) == 0) {
            if (!(*cf).event.is_null()) {
                bufferevent_free((*cf).event);
            }
            if ((*cf).fd != -1) {
                close((*cf).fd);
            }
            rb_remove(files, cf);
            file_free(cf);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read_error_callback(_bev: *mut bufferevent, what: i16, arg: *mut c_void) {
    unsafe {
        let cf = arg as *mut client_file;

        log_debug(c"read error file %d".as_ptr(), (*cf).stream);

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
        rb_remove((*cf).tree, cf);
        file_free(cf);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read_callback(bev: *mut bufferevent, arg: *mut c_void) {
    let mut cf = arg as *mut client_file;
    unsafe {
        let mut msg = xmalloc_::<msg_read_data>();

        loop {
            let mut bdata = EVBUFFER_DATA((*(*cf).event).input);
            let mut bsize = EVBUFFER_LENGTH((*(*cf).event).input);

            if (bsize == 0) {
                break;
            }
            if bsize > MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_read_data>() {
                bsize = MAX_IMSGSIZE - IMSG_HEADER_SIZE - size_of::<msg_read_data>();
            }
            log_debug(c"read %zu from file %d".as_ptr(), bsize, (*cf).stream);

            let msglen = size_of::<msg_read_data>() + bsize;
            msg = xrealloc_(msg.as_ptr(), msglen);
            (*msg.as_ptr()).stream = (*cf).stream;
            memcpy(msg.as_ptr().add(1).cast(), bdata.cast(), bsize);
            proc_send((*cf).peer, msgtype::MSG_READ, -1, msg.as_ptr().cast(), msglen);

            evbuffer_drain((*(*cf).event).input, bsize);
        }
        free_(msg.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read_open(
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
        let mut path = null();
        let mut cf: *mut client_file = null_mut();
        let flags = O_NONBLOCK | O_RDONLY;
        let mut error = 0;

        let mut find = MaybeUninit::<client_file>::uninit();

        'reply: {
            if msglen < size_of::<msg_read_done>() {
                fatalx(c"bad MSG_READ_OPEN size".as_ptr());
            }
            if msglen == size_of::<msg_read_done>() {
                path = c"-".as_ptr();
            } else {
                path = msg.add(1).cast();
            }
            log_debug(c"open read file %d %s".as_ptr(), (*msg).stream, path);

            (*find.as_mut_ptr()).stream = (*msg).stream;
            if !rb_find(files, find.as_mut_ptr()).is_null() {
                error = EBADF;
                break 'reply;
            }
            cf = file_create_with_peer(peer, files, (*msg).stream, cb, cbdata);
            if ((*cf).closed != 0) {
                error = EBADF;
                break 'reply;
            }

            (*cf).fd = -1;
            if ((*msg).fd == -1) {
                (*cf).fd = open(path, flags);
            } else if (allow_streams != 0) {
                if ((*msg).fd != STDIN_FILENO) {
                    *__errno_location() = EBADF;
                } else {
                    (*cf).fd = dup((*msg).fd);
                    if (close_received != 0) {
                        close((*msg).fd);
                    }
                }
            } else {
                *__errno_location() = EBADF;
            }
            if ((*cf).fd == -1) {
                error = *__errno_location();
                break 'reply;
            }

            (*cf).event = bufferevent_new(
                (*cf).fd,
                Some(file_read_callback),
                None,
                Some(file_read_error_callback),
                cf.cast(),
            );
            if ((*cf).event.is_null()) {
                fatalx(c"out of memory".as_ptr());
            }
            bufferevent_enable((*cf).event, EV_READ as i16);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read_cancel(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let mut msg = (*imsg).data as *mut msg_read_cancel;
        let mut msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let mut find = MaybeUninit::<client_file>::uninit();

        if msglen != size_of::<msg_read_cancel>() {
            fatalx(c"bad MSG_READ_CANCEL size".as_ptr());
        }
        (*find.as_mut_ptr()).stream = (*msg).stream;
        let cf = rb_find(files, find.as_mut_ptr());
        if cf.is_null() {
            fatalx(c"unknown stream number".as_ptr());
        }
        log_debug(c"cancel file %d".as_ptr(), (*cf).stream);

        file_read_error_callback(null_mut(), 0, cf.cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_write_ready(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_write_ready;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let mut find = MaybeUninit::<client_file>::uninit();

        if (msglen != size_of::<msg_write_ready>()) {
            fatalx(c"bad MSG_WRITE_READY size".as_ptr());
        }
        (*find.as_mut_ptr()).stream = (*msg).stream;
        let cf = rb_find(files, find.as_mut_ptr());
        if cf.is_null() {
            return;
        }
        if ((*msg).error != 0) {
            (*cf).error = (*msg).error;
            file_fire_done(cf);
        } else {
            file_push(cf);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read_data(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_read_data;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let bdata: *mut c_void = msg.add(1).cast();
        let bsize = msglen - size_of::<msg_read_data>();
        let mut find = MaybeUninit::<client_file>::uninit();

        if (msglen < size_of::<msg_read_data>()) {
            fatalx(c"bad MSG_READ_DATA size".as_ptr());
        }
        (*find.as_mut_ptr()).stream = (*msg).stream;
        let cf = rb_find(files, find.as_mut_ptr());
        if (cf.is_null()) {
            return;
        }

        log_debug(c"file %d read %zu bytes".as_ptr(), (*cf).stream, bsize);
        if ((*cf).error == 0 && (*cf).closed == 0) {
            if (evbuffer_add((*cf).buffer, bdata, bsize) != 0) {
                (*cf).error = ENOMEM;
                file_fire_done(cf);
            } else {
                file_fire_read(cf);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_read_done(files: *mut client_files, imsg: *mut imsg) {
    unsafe {
        let msg = (*imsg).data as *mut msg_read_done;
        let msglen = (*imsg).hdr.len as usize - IMSG_HEADER_SIZE;
        let mut find = MaybeUninit::<client_file>::uninit();

        if (msglen != size_of::<msg_read_done>()) {
            fatalx(c"bad MSG_READ_DONE size".as_ptr());
        }
        (*find.as_mut_ptr()).stream = (*msg).stream;
        let cf = rb_find(files, find.as_mut_ptr());
        if (cf.is_null()) {
            return;
        }

        log_debug(c"file %d read done".as_ptr(), (*cf).stream);
        (*cf).error = (*msg).error;
        file_fire_done(cf);
    }
}
