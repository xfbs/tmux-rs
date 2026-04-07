// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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
//! Window name management.
//!
//! Handles automatic window renaming based on the running command.
//! When `automatic-rename` is enabled, tmux periodically checks if the
//! active pane's command has changed and updates the window title.
//!
//! Key functions:
//! - [`parse_window_name`]: Extracts a clean program name from a command string.
//!   Strips quotes, `"exec "` prefix, leading whitespace/dashes (login shells),
//!   trailing non-printable characters, and takes the basename for absolute paths.
//! - [`check_window_name`]: Timer-driven check that reformats the window name
//!   using `automatic-rename-format` when the active pane changes.
//! - [`default_window_name`]: Returns the default name for a window based on
//!   the active pane's command or shell.

use crate::event_::{event_add, event_initialized};
use crate::libc::{gettimeofday, memcpy, strchr, strcspn, strlen, strncmp};
use crate::*;
use crate::options_::*;

pub unsafe extern "C-unwind" fn name_time_callback(
    _fd: c_int,
    _events: c_short,
    w: NonNull<window>,
) {
    unsafe {
        log_debug!("@{} timer expired", (*w.as_ptr()).id);
    }
}

/// Returns 0 if the name update interval has elapsed, or the remaining
/// microseconds if the timer hasn't expired yet.
pub unsafe fn name_time_expired(w: *mut window, tv: *mut timeval) -> c_int {
    unsafe {
        let mut offset: MaybeUninit<timeval> = MaybeUninit::<timeval>::uninit();

        timersub(tv, &raw mut (*w).name_time, offset.as_mut_ptr());
        let offset = offset.assume_init_ref();

        if offset.tv_sec != 0 || offset.tv_usec > NAME_INTERVAL {
            0
        } else {
            (NAME_INTERVAL - offset.tv_usec) as c_int
        }
    }
}

/// Check if the active pane's command has changed and update the window name.
/// Rate-limited by `NAME_INTERVAL` to avoid excessive renames.
pub unsafe fn check_window_name(w: *mut window) {
    unsafe {
        let mut tv: timeval = zeroed();
        let mut next: timeval = zeroed();

        let active = window_active_pane(w);
        if active.is_null() {
            return;
        }

        if options_get_number_((*w).options, "automatic-rename") == 0 {
            return;
        }

        if !(*active)
            .flags
            .intersects(window_pane_flags::PANE_CHANGED)
        {
            // log_debug!("@{} pane not changed", (*w).id);
            return;
        }
        log_debug!("@{} pane changed", (*w).id);

        gettimeofday(&raw mut tv, null_mut());
        let left = name_time_expired(w, &raw mut tv);
        if left != 0 {
            if event_initialized(&raw mut (*w).name_event) == 0 {
                evtimer_set(
                    &raw mut (*w).name_event,
                    name_time_callback,
                    NonNull::new_unchecked(w),
                );
            }
            if evtimer_pending(&raw mut (*w).name_event, null_mut()) == 0 {
                log_debug!("@{} timer queued ({})", (*w).id, left);
                timerclear(&raw mut next);
                next.tv_usec = left as libc::suseconds_t;
                event_add(&raw mut (*w).name_event, &raw const next);
            } else {
                log_debug!("@{} timer already queued ({})", (*w).id, left);
            }
            return;
        }
        memcpy(
            &raw mut (*w).name_time as _,
            &raw const tv as _,
            size_of::<timeval>(),
        );
        if event_initialized(&raw mut (*w).name_event) != 0 {
            evtimer_del(&raw mut (*w).name_event);
        }

        (*active).flags &= !window_pane_flags::PANE_CHANGED;

        let name = format_window_name(w);
        let name_str = std::ffi::CStr::from_ptr(name as *const i8).to_string_lossy();
        let cur = (*w).name.as_deref().unwrap_or("");
        if name_str != cur {
            log_debug!("@{} name {} (was {})", (*w).id, name_str, cur);
            window_set_name(w, name);
            server_redraw_window_borders(w);
            server_status_window(w);
        } else {
            log_debug!("@{} not changed (still {})", (*w).id, cur);
        }

        free(name as _);
    }
}

/// Returns the default name for a window based on the active pane's command.
pub unsafe fn default_window_name(w: *mut window) -> String {
    unsafe {
        let active = window_active_pane(w);
        if active.is_null() {
            return String::new();
        }

        let cmd =
            CString::new(cmd_stringify_argv((*active).argc, (*active).argv)).unwrap();
        if !cmd.is_empty() {
            parse_window_name(cmd.as_ptr().cast())
        } else {
            let shell_c = (*active).shell.as_deref()
                .and_then(|p| std::ffi::CString::new(p.to_string_lossy().as_bytes()).ok());
            match shell_c {
                Some(c) => parse_window_name(c.as_ptr().cast()),
                None => String::new(),
            }
        }
    }
}

