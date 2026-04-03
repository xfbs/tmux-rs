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
use std::collections::BTreeMap;
use std::ffi::CString;

use crate::*;
use crate::options_::*;

/// An environment variable store, backed by a sorted BTreeMap.
///
/// Each entry has a name (ASCII key) and an optional value. Entries with
/// `value = None` are "cleared" — they mask an inherited variable (like
/// `unset`). Values are arbitrary byte strings since POSIX does not require
/// environment variable values to be valid UTF-8.
pub struct Environ {
    entries: BTreeMap<String, EnvironEntry>,
}

pub fn environ_create() -> NonNull<Environ> {
    let env = Box::new(Environ {
        entries: BTreeMap::new(),
    });
    unsafe { NonNull::new_unchecked(Box::into_raw(env)) }
}

/// Free an environment and all its entries. Since `EnvironEntry` fields are
/// now owned Rust types (`String`, `Vec<u8>`), dropping the `Box` is sufficient.
pub unsafe fn environ_free(env: *mut Environ) {
    unsafe {
        drop(Box::from_raw(env));
    }
}

/// Collect all entry pointers in sorted order. Callers that used
/// `environ_first`/`environ_next` should use this instead.
pub unsafe fn environ_entries(env: *mut Environ) -> Vec<*mut EnvironEntry> {
    unsafe {
        (*env)
            .entries
            .values_mut()
            .map(|e| e as *mut EnvironEntry)
            .collect()
    }
}

pub unsafe fn environ_copy(srcenv: *mut Environ, dstenv: *mut Environ) {
    unsafe {
        for entry in (*srcenv).entries.values() {
            if let Some(ref value) = entry.value {
                environ_set_(
                    dstenv,
                    entry.name.as_str(),
                    entry.flags,
                    value.clone(),
                );
            } else {
                environ_clear(dstenv, entry.name.as_str());
            }
        }
    }
}

pub unsafe fn environ_find(env: *mut Environ, name: *const u8) -> *mut EnvironEntry {
    unsafe {
        let key = cstr_to_str(name);
        (*env)
            .entries
            .get_mut(key)
            .map_or(null_mut(), |e| e as *mut EnvironEntry)
    }
}

macro_rules! environ_set {
   ($env:expr, $name:expr, $flags:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_set_($env, cstr_to_str($name), $flags, format!($fmt $(, $args)*).into_bytes())
    };
}
pub(crate) use environ_set;

/// Set an environment variable. `value` is an owned byte vector.
pub unsafe fn environ_set_(
    env: *mut Environ,
    name: &str,
    flags: environ_flags,
    value: Vec<u8>,
) {
    unsafe {
        if let Some(entry) = (*env).entries.get_mut(name) {
            entry.flags = flags;
            entry.value = Some(value);
        } else {
            (*env).entries.insert(
                name.to_string(),
                EnvironEntry {
                    name: name.to_string(),
                    value: Some(value),
                    flags,
                },
            );
        }
    }
}

/// Clear an environment variable (set value to `None`). This masks an
/// inherited variable — the entry exists but has no value.
pub unsafe fn environ_clear(env: *mut Environ, name: &str) {
    unsafe {
        if let Some(entry) = (*env).entries.get_mut(name) {
            entry.value = None;
        } else {
            (*env).entries.insert(
                name.to_string(),
                EnvironEntry {
                    name: name.to_string(),
                    value: None,
                    flags: environ_flags::empty(),
                },
            );
        }
    }
}

/// Parse a `NAME=VALUE` string and set the variable. Ignores strings
/// without `=`.
pub unsafe fn environ_put(env: *mut Environ, var: *const u8, flags: environ_flags) {
    unsafe {
        let var_bytes = std::ffi::CStr::from_ptr(var.cast()).to_bytes();
        let Some(eq_pos) = var_bytes.iter().position(|&b| b == b'=') else {
            return;
        };
        let name = std::str::from_utf8(&var_bytes[..eq_pos]).expect("env var name not UTF-8");
        let value = var_bytes[eq_pos + 1..].to_vec();
        environ_set_(env, name, flags, value);
    }
}

