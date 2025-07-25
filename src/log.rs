// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use ::core::{
    ffi::{CStr, c_char, c_int},
    ptr::null_mut,
};
use ::std::{
    fs::File,
    io::{LineWriter, Write},
    os::fd::AsRawFd,
    sync::atomic::{AtomicI32, Ordering},
};
use std::{io::BufWriter, sync::Mutex};

use ::libc::{free, snprintf, strerror};

use crate::compat::{stravis, vis_flags};

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

unsafe extern "C" fn log_event_cb(_severity: c_int, msg: *const c_char) {
    unsafe { log_debug!("{}", _s(msg)) }
}

pub fn log_add_level() {
    log_level.fetch_add(1, DEFAULT_ORDERING);
}

pub extern "C" fn log_get_level() -> i32 {
    log_level.load(DEFAULT_ORDERING)
}

pub fn log_open(name: &CStr) {
    if log_level.load(DEFAULT_ORDERING) == 0 {
        return;
    }

    log_close();
    let pid = std::process::id();
    let Ok(file) = std::fs::File::options()
        .read(false)
        .write(true)
        .append(true)
        .create(true)
        .open(format!("tmux-{}-{}.log", name.to_str().unwrap(), pid))
    else {
        return;
    };

    *log_file.lock().unwrap() = Some(LineWriter::new(file));
    unsafe { event_set_log_callback(Some(log_event_cb)) };
}

pub fn log_toggle(name: &CStr) {
    if log_level.fetch_xor(1, DEFAULT_ORDERING) == 0 {
        log_open(name);
        log_debug!("log opened");
    } else {
        log_debug!("log closed");
        log_close();
    };
}

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
                    let bw = std::mem::transmute::<
                        std::io::LineWriter<std::fs::File>,
                        BufWriter<File>,
                    >(lw);
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

#[track_caller]
pub fn log_debug_rs(args: std::fmt::Arguments) {
    if log_file.lock().unwrap().is_none() {
        return;
    }
    log_vwrite_rs(args, "");
}

#[track_caller]
fn log_vwrite_rs(args: std::fmt::Arguments, prefix: &str) {
    unsafe {
        if log_file.lock().unwrap().is_none() {
            return;
        }

        let msg = format!("{args}\0").to_string();
        let mut out: *mut c_char = null_mut();
        if stravis(
            &mut out,
            msg.as_ptr().cast(),
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        ) == -1
        {
            return;
        }
        let duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();
        let micros = duration.subsec_micros();

        let str_out = CStr::from_ptr(out).to_string_lossy();
        if let Some(f) = log_file.lock().unwrap().as_mut() {
            let location = std::panic::Location::caller();
            let file = location.file();
            let line = location.line();
            let _ = f.write_fmt(format_args!(
                "{secs}.{micros:06} {file}:{line} {prefix}{str_out}\n"
            ));
        }

        crate::free_(out);
    }
}

pub unsafe fn fatal(msg: *const c_char) -> ! {
    unsafe {
        let mut tmp: [c_char; 256] = [0; 256];

        if snprintf(
            tmp.as_mut_ptr(),
            size_of_val(&tmp),
            c"fatal: %s: ".as_ptr(),
            strerror(errno!()),
        ) < 0
        {
            std::process::exit(1);
        }

        log_vwrite_rs(
            format_args!("{}", _s(msg)),
            CStr::from_ptr(tmp.as_ptr())
                .to_str()
                .expect("fatal: invalid utf8"),
        );

        std::process::exit(1)
    }
}

macro_rules! fatalx_ {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        crate::log::fatalx_c(format_args!($fmt $(, $args)*))
    };
}
pub(crate) use fatalx_;
pub unsafe fn fatalx_c(args: std::fmt::Arguments) -> ! {
    log_vwrite_rs(args, "fatal: ");
    std::process::exit(1)
}

#[track_caller]
pub fn fatalx(msg: &CStr) -> ! {
    let msg = msg.to_str().unwrap();

    let location = std::panic::Location::caller();
    let file = location.file();
    let line = location.line();

    log_vwrite_rs(format_args!("{file}:{line} {msg}"), "fatal: ");
    std::process::exit(1)
}
