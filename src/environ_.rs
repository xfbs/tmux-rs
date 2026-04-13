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

/// An environment variable store, backed by a sorted `BTreeMap`.
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

/// Iterate all entries in sorted order.
pub fn environ_entries(env: &Environ) -> impl Iterator<Item = &EnvironEntry> {
    env.entries.values()
}

/// Copy all entries from `src` into `dst`, overwriting any existing entries
/// with the same name. Cleared entries (value = None) propagate as clears.
pub fn environ_copy(src: &Environ, dst: &mut Environ) {
    for entry in src.entries.values() {
        if let Some(ref value) = entry.value {
            environ_set_(dst, entry.name.as_str(), entry.flags, value.clone());
        } else {
            environ_clear(dst, entry.name.as_str());
        }
    }
}

/// Look up an environment variable by name. Returns `None` if not found.
pub fn environ_find<'a>(env: &'a Environ, name: &str) -> Option<&'a EnvironEntry> {
    env.entries.get(name)
}

/// Look up an environment variable by C string name. Returns `None` if not found.
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
pub unsafe fn environ_find_raw(env: &Environ, name: *const u8) -> Option<&EnvironEntry> {
    unsafe { environ_find(env, cstr_to_str(name)) }
}

macro_rules! environ_set {
   ($env:expr, $name:expr, $flags:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_set_(&mut *$env, cstr_to_str($name), $flags, format!($fmt $(, $args)*).into_bytes())
    };
}
pub(crate) use environ_set;

/// Set an environment variable. `value` is an owned byte vector.
pub fn environ_set_(env: &mut Environ, name: &str, flags: environ_flags, value: Vec<u8>) {
    if let Some(entry) = env.entries.get_mut(name) {
        entry.flags = flags;
        entry.value = Some(value);
    } else {
        env.entries.insert(
            name.to_string(),
            EnvironEntry {
                name: name.to_string(),
                value: Some(value),
                flags,
            },
        );
    }
}

/// Clear an environment variable (set value to `None`). This masks an
/// inherited variable — the entry exists but has no value.
pub fn environ_clear(env: &mut Environ, name: &str) {
    if let Some(entry) = env.entries.get_mut(name) {
        entry.value = None;
    } else {
        env.entries.insert(
            name.to_string(),
            EnvironEntry {
                name: name.to_string(),
                value: None,
                flags: environ_flags::empty(),
            },
        );
    }
}