pub unsafe fn environ_unset(env: *mut Environ, name: *const u8) {
    unsafe {
        let key = cstr_to_str(name);
        (*env).entries.remove(key);
    }
}

pub unsafe fn environ_update(oo: *mut options, src: *mut Environ, dst: *mut Environ) {
    unsafe {
        let mut found;

        let o = options_get(&mut *oo, "update-environment");
        if o.is_null() {
            return;
        }
        for a in options_array_items(o) {
            let ov = options_array_item_value(a);
            found = false;
            for entry in (*src).entries.values() {
                let c_name =
                    CString::new(entry.name.as_str()).unwrap_or_else(|_| CString::default());
                if libc::fnmatch((*ov).string, c_name.as_ptr().cast(), 0) == 0 {
                    if let Some(ref value) = entry.value {
                        environ_set_(dst, entry.name.as_str(), environ_flags::empty(), value.clone());
                    }
                    found = true;
                }
            }
            if !found {
                environ_clear(dst, cstr_to_str((*ov).string));
            }
        }
    }
}

/// Push all non-hidden, non-empty environment variables into the process
/// environment via `std::env::set_var`.
pub unsafe fn environ_push(env: *mut Environ) {
    unsafe {
        for entry in (*env).entries.values() {
            if let Some(ref value) = entry.value {
                if !entry.name.is_empty() && !entry.flags.intersects(ENVIRON_HIDDEN) {
                    use std::ffi::OsStr;
                    use std::os::unix::ffi::OsStrExt;
                    std::env::set_var(
                        OsStr::new(&entry.name),
                        OsStr::from_bytes(value),
                    );
                }
            }
        }
    }
}

