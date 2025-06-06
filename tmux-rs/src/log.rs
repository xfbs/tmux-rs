use ::core::{
    ffi::{CStr, VaList, c_char, c_int, c_long, c_longlong, c_void},
    ptr::null_mut,
};
use ::std::{
    fs::File,
    io::{LineWriter, Write},
    os::fd::AsRawFd,
    sync::atomic::{AtomicI32, Ordering},
};
use std::{io::BufWriter, sync::Mutex};

use ::libc::{free, gettimeofday, setvbuf, snprintf, strerror};

use crate::compat::{VIS_CSTYLE, VIS_NL, VIS_OCTAL, VIS_TAB, stravis};

use crate::xmalloc::xasprintf;
use crate::{_s, event_::event_set_log_callback};
use crate::{libc_::errno, vasprintf};

macro_rules! log_debug {
    ($($arg:tt)*) => {$crate::log::log_debug_rs(format_args!($($arg)*))};
}
pub(crate) use log_debug;

// can't use File because it's open before fork which causes issues with how file works
static log_file: Mutex<Option<LineWriter<File>>> = Mutex::new(None);
static log_level: AtomicI32 = AtomicI32::new(0);

const DEFAULT_ORDERING: Ordering = Ordering::SeqCst;

unsafe extern "C" fn log_event_cb(_severity: c_int, msg: *const c_char) { unsafe { log_debug!("{}", _s(msg)) } }

pub fn log_add_level() { log_level.fetch_add(1, DEFAULT_ORDERING); }

#[unsafe(no_mangle)]
pub extern "C" fn log_get_level() -> i32 { log_level.load(DEFAULT_ORDERING) }

#[unsafe(no_mangle)]
pub fn log_open(name: &CStr) {
    if log_level.load(DEFAULT_ORDERING) == 0 {
        return;
    }

    log_close();
    let pid = std::process::id();
    let Ok(file) = std::fs::File::options().read(false).write(true).append(true).create(true).open(format!("tmux-{}-{}.log", name.to_str().unwrap(), pid)) else {
        return;
    };

    *log_file.lock().unwrap() = Some(LineWriter::new(file));
    unsafe { event_set_log_callback(Some(log_event_cb)) };
}

#[unsafe(no_mangle)]
pub fn log_toggle(name: &CStr) {
    if log_level.fetch_xor(1, DEFAULT_ORDERING) == 0 {
        log_open(name);
        log_debug!("log opened");
    } else {
        log_debug!("log closed");
        log_close();
    };
}

#[unsafe(no_mangle)]
pub fn log_close() {
    // If we drop the file when it's already closed it will panic in debug mode.
    // Because of this and our use of fork, extra care has to be made when closing the file.
    // see std::sys::pal::unix::fs::debug_assert_fd_is_open;
    use std::os::fd::AsRawFd;
    if let Some(mut old_handle) = log_file.lock().unwrap().take() {
        let _flush_err = old_handle.flush(); // TODO
        match old_handle.into_inner() {
            Ok(file) => unsafe {
                libc::close(file.as_raw_fd());
                std::mem::forget(file);
            },
            Err(err) => {
                let lw = err.into_inner();
                // TODO this is invalid, and compiler version dependent, but prevents a memory leak
                // need a way to properly get out the file and drop the buffer
                unsafe {
                    let bw = std::mem::transmute::<std::io::LineWriter<std::fs::File>, BufWriter<File>>(lw);
                    let (file, _) = bw.into_parts();
                    std::mem::forget(file);
                }
            }
        }

        unsafe {
            event_set_log_callback(None);
        }
    }
}

#[allow(improper_ctypes_definitions)]
unsafe extern "C" fn log_vwrite(msg: &CStr, mut ap: VaList, prefix: &CStr) {
    unsafe {
        if log_file.lock().unwrap().is_none() {
            return;
        }

        let msg = msg.as_ptr();
        let mut s: *mut c_char = null_mut();
        if vasprintf(&mut s, msg, ap.as_va_list()) == -1 {
            return;
        }

        let mut out: *mut c_char = null_mut();
        if stravis(&mut out, s, VIS_OCTAL | VIS_CSTYLE | VIS_TAB | VIS_NL) == -1 {
            free(s as _);
            return;
        }
        free(s as _);

        let duration = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let micros = duration.subsec_micros();

        let str_prefix = prefix.to_str().expect("string prefix must be valid utf8");
        let str_out = CStr::from_ptr(out).to_str().expect("out must be valid utf8");

        let _res = log_file.lock().unwrap().as_mut().expect("race condition").write_fmt(format_args!("{secs}.{micros:06} {str_prefix}{str_out}\n"));
        _res.unwrap();
        free(out as *mut c_void);
    }
}

// TODO: key differences, no string formatting
/*
fn log_vwrite_rs(args: std::fmt::Arguments, prefix: &CStr) {
    unsafe {
        let msg = format!("{args}\0").to_string();

        let mut out: *mut c_char = null_mut();
        let mut tv: timeval = timeval { tv_sec: 0, tv_usec: 0 };
        if log_file.is_null() {
            return;
        }
        if stravis(
            &mut out,
            msg.as_ptr() as _,
            (VIS_OCTAL | VIS_CSTYLE | VIS_TAB | VIS_NL) as i32,
        ) == -1
        {
            return;
        }
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
*/