/// Parse a `NAME=VALUE` C string and set the variable. Ignores strings
/// without `=`.
///
/// # Safety
/// `var` must be a valid NUL-terminated C string.
pub unsafe fn environ_put(env: &mut Environ, var: *const u8, flags: environ_flags) {
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

/// Remove an environment variable entirely. Unlike `environ_clear`, this
/// does not leave a masking entry.
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
pub unsafe fn environ_unset(env: &mut Environ, name: *const u8) {
    unsafe {
        let key = cstr_to_str(name);
        env.entries.remove(key);
    }
}

/// Update `dst` from `src` for variables matching the `update-environment`
/// option patterns (fnmatch globs).
///
/// # Safety
/// `oo` must be a valid pointer to an options struct.
pub unsafe fn environ_update(oo: *mut options, src: &Environ, dst: &mut Environ) {
    unsafe {
        let mut found;

        let o = options_get(&mut *oo, "update-environment");
        if o.is_null() {
            return;
        }
        for a in options_array_items(o) {
            let ov = options_array_item_value(a);
            found = false;
            for entry in src.entries.values() {
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
/// # Safety
/// Modifying the process environment is inherently unsafe when other threads
/// may be reading it. Callers must ensure no concurrent env access.
pub unsafe fn environ_push(env: &Environ) {
    for entry in env.entries.values() {
        if let Some(ref value) = entry.value
            && !entry.name.is_empty() && !entry.flags.intersects(ENVIRON_HIDDEN) {
                use std::ffi::OsStr;
                use std::os::unix::ffi::OsStrExt;
                unsafe {
                    std::env::set_var(OsStr::new(&entry.name), OsStr::from_bytes(value));
                }
            }
    }
}

macro_rules! environ_log {
   ($env:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_log_(&*$env, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use environ_log;

/// Log all non-empty environment variables at debug level.
pub fn environ_log_(env: &Environ, args: std::fmt::Arguments) {
    let prefix = args.to_string();

    for entry in env.entries.values() {
        if let Some(ref value) = entry.value
            && !entry.name.is_empty() {
                log_debug!(
                    "{}{}={}",
                    prefix,
                    entry.name,
                    String::from_utf8_lossy(value),
                );
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
            let env = &mut *environ_create().as_ptr();

            environ_set_(env, "FOO", environ_flags::empty(), b"bar".to_vec());

            let entry = environ_find(env, "FOO").unwrap();
            assert_eq!(entry.value.as_deref(), Some(b"bar".as_slice()));

            assert!(environ_find(env, "NOPE").is_none());

            environ_free(env);
        }
    }

    #[test]
    fn set_overwrites() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_set_(env, "KEY", environ_flags::empty(), b"first".to_vec());
            environ_set_(env, "KEY", environ_flags::empty(), b"second".to_vec());

            let entry = environ_find(env, "KEY").unwrap();
            assert_eq!(entry.value.as_deref(), Some(b"second".as_slice()));

            environ_free(env);
        }
    }

    #[test]
    fn unset_removes() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_set_(env, "DEL", environ_flags::empty(), b"val".to_vec());
            assert!(environ_find(env, "DEL").is_some());

            environ_unset(env, c!("DEL"));
            assert!(environ_find(env, "DEL").is_none());

            environ_free(env);
        }
    }

    #[test]
    fn unset_nonexistent_is_safe() {
        unsafe {
            let env = &mut *environ_create().as_ptr();
            environ_unset(env, c!("NOPE")); // should not crash
            environ_free(env);
        }
    }

    #[test]
    fn clear_sets_value_to_none() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_set_(env, "CLR", environ_flags::empty(), b"val".to_vec());
            environ_clear(env, "CLR");

            let entry = environ_find(env, "CLR").unwrap();
            assert!(entry.value.is_none(), "value should be None after clear");

            environ_free(env);
        }
    }

    #[test]
    fn clear_nonexistent_creates_entry() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_clear(env, "NEW");

            let entry = environ_find(env, "NEW").unwrap();
            assert!(entry.value.is_none());

            environ_free(env);
        }
    }

    #[test]
    fn put_parses_key_value() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_put(env, c!("MY_VAR=hello world"), environ_flags::empty());

            let entry = environ_find(env, "MY_VAR").unwrap();
            assert_eq!(entry.value.as_deref(), Some(b"hello world".as_slice()));

            environ_free(env);
        }
    }

    #[test]
    fn put_ignores_no_equals() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_put(env, c!("NOEQUALS"), environ_flags::empty());

            assert!(environ_find(env, "NOEQUALS").is_none(), "put without = should be ignored");

            environ_free(env);
        }
    }

    #[test]
    fn copy_duplicates_entries() {
        unsafe {
            let src = &mut *environ_create().as_ptr();
            let dst = &mut *environ_create().as_ptr();

            environ_set_(src, "A", environ_flags::empty(), b"1".to_vec());
            environ_set_(src, "B", environ_flags::empty(), b"2".to_vec());

            environ_copy(src, dst);

            let a = environ_find(dst, "A").unwrap();
            assert_eq!(a.value.as_deref(), Some(b"1".as_slice()));

            let b = environ_find(dst, "B").unwrap();
            assert_eq!(b.value.as_deref(), Some(b"2".as_slice()));

            environ_free(src);
            environ_free(dst);
        }
    }

    #[test]
    fn iteration_with_entries() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_set_(env, "X", environ_flags::empty(), b"1".to_vec());
            environ_set_(env, "Y", environ_flags::empty(), b"2".to_vec());
            environ_set_(env, "Z", environ_flags::empty(), b"3".to_vec());

            let entries: Vec<_> = environ_entries(env).collect();
            assert_eq!(entries.len(), 3);

            // BTreeMap iterates in sorted order
            assert_eq!(entries[0].name, "X");
            assert_eq!(entries[1].name, "Y");
            assert_eq!(entries[2].name, "Z");

            environ_free(env);
        }
    }

    #[test]
    fn hidden_flag_preserved() {
        unsafe {
            let env = &mut *environ_create().as_ptr();

            environ_set_(env, "SECRET", ENVIRON_HIDDEN, b"hidden_val".to_vec());

            let entry = environ_find(env, "SECRET").unwrap();
            assert!(entry.flags.intersects(ENVIRON_HIDDEN));
            assert_eq!(entry.value.as_deref(), Some(b"hidden_val".as_slice()));

            environ_free(env);
        }
    }
}

pub unsafe fn environ_for_session(s: *mut session, no_term: c_int) -> *mut Environ {
    let env: *mut Environ = environ_create().as_ptr();

    unsafe {
        environ_copy(&*GLOBAL_ENVIRON, &mut *env);
        if !s.is_null() {
            environ_copy(&*(*s).environ, &mut *env);
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
            environ_clear(&mut *env, "LISTEN_PID");
            environ_clear(&mut *env, "LISTEN_FDS");
            environ_clear(&mut *env, "LISTEN_FDNAMES");
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
