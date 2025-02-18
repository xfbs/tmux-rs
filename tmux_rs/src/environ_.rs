use crate::xmalloc::xcalloc_;

use super::*;

use compat_rs::tree::{rb_find, rb_foreach, rb_foreach_safe, rb_init, rb_insert, rb_min, rb_next, rb_remove};
use libc::{fnmatch, getpid, setenv, strchr, strcmp, strcspn};

unsafe extern "C" {
    pub fn environ_create() -> *mut environ;
    pub fn environ_free(_: *mut environ);
    pub fn environ_first(_: *mut environ) -> *mut environ_entry;
    pub fn environ_next(_: *mut environ_entry) -> *mut environ_entry;
    pub fn environ_copy(_: *mut environ, _: *mut environ);
    pub fn environ_find(_: *mut environ, _: *const c_char) -> *mut environ_entry;
    pub fn environ_set(_: *mut environ, _: *const c_char, _: c_int, _: *const c_char, ...);
    pub fn environ_clear(_: *mut environ, _: *const c_char);
    pub fn environ_put(_: *mut environ, _: *const c_char, _: c_int);
    pub fn environ_unset(_: *mut environ, _: *const c_char);
    pub fn environ_update(_: *mut options, _: *mut environ, _: *mut environ);
    pub fn environ_push(_: *mut environ);
    pub fn environ_log(_: *mut environ, _: *const c_char, ...);
    pub fn environ_for_session(_: *mut session, _: c_int) -> *mut environ;
}

pub type environ = rb_head<environ_entry>;

