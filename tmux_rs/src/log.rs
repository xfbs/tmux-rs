#![allow(clippy::missing_safety_doc)]
#![allow(non_upper_case_globals)]
//
// TODO
// cleanup the extern usage
//
use ::core::{
    ffi::{VaList, c_char, c_int, c_long, c_longlong, c_void},
    ptr::null_mut,
};

use ::libc::{
    __errno_location, FILE, fclose, fflush, fopen, fprintf, free, getpid, gettimeofday, setvbuf,
    snprintf, strerror, timeval,
};

unsafe extern "C" {
    fn vasprintf(_: *mut *mut c_char, _: *const c_char, _: VaList) -> c_int;
    fn stravis(_: *mut *mut c_char, _: *const c_char, _: c_int) -> c_int;
    fn xasprintf(_: *mut *mut c_char, _: *const c_char, _: ...) -> c_int;
    fn event_set_log_callback(cb: Option<unsafe extern "C" fn(c_int, *const c_char)>);
}

static mut log_file: *mut FILE = null_mut();
static mut log_level: c_int = 0;

// TODO log_event_cb

unsafe extern "C" fn log_event_cb(_severity: c_int, msg: *const c_char) {
    unsafe { log_debug(c"%s".as_ptr(), msg) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_add_level() {
    unsafe {
        log_level += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_get_level() -> c_int {
    unsafe { log_level }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_open(name: *const c_char) {
    unsafe {
        let mut path: *mut c_char = null_mut();
        if log_level == 0 {
            return;
        }
        log_close();

        xasprintf(
            &raw mut path as _,
            c"tmux-%s-%ld.log".as_ptr(),
            name,
            getpid() as c_long,
        );
        log_file = fopen(path, c"a".as_ptr());
        free(path as *mut c_void);
        if log_file.is_null() {
            return;
        }
        setvbuf(log_file, null_mut(), 1, 0);
        event_set_log_callback(Some(log_event_cb));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_toggle(name: *const c_char) {
    unsafe {
        if log_level == 0 {
            log_level = 1;
            log_open(name);
            log_debug(c"log opened".as_ptr());
        } else {
            log_debug(c"log closed".as_ptr());
            log_level = 0;
            log_close();
        };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn log_close() {
    unsafe {
        if !log_file.is_null() {
            fclose(log_file);
        }
        log_file = null_mut();
        event_set_log_callback(None);
    }
}

unsafe extern "C" fn log_vwrite(msg: *const c_char, mut ap: VaList, prefix: *const c_char) {
    unsafe {
        let mut s: *mut c_char = null_mut();
        let mut out: *mut c_char = null_mut();
        let mut tv: timeval = timeval {
            tv_sec: 0,
            tv_usec: 0,
        };
        if log_file.is_null() {
            return;
        }
        if vasprintf(&mut s, msg, ap.as_va_list()) == -1 {
            return;
        }
        if stravis(
            &mut out,
            s,
            0x1 as c_int | 0x2 as c_int | 0x8 as c_int | 0x10 as c_int,
        ) == -1
        {
            free(s as _);
            return;
        }
        free(s as _);
        gettimeofday(&mut tv, null_mut());
        if fprintf(
            log_file,
            c"%lld.%06d %s%s\n".as_ptr(),
            tv.tv_sec as c_longlong,
            tv.tv_usec as c_int,
            prefix,
            out,
        ) != -1
        {
            fflush(log_file);
        }
        free(out as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn log_debug(msg: *const c_char, mut args: ...) {
    unsafe {
        if log_file.is_null() {
            return;
        }
        log_vwrite(msg, args.as_va_list(), c"".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fatal(msg: *const c_char, mut ap: ...) -> ! {
    unsafe {
        let mut tmp: [u8; 256] = [0; 256];

        if snprintf(
            tmp.as_mut_ptr() as _,
            size_of_val(&tmp),
            c"fatal: %s: ".as_ptr(),
            strerror(*__errno_location()),
        ) < 0
        {
            std::process::exit(1);
        }

        log_vwrite(msg, ap.as_va_list(), tmp.as_ptr() as _);

        std::process::exit(1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fatalx(msg: *const c_char, mut args: ...) -> ! {
    unsafe {
        log_vwrite(msg, args.as_va_list(), c"fatal: ".as_ptr());
    }
    std::process::exit(1)
}
