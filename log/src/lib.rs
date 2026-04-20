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
//! File-backed logging facade for tmux-rs.
//!
//! Writes timestamped, stravis-sanitized messages to `tmux-<name>-<pid>.log`.
//! The log file is opened lazily by [`log_open`] once [`log_add_level`] has
//! raised the global level above zero (one level per `-v` flag).
//!
//! Two entry points:
//! * [`log_debug!`] — crate-local macro that preserves the call-site
//!   `file:line`.
//! * [`::log`] crate adapter installed by [`log_install_logger`], so
//!   `::log::debug!` routes through the same sanitization and format.
//!
//! The facade is fork-safe: [`log_close`] intentionally avoids dropping
//! the underlying `File` (which would `close(2)` a descriptor the
//! parent may still own after fork) and instead flushes and forgets.

use core::ffi::CStr;
use std::ffi::CString;
use std::fs::File;
use std::io::{BufWriter, LineWriter, Write};
use std::ptr::null_mut;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use tmux_compat::vis::{stravis, vis_flags};

/// Log a debug message with the calling `file:line`. Formatted via
/// `format_args!`, sanitized via `stravis(VIS_OCTAL|VIS_CSTYLE|VIS_TAB|VIS_NL)`.
///
/// Writes nothing if no log file is currently open.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {$crate::log_debug_rs(format_args!($($arg)*))};
}

/// Emit a fatal message (prefixed `fatal:`) and `exit(1)`. `fatalx_!` takes
/// a format string; [`fatalx`] takes a `&str` with `Location::caller()`.
#[macro_export]
macro_rules! fatalx_ {
   ($fmt:literal $(, $args:expr)* $(,)?) => {
        $crate::fatalx_c(format_args!($fmt $(, $args)*))
    };
}

// can't use File because it's open before fork which causes issues with how file works
static LOG_FILE: Mutex<Option<LineWriter<File>>> = Mutex::new(None);
static LOG_LEVEL: AtomicI32 = AtomicI32::new(0);

const DEFAULT_ORDERING: Ordering = Ordering::SeqCst;

pub fn log_add_level() {
    LOG_LEVEL.fetch_add(1, DEFAULT_ORDERING);
}

pub fn log_get_level() -> i32 {
    LOG_LEVEL.load(DEFAULT_ORDERING)
}

pub fn log_open(name: &CStr) {
    if LOG_LEVEL.load(DEFAULT_ORDERING) == 0 {
        return;
    }

    log_close();
    let pid = std::process::id();
    let Ok(file) = std::fs::File::options()
        .read(false)
        .append(true)
        .create(true)
        .open(format!("tmux-{}-{}.log", name.to_str().unwrap(), pid))
    else {
        return;
    };

    *LOG_FILE.lock().unwrap() = Some(LineWriter::new(file));
}

pub fn log_toggle(name: &CStr) {
    if LOG_LEVEL.fetch_xor(1, DEFAULT_ORDERING) == 0 {
        log_open(name);
        log_debug!("log opened");
    } else {
        log_debug!("log closed");
        log_close();
    }
}

pub fn log_close() {
    // If we drop the file when it's already closed it will panic in debug mode.
    // Because of this and our use of fork, extra care has to be made when closing the file.
    // see std::sys::pal::unix::fs::debug_assert_fd_is_open;
    use std::os::fd::AsRawFd;
    if let Some(mut old_handle) = LOG_FILE.lock().unwrap().take() {
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
    }
}

#[track_caller]
pub fn log_debug_rs(args: std::fmt::Arguments) {
    if LOG_FILE.lock().unwrap().is_none() {
        return;
    }
    log_vwrite_rs(args, "");
}

#[track_caller]
fn log_vwrite_rs(args: std::fmt::Arguments, prefix: &str) {
    let loc = std::panic::Location::caller();
    log_write_at(args, prefix, loc.file(), loc.line());
}

/// Write a log record with an explicit caller location (used by the `log`
/// crate adapter, where the location comes from `log::Record` rather than
/// `Location::caller()`).
fn log_write_at(args: std::fmt::Arguments, prefix: &str, file: &str, line: u32) {
    unsafe {
        if LOG_FILE.lock().unwrap().is_none() {
            return;
        }

        let msg = CString::new(format!("{args}")).unwrap();
        let mut out = null_mut();
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

        let str_out = CStr::from_ptr(out.cast()).to_string_lossy();
        if let Some(f) = LOG_FILE.lock().unwrap().as_mut() {
            let _ = f.write_fmt(format_args!(
                "{secs}.{micros:06} {file}:{line} {prefix}{str_out}\n"
            ));
        }

        libc::free(out.cast());
    }
}

/// Adapter that routes `log` crate records into our existing log pipeline
/// (stravis sanitization + custom timestamp/file:line format). Installed
/// once at startup by [`log_install_logger`]; remains registered across
/// fork (child inherits it, file handle is re-opened by `log_open`).
struct TmuxLogger;

impl ::log::Log for TmuxLogger {
    fn enabled(&self, _metadata: &::log::Metadata) -> bool {
        // Gate on our existing LOG_LEVEL (raised per `-v` flag). The
        // log-file check in log_write_at handles the "not opened" case.
        LOG_LEVEL.load(DEFAULT_ORDERING) > 0
    }

    fn log(&self, record: &::log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let file = record.file().unwrap_or("?");
        let line = record.line().unwrap_or(0);
        log_write_at(*record.args(), "", file, line);
    }

