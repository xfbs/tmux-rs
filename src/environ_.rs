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

use crate::*;
use crate::options_::*;

/// An environment variable store, backed by a sorted BTreeMap.
///
/// Replaces the previous intrusive RB-tree implementation. The keys are
/// owned C strings (name), values are optional C strings (None = "cleared"
/// entry that masks an inherited variable).
pub struct environ {
    entries: BTreeMap<String, environ_entry>,
}

pub fn environ_create() -> NonNull<environ> {
    let env = Box::new(environ {
        entries: BTreeMap::new(),
    });
    unsafe { NonNull::new_unchecked(Box::into_raw(env)) }
}

pub unsafe fn environ_free(env: *mut environ) {
    unsafe {
        for entry in (*env).entries.values() {
            free_(transmute_ptr(entry.name));
            free_(transmute_ptr(entry.value));
        }
        drop(Box::from_raw(env));
    }
}

/// Collect all entry pointers in sorted order. Callers that used
/// `environ_first`/`environ_next` should use this instead.
pub unsafe fn environ_entries(env: *mut environ) -> Vec<*mut environ_entry> {
    unsafe {
        (*env)
            .entries
            .values_mut()
            .map(|e| e as *mut environ_entry)
            .collect()
    }
}



pub unsafe fn environ_copy(srcenv: *mut environ, dstenv: *mut environ) {
    unsafe {
        for entry in (*srcenv).entries.values() {
            if let Some(value) = entry.value {
                environ_set!(
                    dstenv,
                    entry.name.unwrap().as_ptr(),
                    entry.flags,
                    "{}",
                    _s(value.as_ptr()),
                );
            } else {
                environ_clear(dstenv, transmute_ptr(entry.name));
            }
        }
    }
}

pub unsafe fn environ_find(env: *mut environ, name: *const u8) -> *mut environ_entry {
    unsafe {
        let key = cstr_to_str(name);
        (*env)
            .entries
            .get_mut(key)
            .map_or(null_mut(), |e| e as *mut environ_entry)
    }
}

macro_rules! environ_set {
   ($env:expr, $name:expr, $flags:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::environ_::environ_set_($env, $name, $flags, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use environ_set;
pub unsafe fn environ_set_(
    env: *mut environ,
    name: *const u8,
    flags: environ_flags,
    args: std::fmt::Arguments,
) {
    unsafe {
        let key = cstr_to_str(name).to_string();
        let mut s = args.to_string();
        s.push('\0');
        let s = NonNull::new(s.leak().as_mut_ptr().cast());

        if let Some(entry) = (*env).entries.get_mut(&key) {
            entry.flags = flags;
            free_(transmute_ptr(entry.value));
            entry.value = s;
        } else {
            (*env).entries.insert(
                key,
                environ_entry {
                    name: Some(xstrdup(name).cast()),
                    value: s,
                    flags,
                },
            );
        }
    }
}

pub unsafe fn environ_clear(env: *mut environ, name: *const u8) {
    unsafe {
        let key = cstr_to_str(name).to_string();

        if let Some(entry) = (*env).entries.get_mut(&key) {
            free_(transmute_ptr(entry.value));
            entry.value = None;
        } else {
            (*env).entries.insert(
                key,
                environ_entry {
                    name: Some(xstrdup(name).cast()),
                    value: None,
                    flags: environ_flags::empty(),
                },
            );
        }
    }
}

pub unsafe fn environ_put(env: *mut environ, var: *const u8, flags: environ_flags) {
    unsafe {
        let mut value = libc::strchr(var, b'=' as c_int);
        if value.is_null() {
            return;
        }
        value = value.add(1);

        let name: *mut u8 = xstrdup(var).cast().as_ptr();
        *name.add(libc::strcspn(name, c!("="))) = b'\0';

        environ_set!(env, name, flags, "{}", _s(value));
        free_(name);
    }
}

pub unsafe fn environ_unset(env: *mut environ, name: *const u8) {
    unsafe {
        let key = cstr_to_str(name);
        if let Some(entry) = (*env).entries.remove(key) {
            free_(transmute_ptr(entry.name));
            free_(transmute_ptr(entry.value));
        }
    }
}

pub unsafe fn environ_update(oo: *mut options, src: *mut environ, dst: *mut environ) {
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
                if libc::fnmatch((*ov).string, transmute_ptr(entry.name), 0) == 0 {
                    environ_set!(
                        dst,
                        transmute_ptr(entry.name),
                        environ_flags::empty(),
                        "{}",
                        _s(transmute_ptr(entry.value)),
                    );
                    found = true;
                }
            }
            if !found {
                environ_clear(dst, (*ov).string);
            }
        }
    }
}

pub unsafe fn environ_push(env: *mut environ) {
    unsafe {
        for entry in (*env).entries.values() {
            if entry.value.is_some()
                && *entry.name.unwrap().as_ptr() != b'\0'
                && !entry.flags.intersects(ENVIRON_HIDDEN)
            {
                std::env::set_var(
                    cstr_to_str(transmute_ptr(entry.name)),
                    cstr_to_str(transmute_ptr(entry.value)),
                );
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

pub unsafe fn environ_log_(env: *mut environ, args: std::fmt::Arguments) {
    unsafe {
        let prefix = args.to_string();

        for entry in (*env).entries.values() {
            if entry.value.is_some() && *entry.name.unwrap().as_ptr() != b'\0' {
                log_debug!(
                    "{}{}={}",
                    prefix,
                    _s(transmute_ptr(entry.name)),
                    _s(transmute_ptr(entry.value))
                );
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
            assert_eq!(_s(transmute_ptr((*entry).value)).to_string(), "bar");

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
            assert_eq!(_s(transmute_ptr((*entry).value)).to_string(), "second");

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
            environ_clear(env, c!("CLR"));

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

            environ_clear(env, c!("NEW"));

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
            assert_eq!(_s(transmute_ptr((*entry).value)).to_string(), "hello world");

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
            assert_eq!(_s(transmute_ptr((*a).value)).to_string(), "1");

            let b = environ_find(dst, c!("B"));
            assert!(!b.is_null());
            assert_eq!(_s(transmute_ptr((*b).value)).to_string(), "2");

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
            assert_eq!(_s(transmute_ptr((*entries[0]).name)).to_string(), "X");
            assert_eq!(_s(transmute_ptr((*entries[1]).name)).to_string(), "Y");
            assert_eq!(_s(transmute_ptr((*entries[2]).name)).to_string(), "Z");

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
            assert_eq!(_s(transmute_ptr((*entry).value)).to_string(), "hidden_val");

            environ_free(env);
        }
    }
}

pub unsafe fn environ_for_session(s: *mut session, no_term: c_int) -> *mut environ {
    let env: *mut environ = environ_create().as_ptr();

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
            environ_clear(env, c!("LISTEN_PID"));
            environ_clear(env, c!("LISTEN_FDS"));
            environ_clear(env, c!("LISTEN_FDNAMES"));
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
