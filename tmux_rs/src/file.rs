use compat_rs::tree::{rb_find, rb_insert, rb_remove};
use libc::{STDERR_FILENO, STDOUT_FILENO};
use libevent_sys::{EV_TIMEOUT, evbuffer_add, evbuffer_add_vprintf, evbuffer_free, evbuffer_new, event_once};

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
    pub fn file_write(
        _: *mut client,
        _: *const c_char,
        _: c_int,
        _: *const c_void,
        _: usize,
        _: client_file_cb,
        _: *mut c_void,
    );
    pub fn file_read(_: *mut client, _: *const c_char, _: client_file_cb, _: *const c_void) -> *mut client_file;
    pub fn file_cancel(_: *mut client_file);
    pub fn file_push(_: *mut client_file);
    pub fn file_write_left(_: *mut client_files) -> c_int;
    pub fn file_write_open(
        _: *mut client_files,
        _: *mut tmuxpeer,
        _: *mut imsg,
        _: c_int,
        _: c_int,
        _: client_file_cb,
        _: *mut c_void,
    );
    pub fn file_write_data(_: *mut client_files, _: *mut imsg);
    pub fn file_write_close(_: *mut client_files, _: *mut imsg);
    pub fn file_read_open(
        _: *mut client_files,
        _: *mut tmuxpeer,
        _: *mut imsg,
        _: c_int,
        _: c_int,
        _: client_file_cb,
        _: *mut c_void,
    );
    pub fn file_write_ready(_: *mut client_files, _: *mut imsg);
    pub fn file_read_data(_: *mut client_files, _: *mut imsg);
    pub fn file_read_done(_: *mut client_files, _: *mut imsg);
    pub fn file_read_cancel(_: *mut client_files, _: *mut imsg);
}

#[unsafe(no_mangle)]
pub static mut file_next_stream: i32 = 3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn file_get_path(c: *mut client, file: *const c_char) -> *const c_char {
    let mut path = null_mut();

    unsafe {
        if *file == b'/' as c_char {
            path = xstrdup(file).as_ptr();
        } else {
            xasprintf(
                &raw mut path,
                c"%s/%s".as_ptr(),
                server_client_get_cwd(c, null_mut()),
                file,
            );
        }
    }

    path
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
        rb_insert::<client_file>(files, cf);

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

/*
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
    unsafe { }
}
*/