// old rust version
//
// pub unsafe extern "C" fn log_debug(msg: &CStr, mut args: ...) {
//     unsafe {
//         if log_file.is_none() {
//             return;
//         }
//         log_vwrite(msg, args.as_va_list(), c"");
//     }
// }

#[unsafe(export_name = "log_debug")]
pub unsafe extern "C" fn log_debug_c(msg: *const c_char, mut args: ...) {
    unsafe {
        if log_file.lock().unwrap().is_none() {
            return;
        }
        log_vwrite(CStr::from_ptr(msg), args.as_va_list(), c"");
    }
}

pub fn log_debug_rs(args: std::fmt::Arguments) {
    if log_file.lock().unwrap().is_none() {
        return;
    }
    log_vwrite_rs(args, "");
}

fn log_vwrite_rs(args: std::fmt::Arguments, prefix: &str) {
    unsafe {
        if log_file.lock().unwrap().is_none() {
            return;
        }

        let msg = format!("{args}\0").to_string();
        let mut out: *mut c_char = null_mut();
        if stravis(&mut out, msg.as_ptr().cast(), VIS_OCTAL | VIS_CSTYLE | VIS_TAB | VIS_NL) == -1 {
            return;
        }
        let duration = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();
        let micros = duration.subsec_micros();

        let str_out = CStr::from_ptr(out).to_string_lossy();
        if let Some(f) = log_file.lock().unwrap().as_mut() {
            let _ = f.write_fmt(format_args!("{secs}.{micros:06} {prefix}{str_out}\n"));
        }

        crate::free_(out);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fatal(msg: *const c_char, mut ap: ...) -> ! {
    unsafe {
        let mut tmp: [c_char; 256] = [0; 256];

        if snprintf(tmp.as_mut_ptr(), size_of_val(&tmp), c"fatal: %s: ".as_ptr(), strerror(errno!())) < 0 {
            std::process::exit(1);
        }

        log_vwrite(CStr::from_ptr(msg), ap.as_va_list(), CStr::from_ptr(tmp.as_ptr()));

        std::process::exit(1)
    }
}

#[unsafe(export_name = "fatalx")]
pub unsafe extern "C" fn fatalx_c(msg: *const c_char, mut args: ...) -> ! {
    unsafe {
        log_vwrite(CStr::from_ptr(msg), args.as_va_list(), c"fatal: ");
    }
    std::process::exit(1)
}

pub fn fatalx(msg: &CStr) -> ! {
    let msg = msg.to_str().unwrap();
    log_vwrite_rs(format_args!("{msg}"), "fatal: ");
    std::process::exit(1)
}

// pub fn fatalx_(args: std::fmt::Arguments) -> ! { unsafe { log_vwrite_rs(args, c"fatal: "); } std::process::exit(1) }

// below are more ergonomic rust implementations
/*
unsafe extern "C" fn log_vwrite_rs(msg: std::fmt::Arguments<'_>, prefix: std::fmt::Arguments<'_>) {
    unsafe {
        if log_file.is_null() {
            return;
        }

        // TODO strip formatted msg
        /*
        if stravis(&mut out, s, 0x1 | 0x2 | 0x8 | 0x10 ) == -1 {
            return;
        }
        */
        let mut tv: timeval = timeval { tv_sec: 0, tv_usec: 0 };
        gettimeofday(&mut tv, null_mut());
        let s = format!("{}.{:06} {}{}\n\0", tv.tv_sec, tv.tv_usec, prefix, msg);
        fprintf(log_file, "%s", s.as_ptr());
        fflush(log_file);
    }
}


macro_rules! fatal_rs {
    ($e:expr) => {};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fatal_rs(args: &std::fmt::Arguments<'_>) -> ! {
    unsafe {
        let mut tmp: [u8; 256] = [0; 256];

        if std::fmt::write(tmp, args).is_err() {
            std::process::exit(1);
        }
        if snprintf(
            tmp.as_mut_ptr() as _,
            size_of_val(&tmp),
            c"fatal: %s: ".as_ptr(),
            strerror(errno!()),
        ) < 0
        {}

        log_vwrite_rs();

        std::process::exit(1)
    }
}
*/

/*
mod test {
    use super::*;

    #[test]
    fn test_open_logfile() {
        let pid = 100;
        let name = c"test";

        let path = format!("tmux-{}-{}.log", name.to_str().unwrap(), pid);
        let ptr = path.as_ptr();
        let file = match std::fs::File::options()
            .read(false)
            .write(true)
            .append(true)
            .create(true)
            .open(path)
        {
            Ok(file) => file,
            Err(reason) => {
                std::fs::write(
                    "panic.log",
                    format!("unable to open log file {}: {reason}", name.to_str().unwrap()),
                );
                return;
            }
        };

        let mut some_file = Some(LineWriter::new(file));
        some_file
            .as_mut()
            .unwrap()
            .write_fmt(format_args!("hello\nworld\n"))
            .unwrap();
    }
}
*/