macro_rules! environ_log {
   ($env:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_log_($env, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use environ_log;

pub unsafe fn environ_log_(env: *mut Environ, args: std::fmt::Arguments) {
    unsafe {
        let prefix = args.to_string();

        for entry in (*env).entries.values() {
            if let Some(ref value) = entry.value {
                if !entry.name.is_empty() {
                    log_debug!(
                        "{}{}={}",
                        prefix,
                        entry.name,
                        String::from_utf8_lossy(value),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_free() {
        unsafe {
            let env = environ_create();
            environ_free(env.as_ptr());
        }
    }

    #[test]
    fn set_and_find() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_set!(env, c!("FOO"), environ_flags::empty(), "{}", "bar");

            let entry = environ_find(env, c!("FOO"));
            assert!(!entry.is_null());
            assert_eq!((*entry).value.as_deref(), Some(b"bar".as_slice()));

            // Non-existent key
            let missing = environ_find(env, c!("NOPE"));
            assert!(missing.is_null());

            environ_free(env);
        }
    }

    #[test]
    fn set_overwrites() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_set!(env, c!("KEY"), environ_flags::empty(), "{}", "first");
            environ_set!(env, c!("KEY"), environ_flags::empty(), "{}", "second");

            let entry = environ_find(env, c!("KEY"));
            assert!(!entry.is_null());
            assert_eq!((*entry).value.as_deref(), Some(b"second".as_slice()));

            environ_free(env);
        }
    }

    #[test]
    fn unset_removes() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_set!(env, c!("DEL"), environ_flags::empty(), "{}", "val");
            assert!(!environ_find(env, c!("DEL")).is_null());

            environ_unset(env, c!("DEL"));
            assert!(environ_find(env, c!("DEL")).is_null());

            environ_free(env);
        }
    }

    #[test]
    fn unset_nonexistent_is_safe() {
        unsafe {
            let env = environ_create().as_ptr();
            environ_unset(env, c!("NOPE")); // should not crash
            environ_free(env);
        }
    }

    #[test]
    fn clear_sets_value_to_none() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_set!(env, c!("CLR"), environ_flags::empty(), "{}", "val");
            environ_clear(env, cstr_to_str(c!("CLR")));

            let entry = environ_find(env, c!("CLR"));
            assert!(!entry.is_null());
            assert!((*entry).value.is_none(), "value should be None after clear");

            environ_free(env);
        }
    }

    #[test]
    fn clear_nonexistent_creates_entry() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_clear(env, cstr_to_str(c!("NEW")));

            let entry = environ_find(env, c!("NEW"));
            assert!(!entry.is_null(), "clear should create entry");
            assert!((*entry).value.is_none());

            environ_free(env);
        }
    }

    #[test]
    fn put_parses_key_value() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_put(env, c!("MY_VAR=hello world"), environ_flags::empty());

            let entry = environ_find(env, c!("MY_VAR"));
            assert!(!entry.is_null());
            assert_eq!((*entry).value.as_deref(), Some(b"hello world".as_slice()));

            environ_free(env);
        }
    }

    #[test]
    fn put_ignores_no_equals() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_put(env, c!("NOEQUALS"), environ_flags::empty());

            let entry = environ_find(env, c!("NOEQUALS"));
            assert!(entry.is_null(), "put without = should be ignored");

            environ_free(env);
        }
    }

    #[test]
    fn copy_duplicates_entries() {
        unsafe {
            let src = environ_create().as_ptr();
            let dst = environ_create().as_ptr();

            environ_set!(src, c!("A"), environ_flags::empty(), "{}", "1");
            environ_set!(src, c!("B"), environ_flags::empty(), "{}", "2");

            environ_copy(src, dst);

            let a = environ_find(dst, c!("A"));
            assert!(!a.is_null());
            assert_eq!((*a).value.as_deref(), Some(b"1".as_slice()));

            let b = environ_find(dst, c!("B"));
            assert!(!b.is_null());
            assert_eq!((*b).value.as_deref(), Some(b"2".as_slice()));

            environ_free(src);
            environ_free(dst);
        }
    }

    #[test]
    fn iteration_with_entries() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_set!(env, c!("X"), environ_flags::empty(), "{}", "1");
            environ_set!(env, c!("Y"), environ_flags::empty(), "{}", "2");
            environ_set!(env, c!("Z"), environ_flags::empty(), "{}", "3");

            let entries = environ_entries(env);
            assert_eq!(entries.len(), 3);

            // BTreeMap iterates in sorted order
            assert_eq!((*entries[0]).name, "X");
            assert_eq!((*entries[1]).name, "Y");
            assert_eq!((*entries[2]).name, "Z");

            environ_free(env);
        }
    }

    #[test]
    fn hidden_flag_preserved() {
        unsafe {
            let env = environ_create().as_ptr();

            environ_set!(env, c!("SECRET"), ENVIRON_HIDDEN, "{}", "hidden_val");

            let entry = environ_find(env, c!("SECRET"));
            assert!(!entry.is_null());
            assert!((*entry).flags.intersects(ENVIRON_HIDDEN));
            assert_eq!((*entry).value.as_deref(), Some(b"hidden_val".as_slice()));

            environ_free(env);
        }
    }
}

pub unsafe fn environ_for_session(s: *mut session, no_term: c_int) -> *mut Environ {
    let env: *mut Environ = environ_create().as_ptr();

    unsafe {
        environ_copy(GLOBAL_ENVIRON, env);
        if !s.is_null() {
            environ_copy((*s).environ, env);
        }

        if no_term == 0 {
            let value = options_get_string_(GLOBAL_OPTIONS, "default-terminal");
            environ_set!(env, c!("TERM"), environ_flags::empty(), "{}", _s(value));
            environ_set!(
                env,
                c!("TERM_PROGRAM"),
                environ_flags::empty(),
                "{}",
                "tmux"
            );
            environ_set!(
                env,
                c!("TERM_PROGRAM_VERSION"),
                environ_flags::empty(),
                "{}",
                getversion()
            );
        }

        #[cfg(feature = "systemd")]
        {
            environ_clear(env, "LISTEN_PID");
            environ_clear(env, "LISTEN_FDS");
            environ_clear(env, "LISTEN_FDNAMES");
        }

        let idx = if !s.is_null() { (*s).id as i32 } else { -1 };

        environ_set!(
            env,
            c!("TMUX"),
            environ_flags::empty(),
            "{},{},{}",
            _s(SOCKET_PATH),
            std::process::id(),
            idx,
        );

        env
    }
}