unsafe fn format_window_name(w: *mut window) -> *const u8 {
    unsafe {
        let ft = format_create(
            null_mut(),
            null_mut(),
            (FORMAT_WINDOW | (*w).id) as i32,
            format_flags::empty(),
        );
        format_defaults_window(ft, w);
        format_defaults_pane(ft, window_active_pane(w));

        let fmt = options_get_string_((*w).options, "automatic-rename-format");
        let name = format_expand(ft, fmt);

        format_free(ft);
        name
    }
}

/// Extracts a clean program name from a command string.
///
/// Processing steps:
/// 1. Strip leading/trailing double quotes
/// 2. Strip `"exec "` prefix
/// 3. Skip leading spaces and dashes (login shell convention: `-bash`)
/// 4. Take only the first word (up to first space)
/// 5. Strip trailing non-alphanumeric, non-punctuation bytes
/// 6. If the result starts with `/`, take the basename
pub unsafe fn parse_window_name(in_: *const u8) -> String {
    unsafe {
        let sizeof_exec: usize = 6; // sizeof "exec "
        let copy: *mut u8 = xstrdup(in_).cast().as_ptr();
        let mut name = copy;
        if *name == b'"' {
            name = name.wrapping_add(1);
        }
        *name.add(strcspn(name, c!("\""))) = b'\0';

        if strncmp(name, c!("exec "), sizeof_exec - 1) == 0 {
            name = name.wrapping_add(sizeof_exec - 1);
        }

        while *name == b' ' || *name == b'-' {
            name = name.wrapping_add(1);
        }

        let mut ptr = strchr(name, b' ' as _);
        if !ptr.is_null() {
            *ptr = b'\0' as _;
        }

        if *name != b'\0' {
            ptr = name.add(strlen(name) - 1);
            while ptr > name
                && !(*ptr as u8).is_ascii_alphanumeric()
                && !(*ptr as u8).is_ascii_punctuation()
            {
                *ptr = b'\0';
                ptr = ptr.wrapping_sub(1);
            }
        }

        let tmp = if *name == b'/' {
            basename(cstr_to_str(name)).to_string()
        } else {
            cstr_to_str(name).to_string()
        };
        free(copy as _);
        tmp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: call parse_window_name with a Rust string.
    fn parse(input: &str) -> String {
        let c = CString::new(input).unwrap();
        unsafe { parse_window_name(c.as_ptr().cast()) }
    }

    // ---------------------------------------------------------------
    // Basic command names
    // ---------------------------------------------------------------

    #[test]
    fn simple_command() {
        assert_eq!(parse("vim"), "vim");
    }

    #[test]
    fn command_with_args() {
        assert_eq!(parse("vim foo.txt"), "vim");
    }

    #[test]
    fn empty_input() {
        assert_eq!(parse(""), "");
    }

    // ---------------------------------------------------------------
    // Quote stripping
    // ---------------------------------------------------------------

    #[test]
    fn strips_leading_quote() {
        assert_eq!(parse("\"bash\""), "bash");
    }

    #[test]
    fn strips_quotes_with_args() {
        assert_eq!(parse("\"vim\" foo.txt"), "vim");
    }

    // ---------------------------------------------------------------
    // "exec " prefix
    // ---------------------------------------------------------------

    #[test]
    fn strips_exec_prefix() {
        assert_eq!(parse("exec bash"), "bash");
    }

    #[test]
    fn exec_with_quoted() {
        assert_eq!(parse("\"exec zsh\""), "zsh");
    }

    // ---------------------------------------------------------------
    // Login shell dashes
    // ---------------------------------------------------------------

    #[test]
    fn strips_leading_dash() {
        assert_eq!(parse("-bash"), "bash");
    }

    #[test]
    fn strips_multiple_dashes() {
        assert_eq!(parse("--zsh"), "zsh");
    }

    #[test]
    fn strips_leading_spaces() {
        assert_eq!(parse("  vim"), "vim");
    }

    #[test]
    fn strips_mixed_space_dash() {
        assert_eq!(parse(" -bash"), "bash");
    }

    // ---------------------------------------------------------------
    // Basename for absolute paths
    // ---------------------------------------------------------------

    #[test]
    fn basename_of_absolute_path() {
        assert_eq!(parse("/usr/bin/vim"), "vim");
    }

    #[test]
    fn basename_with_args() {
        assert_eq!(parse("/bin/bash --login"), "bash");
    }

    #[test]
    fn relative_path_not_basenamed() {
        // Relative paths are not stripped — only absolute paths starting with /
        assert_eq!(parse("./foo"), "./foo");
    }

    // ---------------------------------------------------------------
    // Combined transformations
    // ---------------------------------------------------------------

    #[test]
    fn exec_with_path() {
        assert_eq!(parse("exec /usr/bin/python3"), "python3");
    }

    #[test]
    fn quoted_exec_path_with_args() {
        assert_eq!(parse("\"exec /usr/local/bin/node\" server.js"), "node");
    }

    #[test]
    fn login_shell_path() {
        assert_eq!(parse("-/bin/zsh"), "zsh");
    }
}