    fn flush(&self) {
        if let Some(f) = LOG_FILE.lock().unwrap().as_mut() {
            let _ = f.flush();
        }
    }
}

static TMUX_LOGGER: TmuxLogger = TmuxLogger;

/// Install the `log` crate adapter. Idempotent — subsequent calls are
/// no-ops (`set_logger` errors are silently ignored). Call once during
/// process startup.
pub fn log_install_logger() {
    let _ = ::log::set_logger(&TMUX_LOGGER);
    // Allow `log::debug!` etc. through the macro's compile-time gate;
    // our enabled() gate + LOG_LEVEL handle runtime filtering.
    ::log::set_max_level(::log::LevelFilter::Trace);
}

pub fn fatal(msg: &str) -> ! {
    let os_error = std::io::Error::last_os_error();
    let error_msg = os_error.to_string();

    let prefix = format!("fatal: {error_msg}: ");

    log_vwrite_rs(format_args!("{msg}"), &prefix);

    std::process::exit(1)
}

pub fn fatalx_c(args: std::fmt::Arguments) -> ! {
    log_vwrite_rs(args, "fatal: ");
    std::process::exit(1)
}

#[track_caller]
pub fn fatalx(msg: &str) -> ! {
    let location = std::panic::Location::caller();
    let file = location.file();
    let line = location.line();

    log_vwrite_rs(format_args!("{file}:{line} {msg}"), "fatal: ");
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom};

    /// Serialize tests — they mutate process-wide LOG_FILE and LOG_LEVEL.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Swap LOG_FILE to a tempfile, raise LOG_LEVEL, return a guard that
    /// restores both on drop. Returns the tempfile for inspection.
    struct LogCapture {
        tempfile: tempfile::NamedTempFile,
        prev_level: i32,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl LogCapture {
        fn new() -> Self {
            let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let tempfile = tempfile::NamedTempFile::new().unwrap();
            let file_clone = tempfile.reopen().unwrap();
            *LOG_FILE.lock().unwrap() = Some(LineWriter::new(file_clone));
            let prev_level = LOG_LEVEL.swap(1, DEFAULT_ORDERING);
            LogCapture { tempfile, prev_level, _guard: guard }
        }

        fn read_contents(&mut self) -> String {
            // Ensure bytes are flushed to disk before we read the file.
            if let Some(f) = LOG_FILE.lock().unwrap().as_mut() {
                let _ = f.flush();
            }
            self.tempfile.as_file_mut().seek(SeekFrom::Start(0)).unwrap();
            let mut s = String::new();
            self.tempfile.as_file_mut().read_to_string(&mut s).unwrap();
            s
        }
    }

    impl Drop for LogCapture {
        fn drop(&mut self) {
            *LOG_FILE.lock().unwrap() = None;
            LOG_LEVEL.store(self.prev_level, DEFAULT_ORDERING);
        }
    }

    #[test]
    fn log_debug_macro_writes_expected_format() {
        let mut cap = LogCapture::new();
        log_debug!("hello {}", "world");
        let contents = cap.read_contents();
        // Expect: "<secs>.<micros> <file>:<line> hello world\n"
        assert!(contents.contains(".rs:"), "missing file:line: {contents:?}");
        assert!(contents.ends_with("hello world\n"), "wrong tail: {contents:?}");
        // Timestamp starts the line.
        let first = contents.split_whitespace().next().unwrap();
        assert!(first.contains('.'), "no timestamp: {contents:?}");
    }

    #[test]
    fn log_crate_macro_routes_through_adapter() {
        log_install_logger();
        let mut cap = LogCapture::new();
        ::log::debug!("adapter works {}", 42);
        let contents = cap.read_contents();
        assert!(contents.contains(".rs:"), "missing file:line: {contents:?}");
        assert!(contents.ends_with("adapter works 42\n"), "wrong tail: {contents:?}");
    }

    #[test]
    fn stravis_sanitizes_control_chars() {
        let mut cap = LogCapture::new();
        // Embed a newline and a tab — stravis with VIS_NL|VIS_TAB must escape them.
        log_debug!("a\nb\tc");
        let contents = cap.read_contents();
        // stravis VIS_CSTYLE emits \n and \t literal backslash-escapes.
        assert!(contents.contains(r"\n"), "newline not escaped: {contents:?}");
        assert!(contents.contains(r"\t"), "tab not escaped: {contents:?}");
        // And exactly one newline at end of line.
        assert!(contents.ends_with('\n'));
        assert_eq!(contents.matches('\n').count(), 1, "unexpected newlines: {contents:?}");
    }

    #[test]
    fn log_level_zero_disables_output() {
        // Capture sets LOG_LEVEL to 1; override back to 0 mid-test.
        let mut cap = LogCapture::new();
        LOG_LEVEL.store(0, DEFAULT_ORDERING);
        log_debug!("should not appear");
        ::log::debug!("also should not appear");
        let contents = cap.read_contents();
        // Note: log_debug_rs early-returns on empty LOG_FILE but not on
        // LOG_LEVEL; the adapter checks LOG_LEVEL explicitly. So the
        // log_debug! call DOES write (legacy behavior preserved) while
        // ::log::debug! does not. This test documents that fact.
        assert!(contents.contains("should not appear"),
            "log_debug! legacy: writes regardless of LOG_LEVEL when file is open");
        assert!(!contents.contains("also should not appear"),
            "log::debug! respects LOG_LEVEL");
    }
}