/*

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_cmp(envent1: *const environ_entry, envent2: *const environ_entry) -> c_int {
    unsafe { strcmp((*envent1).name, (*envent2).name) }
}

#[unsafe(no_mangle)]
pub extern "C" fn environ_create() -> *mut environ {
    unsafe {
        let mut env: *mut environ = xcalloc_::<environ>(1).as_ptr();
        rb_init(env);
        env
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_free(env: *mut environ) {
    unsafe {
        rb_foreach_safe(env, |envent| {
            // eprintln!("{:?} {:?}", envent, (*envent).entry);

            rb_remove(env, envent);
            free_((*envent).name);
            free_((*envent).value);
            free_(envent);
            ControlFlow::Continue::<(), ()>(())
        });
        free_(env);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_first(env: *mut environ) -> *mut environ_entry {
    unsafe { rb_min(env) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_next(envent: *mut environ_entry) -> *mut environ_entry {
    unsafe { rb_next(envent) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_copy(srcenv: *mut environ, dstenv: *mut environ) {
    unsafe {
        rb_foreach(srcenv, |envent| {
            if (*envent).value.is_null() {
                environ_clear(dstenv, (*envent).name);
            } else {
                environ_set(dstenv, (*envent).name, (*envent).flags, c"%s".as_ptr(), (*envent).value);
            }
            ControlFlow::Continue::<(), ()>(())
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_find(env: *mut environ, name: *const c_char) -> *mut environ_entry {
    unsafe {
        let mut envent = environ_entry {
            name: name as _,
            ..zeroed()
        };

        rb_find(env, &raw mut envent)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_set(
    env: *mut environ,
    name: *const c_char,
    flags: c_int,
    fmt: *const c_char,
    args: ...
) {
    unsafe {
        let mut envent = environ_find(env, name);
        if !envent.is_null() {
            (*envent).flags = flags;
            free_((*envent).value);
            let mut ap = args.clone();
            xvasprintf(&raw mut (*envent).value, fmt, ap.as_va_list());
        } else {
            envent = xmalloc_::<environ_entry>().as_ptr();
            (*envent).name = xstrdup(name).cast().as_ptr();
            (*envent).flags = flags;
            let mut ap = args.clone();
            xvasprintf(&raw mut (*envent).value, fmt, ap.as_va_list());
            rb_insert(env, envent);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_clear(env: *mut environ, name: *const c_char) {
    unsafe {
        let mut envent = environ_find(env, name);
        if !envent.is_null() {
            free_((*envent).value);
            (*envent).value = null_mut();
        } else {
            envent = xmalloc_::<environ_entry>().as_ptr();
            (*envent).name = xstrdup(name).cast().as_ptr();
            (*envent).flags = 0;
            (*envent).value = null_mut();
            rb_insert(env, envent);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_put(env: *mut environ, var: *const c_char, flags: c_int) {
    unsafe {
        let mut value = strchr(var, b'=' as _);
        if value.is_null() {
            return;
        }
        value = value.wrapping_add(1);

        let mut name: *mut c_char = xstrdup(var).cast().as_ptr();
        *name.add(strcspn(name, c"=".as_ptr())) = b'\0' as c_char;

        environ_set(env, name, flags, c"%s".as_ptr(), value);
        free_(name);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_unset(env: *mut environ, name: *const c_char) {
    unsafe {
        let mut envent = environ_find(env, name);
        if envent.is_null() {
            return;
        }
        rb_remove(env, envent);
        free_((*envent).name);
        free_((*envent).value);
        free_(envent);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_update(oo: *mut options, src: *mut environ, dst: *mut environ) {
    unsafe {
        let mut found: i32 = 0;

        let o = options_get(oo, c"update-environment".as_ptr());
        if o.is_null() {
            return;
        }
        let mut a = options_array_first(o);
        while !a.is_null() {
            let ov = options_array_item_value(a);
            found = 0;
            rb_foreach_safe(src, |envent| {
                if fnmatch((*ov).string, (*envent).name, 0) == 0 {
                    environ_set(dst, (*envent).name, 0, c"%s".as_ptr(), (*envent).value);
                    found = 1;
                }
                ControlFlow::<(), ()>::Continue(())
            });
            if found == 0 {
                environ_clear(dst, (*ov).string);
            }
            a = options_array_next(a);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_push(env: *mut environ) {
    unsafe {
        let mut envent: *mut environ_entry;

        environ = xcalloc_::<*mut c_char>(1).as_ptr();
        rb_foreach(env, |envent| {
            if !(*envent).value.is_null()
                && *(*envent).name != b'\0' as c_char
                && !(*envent).flags & ENVIRON_HIDDEN != 0
            {
                setenv((*envent).name, (*envent).value, 1);
            }
            ControlFlow::<(), ()>::Continue(())
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_log(env: *mut environ, fmt: *const c_char, mut args: ...) {
    unsafe {
        let mut prefix: *mut c_char = null_mut();

        vasprintf(&raw mut prefix, fmt, args.as_va_list());

        rb_foreach(env, |envent| {
            if (!(*envent).value.is_null() && *(*envent).name != b'\0' as c_char) {
                log_debug(c"%s%s=%s".as_ptr(), prefix, (*envent).name, (*envent).value);
            }
            ControlFlow::<(), ()>::Continue(())
        });

        free_(prefix);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_for_session(s: *mut session, no_TERM: c_int) -> *mut environ {
    unsafe {
        let mut env: *mut environ = null_mut();

        let mut env = environ_create();
        environ_copy(global_environ, env);
        if !s.is_null() {
            environ_copy((*s).environ, env);
        }

        if no_TERM == 0 {
            let value = options_get_string(global_options, c"default-terminal".as_ptr());
            environ_set(env, c"TERM".as_ptr(), 0, c"%s".as_ptr(), value);
            environ_set(env, c"TERM_PROGRAM".as_ptr(), 0, c"%s".as_ptr(), c"tmux".as_ptr());
            environ_set(env, c"TERM_PROGRAM_VERSION".as_ptr(), 0, c"%s".as_ptr(), getversion());
        }

        #[cfg(feature = "systemd")]
        {
            environ_clear(env, c"LISTEN_PID".as_ptr());
            environ_clear(env, c"LISTEN_FDS".as_ptr());
            environ_clear(env, c"LISTEN_FDNAMES".as_ptr());
        }

        let idx = if !s.is_null() { (*s).id as i32 } else { -1 };

        environ_set(
            env,
            c"TMUX".as_ptr(),
            0,
            c"%s,%ld,%d".as_ptr(),
            socket_path,
            getpid() as c_long,
            idx,
        );

        env
    }
}

*/
