use crate::{xmalloc::xcalloc_, *};

use compat_rs::{
    RB_GENERATE,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_min, rb_next, rb_remove},
};
use libc::{fnmatch, setenv, strchr, strcmp, strcspn};

unsafe extern "C" {
    // pub fn environ_create() -> *mut environ;
    // pub unsafe fn environ_free(_: *mut environ);
    // pub fn environ_first(_: *mut environ) -> *mut environ_entry;
    // pub fn environ_next(_: *mut environ_entry) -> *mut environ_entry;
    // pub fn environ_copy(_: *mut environ, _: *mut environ);
    // pub fn environ_find(_: *mut environ, _: *const c_char) -> *mut environ_entry;
    // pub fn environ_set(_: *mut environ, _: *const c_char, _: c_int, _: *const c_char, ...);
    // pub fn environ_clear(_: *mut environ, _: *const c_char);
    // pub fn environ_put(_: *mut environ, _: *const c_char, _: c_int);
    // pub fn environ_unset(_: *mut environ, _: *const c_char);
    // pub fn environ_update(_: *mut options, _: *mut environ, _: *mut environ);
    // pub fn environ_push(_: *mut environ);
    // pub fn environ_log(_: *mut environ, _: *const c_char, ...);
    // pub fn environ_for_session(_: *mut session, _: c_int) -> *mut environ;
}

pub type environ = rb_head<environ_entry>;
RB_GENERATE!(environ, environ_entry, entry, environ_cmp);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_cmp(envent1: *const environ_entry, envent2: *const environ_entry) -> c_int {
    unsafe { strcmp(transmute_ptr((*envent1).name), transmute_ptr((*envent2).name)) }
}

#[unsafe(no_mangle)]
pub extern "C" fn environ_create() -> NonNull<environ> {
    unsafe {
        let env: NonNull<environ> = xcalloc_::<environ>(1);
        rb_init(env.as_ptr());
        env
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_free(env: *mut environ) {
    unsafe {
        for envent in rb_foreach(env).map(NonNull::as_ptr) {
            rb_remove(env, envent);
            free_(transmute_ptr((*envent).name));
            free_(transmute_ptr((*envent).value));
            free_(envent);
        }
        free_(env);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_first(env: *mut environ) -> *mut environ_entry { unsafe { rb_min(env) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_next(envent: *mut environ_entry) -> *mut environ_entry { unsafe { rb_next(envent) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_copy(srcenv: *mut environ, dstenv: *mut environ) {
    unsafe {
        for envent in rb_foreach(srcenv).map(NonNull::as_ptr) {
            if let Some(value) = (*envent).value {
                environ_set(
                    dstenv,
                    (*envent).name.unwrap().as_ptr(),
                    (*envent).flags,
                    c"%s".as_ptr(),
                    value.as_ptr(),
                );
            } else {
                environ_clear(dstenv, transmute_ptr((*envent).name));
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_find(env: *mut environ, name: *const c_char) -> *mut environ_entry {
    let mut envent: MaybeUninit<environ_entry> = MaybeUninit::uninit();
    let envent = envent.as_mut_ptr();

    unsafe {
        (*envent).name = NonNull::new(name.cast_mut());
        // std::ptr::write(&raw mut (*envent).name, name);
    }

    unsafe { rb_find(env, envent) }
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
            free_(transmute_ptr((*envent).value));
            let mut ap = args.clone();
            xvasprintf(&raw mut (*envent).value as _, fmt, ap.as_va_list());
        } else {
            envent = xmalloc_::<environ_entry>().as_ptr();
            (*envent).name = Some(xstrdup(name).cast());
            (*envent).flags = flags;
            let mut ap = args.clone();
            xvasprintf(&raw mut (*envent).value as _, fmt, ap.as_va_list());
            rb_insert(env, envent);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_clear(env: *mut environ, name: *const c_char) {
    unsafe {
        let mut envent = environ_find(env, name);
        if !envent.is_null() {
            free_(transmute_ptr((*envent).value));
            (*envent).value = None;
        } else {
            envent = xmalloc_::<environ_entry>().as_ptr();
            (*envent).name = Some(xstrdup(name).cast());
            (*envent).flags = 0;
            (*envent).value = None;
            rb_insert(env, envent);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_put(env: *mut environ, var: *const c_char, flags: c_int) {
    unsafe {
        let mut value = strchr(var, b'=' as c_int);
        if value.is_null() {
            return;
        }
        value = value.add(1);

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
        free_(transmute_ptr((*envent).name));
        free_(transmute_ptr((*envent).value));
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
            for envent in rb_foreach(src).map(NonNull::as_ptr) {
                if fnmatch((*ov).string, transmute_ptr((*envent).name), 0) == 0 {
                    environ_set(dst, transmute_ptr((*envent).name), 0, c"%s".as_ptr(), (*envent).value);
                    found = 1;
                }
            }
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
        for envent in rb_foreach(env).map(NonNull::as_ptr) {
            if !(*envent).value.is_none()
                && *(*envent).name.unwrap().as_ptr() != b'\0' as c_char
                && !(*envent).flags & ENVIRON_HIDDEN != 0
            {
                setenv(transmute_ptr((*envent).name), transmute_ptr((*envent).value), 1);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_log(env: *mut environ, fmt: *const c_char, mut args: ...) {
    unsafe {
        let mut prefix: *mut c_char = null_mut();

        vasprintf(&raw mut prefix, fmt, args.as_va_list());

        for envent in rb_foreach(env).map(NonNull::as_ptr) {
            if (!(*envent).value.is_none() && *(*envent).name.unwrap().as_ptr() != b'\0' as c_char) {
                log_debug!(
                    "{}{}={}",
                    _s(prefix),
                    _s(transmute_ptr((*envent).name)),
                    _s(transmute_ptr((*envent).value))
                );
            }
        }

        free_(prefix);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn environ_for_session(s: *mut session, no_TERM: c_int) -> *mut environ {
    let env: *mut environ = environ_create().as_ptr();

    unsafe {
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
            std::process::id() as c_long,
            idx,
        );

        env
    }
}
