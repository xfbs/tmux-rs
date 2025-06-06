use crate::*;

use libc::{FNM_CASEFOLD, REG_NOSUB, ctime_r, getpwuid, getuid, ispunct, localtime_r, memcpy, regcomp, regex_t, regexec, regfree, strchr, strcmp, strcspn, strftime, strstr, strtod, tm};

use crate::{
    compat::{
        HOST_NAME_MAX, RB_GENERATE_STATIC,
        queue::tailq_empty,
        strlcat, strtonum,
        tree::{rb_find, rb_foreach, rb_init, rb_initializer, rb_insert, rb_max, rb_min, rb_remove},
    },
    xmalloc::{xreallocarray, xstrndup},
};

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct format_flags: i32 {
        const FORMAT_STATUS  = 1;
        const FORMAT_FORCE   = 2;
        const FORMAT_NOJOBS  = 4;
        const FORMAT_VERBOSE = 8;
    }
}

pub const FORMAT_NONE: i32 = 0;
pub const FORMAT_PANE: u32 = 0x80000000u32;
pub const FORMAT_WINDOW: u32 = 0x40000000u32;

pub type format_cb = Option<unsafe extern "C" fn(_: *mut format_tree) -> *mut c_void>;

// Entry in format job tree.
#[repr(C)]
pub struct format_job {
    pub client: *mut client,
    pub tag: u32,
    pub cmd: *mut c_char,
    pub expanded: *mut c_char,

    pub last: time_t,
    pub out: *mut c_char,
    pub updated: i32,

    pub job: *mut job,
    pub status: i32,

    pub entry: rb_entry<format_job>,
}

pub type format_job_tree = rb_head<format_job>;
#[unsafe(no_mangle)]
pub static mut format_jobs: format_job_tree = rb_initializer();
RB_GENERATE_STATIC!(format_job_tree, format_job, entry, format_job_cmp);

// Format job tree comparison function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_job_cmp(fj1: *const format_job, fj2: *const format_job) -> i32 {
    unsafe {
        if ((*fj1).tag < (*fj2).tag) {
            return -1;
        }
        if ((*fj1).tag > (*fj2).tag) {
            return 1;
        }
        strcmp((*fj1).cmd, (*fj2).cmd)
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct format_modifiers : i32 {
        const FORMAT_TIMESTRING = 0x1;
        const FORMAT_BASENAME   = 0x2;
        const FORMAT_DIRNAME    = 0x4;
        const FORMAT_QUOTE_SHELL  = 0x8;
        const FORMAT_LITERAL = 0x10;
        const FORMAT_EXPAND = 0x20;
        const FORMAT_EXPANDTIME = 0x40;
        const FORMAT_SESSIONS = 0x80;
        const FORMAT_WINDOWS = 0x100;
        const FORMAT_PANES = 0x200;
        const FORMAT_PRETTY = 0x400;
        const FORMAT_LENGTH = 0x800;
        const FORMAT_WIDTH = 0x1000;
        const FORMAT_QUOTE_STYLE = 0x2000;
        const FORMAT_WINDOW_NAME = 0x4000;
        const FORMAT_SESSION_NAME = 0x8000;
        const FORMAT_CHARACTER = 0x10000;
        const FORMAT_COLOUR = 0x20000;
        const FORMAT_CLIENTS = 0x40000;
    }
}

/// Limit on recursion.
const FORMAT_LOOP_LIMIT: i32 = 100;

/// Format expand flags.
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    pub struct format_expand_flags: i32 {
        const FORMAT_EXPAND_TIME = 0x1;
        const FORMAT_EXPAND_NOJOBS = 0x2;
    }
}

#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum format_type {
    FORMAT_TYPE_UNKNOWN,
    FORMAT_TYPE_SESSION,
    FORMAT_TYPE_WINDOW,
    FORMAT_TYPE_PANE,
}

// Entry in format tree.
#[repr(C)]
pub struct format_entry {
    pub key: *mut c_char,
    pub value: *mut c_char,
    pub time: time_t,
    pub cb: format_cb,
    pub entry: rb_entry<format_entry>,
}

#[repr(C)]
pub struct format_tree {
    pub type_: format_type,

    pub c: *mut client,
    pub s: *mut session,
    pub wl: *mut winlink,
    pub w: *mut window,
    pub wp: *mut window_pane,
    pub pb: *mut paste_buffer,

    pub item: *mut cmdq_item,
    pub client: *mut client,
    pub flags: format_flags,
    pub tag: u32,

    pub m: mouse_event,

    pub tree: format_entry_tree,
}
pub type format_entry_tree = rb_head<format_entry>;
RB_GENERATE_STATIC!(format_entry_tree, format_entry, entry, format_entry_cmp);

/// Format expand state.
#[repr(C)]
pub struct format_expand_state {
    pub ft: *mut format_tree,
    pub loop_: u32,
    pub time: time_t,
    pub tm: tm,
    pub flags: format_expand_flags,
}

/// Format modifier.
#[repr(C)]
pub struct format_modifier {
    pub modifier: [c_char; 3],
    pub size: u32,

    pub argv: *mut *mut c_char,
    pub argc: i32,
}

/// Format entry tree comparison function.
#[unsafe(no_mangle)]
unsafe extern "C" fn format_entry_cmp(fe1: *const format_entry, fe2: *const format_entry) -> i32 { unsafe { strcmp((*fe1).key, (*fe2).key) } }

/// Single-character uppercase aliases.
#[unsafe(no_mangle)]
static format_upper: [SyncCharPtr; 26] = const {
    const fn idx(c: char) -> usize { (c as u8 - b'A') as usize }
    let mut tmp = [SyncCharPtr::null(); 26];

    tmp[idx('D')] = SyncCharPtr::new(c"pane_id");
    tmp[idx('F')] = SyncCharPtr::new(c"window_flags");
    tmp[idx('H')] = SyncCharPtr::new(c"host");
    tmp[idx('I')] = SyncCharPtr::new(c"window_index");
    tmp[idx('P')] = SyncCharPtr::new(c"pane_index");
    tmp[idx('S')] = SyncCharPtr::new(c"session_name");
    tmp[idx('T')] = SyncCharPtr::new(c"pane_title");
    tmp[idx('W')] = SyncCharPtr::new(c"window_name");

    tmp
};

/// Single-character lowercase aliases.
#[unsafe(no_mangle)]
static format_lower: [SyncCharPtr; 26] = const {
    const fn idx(c: char) -> usize { (c as u8 - b'a') as usize }
    let mut tmp = [SyncCharPtr::null(); 26];
    tmp[idx('h')] = SyncCharPtr::new(c"host_short");
    tmp
};

/// Is logging enabled?
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_logging(ft: *mut format_tree) -> boolint { unsafe { log_get_level() != 0 || (*ft).flags.intersects(format_flags::FORMAT_VERBOSE) }.into() }

/// Log a message if verbose.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_log1(es: *mut format_expand_state, from: *const c_char, fmt: *const c_char, mut ap: ...) {
    unsafe {
        let ft: *mut format_tree = (*es).ft;
        let mut s = null_mut();
        let spaces = c"          ";

        if (!format_logging(ft)) {
            return;
        }

        xvasprintf(&raw mut s, fmt, ap.as_va_list());

        log_debug!("{}: {}", _s(from), _s(s));
        if !(*ft).item.is_null() && (*ft).flags.intersects(format_flags::FORMAT_VERBOSE) {
            cmdq_print((*ft).item, c"#%.*s%s".as_ptr(), (*es).loop_, spaces, s);
        }

        free(s as *mut c_void);
    }
}

// #define format_log(es, fmt, ...) format_log1(es, __func__, fmt, ##__VA_ARGS__)
// should make this support multiple arg lengths, but easier to just support what's needed
macro_rules! format_log {
    ($es:expr, $fmt:expr) => {
        format_log1($es, __func__!(), $fmt)
    };
    ($es:expr, $fmt:expr, $a1:expr) => {
        format_log1($es, __func__!(), $fmt, $a1)
    };
    ($es:expr, $fmt:expr, $a1:expr, $a2:expr) => {
        format_log1($es, __func__!(), $fmt, $a1, $a2)
    };
    ($es:expr, $fmt:expr, $a1:expr, $a2:expr, $a3:expr) => {
        format_log1($es, __func__!(), $fmt, $a1, $a2, $a3)
    };
}

/// Copy expand state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_copy_state(to: *mut format_expand_state, from: *mut format_expand_state, flags: format_expand_flags) {
    unsafe {
        (*to).ft = (*from).ft;
        (*to).loop_ = (*from).loop_;
        (*to).time = (*from).time;
        memcpy__(&raw mut (*to).tm, &raw const (*from).tm);
        (*to).flags = (*from).flags | flags;
    }
}

/// Format job update callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_job_update(job: *mut job) {
    unsafe {
        let fj = job_get_data(job) as *mut format_job;
        let evb: *mut evbuffer = (*job_get_event(job)).input;
        // char *line = NULL, *next;
        let mut line: *mut c_char = null_mut();

        while let Some(next) = NonNull::new(evbuffer_readline(evb)) {
            free(line.cast());
            line = next.as_ptr();
        }
        if line.is_null() {
            return;
        }
        (*fj).updated = 1;

        free((*fj).out.cast());
        (*fj).out = line;

        log_debug!("{}: {:p} {}: {}", function_name!(), fj, _s((*fj).cmd), _s((*fj).out));

        let t = libc::time(null_mut());
        if ((*fj).status != 0 && (*fj).last != t) {
            if (!(*fj).client.is_null()) {
                server_status_client((*fj).client);
            }
            (*fj).last = t;
        }
    }
}

// Format job complete callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_job_complete(job: *mut job) {
    unsafe {
        let mut fj = job_get_data(job) as *mut format_job;
        let mut evb: *mut evbuffer = (*job_get_event(job)).input;

        (*fj).job = null_mut();

        let mut buf: *mut c_char = null_mut();

        let mut line = evbuffer_readline(evb);
        if line.is_null() {
            let len = EVBUFFER_LENGTH(evb);
            buf = xmalloc(len + 1).as_ptr().cast();
            if (len != 0) {
                memcpy(buf.cast(), EVBUFFER_DATA(evb).cast(), len);
            }
            *buf.add(len) = b'\0' as c_char;
        } else {
            buf = line;
        }

        log_debug!("{}: {:p} {}: {}", function_name!(), fj, _s((*fj).cmd), _s(buf));

        if *buf != b'\0' as c_char || !(*fj).updated != 0 {
            free((*fj).out.cast());
            (*fj).out = buf;
        } else {
            free(buf.cast());
        }

        if (*fj).status != 0 {
            if (!(*fj).client.is_null()) {
                server_status_client((*fj).client);
            }
            (*fj).status = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_job_get(es: *mut format_expand_state, cmd: *mut c_char) -> *mut c_char {
    unsafe {
        let mut ft: *mut format_tree = (*es).ft;
        // format_job_tree *jobs;
        // format_job fj0, *fj;
        let mut fj0 = MaybeUninit::<format_job>::uninit();
        let mut fj1 = MaybeUninit::<format_job>::uninit();
        let mut fj0 = fj0.as_mut_ptr();
        let mut fj1 = fj1.as_mut_ptr();

        // time_t t;
        // char *expanded;
        // int force;

        let jobs = if ((*ft).client.is_null()) {
            &raw mut format_jobs
        } else if !(*(*ft).client).jobs.is_null() {
            (*(*ft).client).jobs
        } else {
            (*(*ft).client).jobs = xmalloc_().as_ptr();
            rb_init((*(*ft).client).jobs);
            (*(*ft).client).jobs
        };

        (*fj0).tag = (*ft).tag;
        (*fj0).cmd = cmd;
        let mut fj = rb_find(jobs, fj0);
        if fj.is_null() {
            fj = xcalloc1() as *mut format_job;
            (*fj).client = (*ft).client;
            (*fj).tag = (*ft).tag;
            (*fj).cmd = xstrdup(cmd).as_ptr();

            rb_insert(jobs, fj);
        }

        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();
        format_copy_state(next, es, format_expand_flags::FORMAT_EXPAND_NOJOBS);
        (*next).flags &= !format_expand_flags::FORMAT_EXPAND_TIME;

        let expanded = format_expand1(next, cmd);

        let mut force = if ((*fj).expanded.is_null() || strcmp(expanded, (*fj).expanded) != 0) {
            free((*fj).expanded.cast());
            (*fj).expanded = xstrdup(expanded).as_ptr();
            true
        } else {
            (*ft).flags.intersects(format_flags::FORMAT_FORCE)
        };

        let t = libc::time(null_mut());
        if (force && !(*fj).job.is_null()) {
            job_free((*fj).job);
        }
        if (force || ((*fj).job.is_null() && (*fj).last != t)) {
            (*fj).job = job_run(
                expanded,
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                server_client_get_cwd((*ft).client, null_mut()),
                Some(format_job_update),
                Some(format_job_complete),
                None,
                fj.cast(),
                JOB_NOWAIT,
                -1,
                -1,
            );
            if ((*fj).job.is_null()) {
                free((*fj).out.cast());
                xasprintf(&raw mut (*fj).out, c"<'%s' didn't start>".as_ptr(), (*fj).cmd);
            }
            (*fj).last = t;
            (*fj).updated = 0;
        } else if (!(*fj).job.is_null() && (t - (*fj).last) > 1 && (*fj).out.is_null()) {
            xasprintf(&raw mut (*fj).out, c"<'%s' not ready>".as_ptr(), (*fj).cmd);
        }
        free(expanded.cast());

        if (*ft).flags.intersects(format_flags::FORMAT_STATUS) {
            (*fj).status = 1;
        }
        if ((*fj).out.is_null()) {
            return xstrdup_(c"").as_ptr();
        }

        format_expand1(next, (*fj).out)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_job_tidy(jobs: *mut format_job_tree, force: i32) {
    unsafe {
        let now = libc::time(null_mut());
        for fj in rb_foreach(jobs) {
            let fj = fj.as_ptr();
            if (force == 0 && ((*fj).last > now || now - (*fj).last < 3600)) {
                continue;
            }
            rb_remove(jobs, fj);

            log_debug!("{}: {}", "format_job_tidy", _s((*fj).cmd));

            if (!(*fj).job.is_null()) {
                job_free((*fj).job);
            }

            free_((*fj).expanded);
            free_((*fj).cmd);
            free_((*fj).out);

            free_(fj);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_tidy_jobs() {
    unsafe {
        format_job_tidy(&raw mut format_jobs, 0);
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if !(*c).jobs.is_null() {
                format_job_tidy((*c).jobs, 0);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_lost_client(c: *mut client) {
    unsafe {
        if !(*c).jobs.is_null() {
            format_job_tidy((*c).jobs, 1);
        }
        free_((*c).jobs);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_printf(fmt: *const c_char, mut ap: ...) -> *mut c_char {
    unsafe {
        let mut s: *mut c_char = null_mut();
        xvasprintf(&raw mut s, fmt, ap.as_va_list());
        s
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_host(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut host = MaybeUninit::<[c_char; HOST_NAME_MAX + 1]>::uninit();

        if (libc::gethostname(host.as_mut_ptr().cast(), HOST_NAME_MAX + 1) != 0) {
            xstrdup_(c"").as_ptr().cast()
        } else {
            xstrdup(host.as_ptr().cast()).as_ptr().cast()
        }
    }
}

/// Callback for host_short.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_host_short(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut host = MaybeUninit::<[c_char; HOST_NAME_MAX + 1]>::uninit();

        if libc::gethostname(host.as_mut_ptr().cast(), HOST_NAME_MAX + 1) != 0 {
            return xstrdup_(c"").as_ptr().cast();
        }

        let mut cp = strchr(host.as_mut_ptr().cast(), b'.' as i32);
        if (!cp.is_null()) {
            *cp = b'\0' as c_char;
        }
        xstrdup(host.as_ptr().cast()).as_ptr().cast()
    }
}

/// Callback for pid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pid(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut value = null_mut();
        xasprintf(&raw mut value, c"%ld".as_ptr(), libc::getpid());
        value.cast()
    }
}

/// Callback for session_attached_list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_attached_list(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut s = (*ft).s;
        let mut value: *mut c_char = null_mut();

        if (s.is_null()) {
            return null_mut();
        }

        let buffer = evbuffer_new();
        if (buffer.is_null()) {
            fatalx(c"out of memory");
        }

        for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if ((*loop_).session == s) {
                if (EVBUFFER_LENGTH(buffer) > 0) {
                    evbuffer_add(buffer, c",".as_ptr().cast(), 1);
                }
                evbuffer_add_printf(buffer, c"%s".as_ptr(), (*loop_).name);
            }
        }

        let size = EVBUFFER_LENGTH(buffer);
        if (size != 0) {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for session_alerts.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_alerts(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut s: *mut session = (*ft).s;
        const sizeof_alerts: usize = 1024;
        const sizeof_tmp: usize = 16;
        let mut alerts = MaybeUninit::<[c_char; 1024]>::uninit();
        let mut alerts: *mut c_char = alerts.as_mut_ptr().cast();
        let mut tmp = MaybeUninit::<[c_char; 16]>::uninit();
        let mut tmp: *mut c_char = tmp.as_mut_ptr().cast();

        if (s.is_null()) {
            return null_mut();
        }

        *alerts = b'\0' as c_char;
        for wl in rb_foreach(&raw mut (*s).windows).map(NonNull::as_ptr) {
            if (((*wl).flags & WINLINK_ALERTFLAGS) == 0) {
                continue;
            }
            xsnprintf(tmp, sizeof_tmp, c"%u".as_ptr(), (*wl).idx);

            if (*alerts != b'\0' as c_char) {
                strlcat(alerts, c",".as_ptr(), sizeof_alerts);
            }
            strlcat(alerts, tmp, sizeof_alerts);
            if ((*wl).flags & WINLINK_ACTIVITY != 0) {
                strlcat(alerts, c"#".as_ptr(), sizeof_alerts);
            }
            if ((*wl).flags & WINLINK_BELL != 0) {
                strlcat(alerts, c"!".as_ptr(), sizeof_alerts);
            }
            if ((*wl).flags & WINLINK_SILENCE != 0) {
                strlcat(alerts, c"~".as_ptr(), sizeof_alerts);
            }
        }
        xstrdup(alerts).as_ptr().cast()
    }
}

/// Callback for session_stack.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_stack(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut s = (*ft).s;
        const sizeof_result: usize = 1024;
        const sizeof_tmp: usize = 16;

        let mut result = MaybeUninit::<[c_char; 1024]>::uninit();
        let mut result: *mut c_char = result.as_mut_ptr().cast();
        let mut tmp = MaybeUninit::<[c_char; 16]>::uninit();
        let mut tmp: *mut c_char = tmp.as_mut_ptr().cast();

        if (s.is_null()) {
            return null_mut();
        }

        xsnprintf(result, sizeof_result, c"%u".as_ptr(), (*(*s).curw).idx);
        for wl in tailq_foreach::<_, discr_sentry>(&raw mut (*s).lastw).map(NonNull::as_ptr) {
            xsnprintf(tmp, sizeof_tmp, c"%u".as_ptr(), (*wl).idx);

            if (*result != b'\0' as c_char) {
                strlcat(result, c",".as_ptr(), sizeof_result);
            }
            strlcat(result, tmp, sizeof_result);
        }
        xstrdup(result.cast()).as_ptr().cast()
    }
}

/// Callback for window_stack_index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_stack_index(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut value: *mut c_char = null_mut();

        if ((*ft).wl.is_null()) {
            return null_mut();
        }
        let mut s = (*(*ft).wl).session;

        let mut idx: u32 = 0;
        let mut wl = null_mut();
        for wl_ in tailq_foreach::<_, discr_sentry>(&raw mut (*s).lastw).map(NonNull::as_ptr) {
            wl = wl_;
            idx += 1;
            if wl == (*ft).wl {
                break;
            }
        }
        if wl.is_null() {
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        xasprintf(&raw mut value, c"%u".as_ptr(), idx);
        value.cast()
    }
}

/// Callback for window_linked_sessions_list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_linked_sessions_list(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut value = null_mut();

        if ((*ft).wl.is_null()) {
            return null_mut();
        }
        let mut w = (*(*ft).wl).window;

        let buffer = evbuffer_new();
        if (buffer.is_null()) {
            fatalx(c"out of memory");
        }

        for wl in tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            if (EVBUFFER_LENGTH(buffer) > 0) {
                evbuffer_add(buffer, c",".as_ptr().cast(), 1);
            }
            evbuffer_add_printf(buffer, c"%s".as_ptr(), (*(*wl).session).name);
        }

        let size = EVBUFFER_LENGTH(buffer);
        if size != 0 {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for window_active_sessions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_active_sessions(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if ((*ft).wl.is_null()) {
            return null_mut();
        }
        let w = (*(*ft).wl).window;

        let n = tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks).filter(|wl| (*(*wl.as_ptr()).session).curw == wl.as_ptr()).count() as u32;

        let mut value = null_mut();
        xasprintf(&raw mut value, c"%u".as_ptr(), n);
        value.cast()
    }
}

/// Callback for window_active_sessions_list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_active_sessions_list(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if ((*ft).wl.is_null()) {
            return null_mut();
        }
        let w = (*(*ft).wl).window;

        let buffer = evbuffer_new();
        if (buffer.is_null()) {
            fatalx(c"out of memory");
        }

        for wl in tailq_foreach::<_, discr_wentry>(&raw mut (*w).winlinks).map(NonNull::as_ptr) {
            if ((*(*wl).session).curw == wl) {
                if (EVBUFFER_LENGTH(buffer) > 0) {
                    evbuffer_add(buffer, c",".as_ptr().cast(), 1);
                }
                evbuffer_add_printf(buffer, c"%s".as_ptr(), (*(*wl).session).name);
            }
        }

        let size = EVBUFFER_LENGTH(buffer);
        let mut value = null_mut();
        if size != 0 {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for window_active_clients.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_active_clients(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if ((*ft).wl.is_null()) {
            return null_mut();
        }
        let w = (*(*ft).wl).window;

        let mut n = 0u32;
        for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            let mut client_session = (*loop_).session;
            if (client_session.is_null()) {
                continue;
            }

            if (w == (*(*client_session).curw).window) {
                n += 1;
            }
        }

        let mut value = null_mut();
        xasprintf(&raw mut value, c"%u".as_ptr(), n);
        value.cast()
    }
}

/// Callback for window_active_clients_list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_active_clients_list(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if ((*ft).wl.is_null()) {
            return null_mut();
        }
        let w = (*(*ft).wl).window;

        let buffer = evbuffer_new();
        if (buffer.is_null()) {
            fatalx(c"out of memory");
        }

        for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            let client_session = (*loop_).session;
            if (client_session.is_null()) {
                continue;
            }

            if (w == (*(*client_session).curw).window) {
                if (EVBUFFER_LENGTH(buffer) > 0) {
                    evbuffer_add(buffer, c",".as_ptr().cast(), 1);
                }
                evbuffer_add_printf(buffer, c"%s".as_ptr(), (*loop_).name);
            }
        }

        let mut value = null_mut();
        let mut size = EVBUFFER_LENGTH(buffer);
        if size != 0 {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for window_layout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_layout(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut w = (*ft).w;

        if (w.is_null()) {
            return null_mut();
        }

        if !(*w).saved_layout_root.is_null() {
            return layout_dump((*w).saved_layout_root).cast();
        }
        layout_dump((*w).layout_root).cast()
    }
}

/// Callback for window_visible_layout.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_visible_layout(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let w = (*ft).w;

        if (w.is_null()) {
            return null_mut();
        }

        layout_dump((*w).layout_root).cast()
    }
}
/// Callback for pane_start_command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_start_command(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return null_mut();
        }

        cmd_stringify_argv((*wp).argc, (*wp).argv).cast()
    }
}

/// Callback for pane_start_path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_start_path(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return null_mut();
        }

        if (*wp).cwd.is_null() {
            return xstrdup(c"".as_ptr()).as_ptr().cast();
        }
        xstrdup((*wp).cwd).as_ptr().cast()
    }
}

/// Callback for pane_current_command.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_current_command(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() || (*wp).shell.is_null() {
            return null_mut();
        }

        let mut cmd = osdep_get_name((*wp).fd, (*wp).tty.as_ptr());
        if cmd.is_null() || *cmd == b'\0' as c_char {
            free_(cmd);
            cmd = cmd_stringify_argv((*wp).argc, (*wp).argv);
            if cmd.is_null() || *cmd == b'\0' as c_char {
                free_(cmd);
                cmd = xstrdup((*wp).shell).as_ptr().cast();
            }
        }
        let value = parse_window_name(cmd);
        free_(cmd);
        value.cast()
    }
}

/// Callback for pane_current_path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_current_path(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return null_mut();
        }

        let cwd = osdep_get_cwd((*wp).fd);
        if cwd.is_null() {
            return null_mut();
        }
        xstrdup(cwd).as_ptr().cast()
    }
}

/// Callback for history_bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_history_bytes(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return null_mut();
        }

        let gd = (*wp).base.grid;
        let mut size: usize = 0;

        for i in 0..((*gd).hsize + (*gd).sy) {
            let gl = grid_get_line(gd, i);
            size += (*gl).cellsize as usize * std::mem::size_of::<grid_cell>();
            size += (*gl).extdsize as usize * std::mem::size_of::<grid_cell>();
        }
        size += ((*gd).hsize + (*gd).sy) as usize * std::mem::size_of::<grid_line>();

        let mut value = null_mut();
        xasprintf(&raw mut value, c"%zu".as_ptr(), size);
        value.cast()
    }
}

/// Callback for history_all_bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_history_all_bytes(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return null_mut();
        }

        let gd = (*wp).base.grid;
        let lines = (*gd).hsize + (*gd).sy;
        let mut cells = 0;
        let mut extended_cells = 0;

        for i in 0..lines {
            let gl = grid_get_line(gd, i);
            cells += (*gl).cellsize;
            extended_cells += (*gl).extdsize;
        }

        let mut value = null_mut();
        xasprintf(
            &raw mut value,
            c"%u,%zu,%u,%zu,%u,%zu".as_ptr(),
            lines,
            lines as usize * std::mem::size_of::<grid_line>(),
            cells,
            cells as usize * std::mem::size_of::<grid_cell>(),
            extended_cells,
            extended_cells as usize * std::mem::size_of::<grid_cell>(),
        );
        value.cast()
    }
}

/// Callback for pane_tabs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_tabs(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;

        if wp.is_null() {
            return null_mut();
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx(c"out of memory");
        }

        let mut first = true;
        for i in 0..(*(*wp).base.grid).sx {
            if !bit_test((*wp).base.tabs, i) {
                continue;
            }

            if !first {
                evbuffer_add(buffer, c",".as_ptr().cast(), 1);
            }
            evbuffer_add_printf(buffer, c"%u".as_ptr(), i);
            first = false;
        }

        let mut value = null_mut();
        let size = EVBUFFER_LENGTH(buffer);
        if size != 0 {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for pane_fg.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_fg(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        if wp.is_null() {
            return null_mut();
        }

        tty_default_colours(gc.as_mut_ptr(), wp);
        xstrdup(colour_tostring((*gc.as_ptr()).fg)).as_ptr().cast()
    }
}

/// Callback for pane_bg.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_bg(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        if wp.is_null() {
            return null_mut();
        }

        tty_default_colours(gc.as_mut_ptr(), wp);
        xstrdup(colour_tostring((*gc.as_ptr()).bg)).as_ptr().cast()
    }
}

/// Callback for session_group_list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_group_list(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let s = (*ft).s;
        if s.is_null() {
            return null_mut();
        }

        let sg = session_group_contains(s);
        if sg.is_null() {
            return null_mut();
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx(c"out of memory");
        }

        for loop_ in tailq_foreach(&raw mut (*sg).sessions).map(NonNull::as_ptr) {
            if EVBUFFER_LENGTH(buffer) > 0 {
                evbuffer_add(buffer, c",".as_ptr().cast(), 1);
            }
            evbuffer_add_printf(buffer, c"%s".as_ptr(), (*loop_).name);
        }

        let mut value = null_mut();
        let size = EVBUFFER_LENGTH(buffer);
        if size != 0 {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for session_group_attached_list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_group_attached_list(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let s = (*ft).s;
        if s.is_null() {
            return null_mut();
        }

        let sg = session_group_contains(s);
        if sg.is_null() {
            return null_mut();
        }

        let buffer = evbuffer_new();
        if buffer.is_null() {
            fatalx(c"out of memory");
        }

        let mut first = true;
        for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            let client_session = (*loop_).session;
            if client_session.is_null() {
                continue;
            }

            for session_loop in tailq_foreach(&raw mut (*sg).sessions).map(NonNull::as_ptr) {
                if session_loop == client_session {
                    if EVBUFFER_LENGTH(buffer) > 0 {
                        evbuffer_add(buffer, c",".as_ptr().cast(), 1);
                    }
                    evbuffer_add_printf(buffer, c"%s".as_ptr(), (*loop_).name);
                }
            }
        }

        let mut value = null_mut();
        let size = EVBUFFER_LENGTH(buffer);
        if size != 0 {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), size, EVBUFFER_DATA(buffer));
        }
        evbuffer_free(buffer);
        value.cast()
    }
}

/// Callback for pane_in_mode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_in_mode(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return null_mut();
        }

        let mut n = tailq_foreach(&raw mut (*wp).modes).count() as u32;

        let mut value = null_mut();
        xasprintf(&raw mut value, c"%u".as_ptr(), n);
        value.cast()
    }
}

/// Callback for pane_at_top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_at_top(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return null_mut();
        }

        let w = (*wp).window;
        let status = options_get_number((*w).options, c"pane-border-status".as_ptr());
        let flag = if status == pane_status::PANE_STATUS_TOP as i64 { (*wp).yoff == 1 } else { (*wp).yoff == 0 };

        let mut value = null_mut();
        xasprintf(&raw mut value, c"%d".as_ptr(), flag as i32);
        value.cast()
    }
}

/// Callback for pane_at_bottom.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_at_bottom(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return null_mut();
        }

        let w = (*wp).window;
        let status = options_get_number((*w).options, c"pane-border-status".as_ptr());
        let flag = if status == pane_status::PANE_STATUS_BOTTOM as i64 { (*wp).yoff + (*wp).sy == (*w).sy - 1 } else { (*wp).yoff + (*wp).sy == (*w).sy };

        let mut value = null_mut();
        xasprintf(&raw mut value, c"%d".as_ptr(), flag as i32);
        value.cast()
    }
}

/// Callback for cursor_character.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_cursor_character(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if wp.is_null() {
            return null_mut();
        }
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        grid_view_get_cell((*wp).base.grid, (*wp).base.cx, (*wp).base.cy, gc.as_mut_ptr());
        let mut value = null_mut();
        if !(*gc.as_ptr()).flags.intersects(grid_flag::PADDING) {
            xasprintf(&raw mut value, c"%.*s".as_ptr(), (*gc.as_ptr()).data.size as i32, (*gc.as_ptr()).data.data);
        }
        value.cast()
    }
}

/// Callback for mouse_word.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_word(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) else {
            return null_mut();
        };
        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) != 0 {
            return null_mut();
        }

        if !tailq_empty(&raw mut (*wp.as_ptr()).modes) {
            if window_pane_mode(wp.as_ptr()) != WINDOW_PANE_NO_MODE {
                return window_copy_get_word(wp.as_ptr(), x, y).cast();
            }
            return null_mut();
        }
        let gd = (*wp.as_ptr()).base.grid;
        format_grid_word(gd, x, (*gd).hsize + y).cast()
    }
}

/// Callback for mouse_hyperlink.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_hyperlink(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) else {
            return null_mut();
        };
        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) != 0 {
            return null_mut();
        }
        let gd = (*wp.as_ptr()).base.grid;
        format_grid_hyperlink(gd, x, (*gd).hsize + y, (*wp.as_ptr()).screen).cast()
    }
}

/// Callback for mouse_line.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_line(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) else {
            return null_mut();
        };
        let mut x = 0;
        let mut y = 0;
        if cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) != 0 {
            return null_mut();
        }

        if !tailq_empty(&raw mut (*wp.as_ptr()).modes) {
            if window_pane_mode(wp.as_ptr()) != WINDOW_PANE_NO_MODE {
                return window_copy_get_line(wp.as_ptr(), y).cast();
            }
            return null_mut();
        }
        let gd = (*wp.as_ptr()).base.grid;
        format_grid_line(gd, (*gd).hsize + y).cast()
    }
}

/// Callback for mouse_status_line.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_status_line(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        if (*ft).c.is_null() || !(*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return null_mut();
        }

        let y = if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
            (*ft).m.y
        } else if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
            (*ft).m.y - (*ft).m.statusat as u32
        } else {
            return null_mut();
        };

        let mut value = null_mut();
        xasprintf(&mut value, c"%u".as_ptr(), y);
        value.cast()
    }
}

/// Callback for mouse_status_range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_status_range(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        if (*ft).c.is_null() || !(*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return null_mut();
        }

        let mut x = 0;
        let mut y = 0;
        if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
            x = (*ft).m.x;
            y = (*ft).m.y;
        } else if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
            x = (*ft).m.x;
            y = (*ft).m.y - (*ft).m.statusat as u32;
        } else {
            return null_mut();
        }

        let sr = status_get_range((*ft).c, x, y);
        if sr.is_null() {
            return null_mut();
        }
        match (*sr).type_ {
            style_range_type::STYLE_RANGE_NONE => {
                return null_mut();
            }
            style_range_type::STYLE_RANGE_LEFT => {
                return xstrdup(c"left".as_ptr()).as_ptr().cast();
            }
            style_range_type::STYLE_RANGE_RIGHT => {
                return xstrdup(c"right".as_ptr()).as_ptr().cast();
            }
            style_range_type::STYLE_RANGE_PANE => {
                return xstrdup(c"pane".as_ptr()).as_ptr().cast();
            }
            style_range_type::STYLE_RANGE_WINDOW => {
                return xstrdup(c"window".as_ptr()).as_ptr().cast();
            }
            style_range_type::STYLE_RANGE_SESSION => {
                return xstrdup(c"session".as_ptr()).as_ptr().cast();
            }
            style_range_type::STYLE_RANGE_USER => {
                return xstrdup((*sr).string.as_ptr().cast()).as_ptr().cast();
            }
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_alternate_on(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if !(*(*ft).wp).base.saved_grid.is_null() {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_alternate_saved_x(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).base.saved_cx).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_alternate_saved_y(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).base.saved_cy).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_buffer_name(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if let Some(pb) = NonNull::new((*ft).pb) {
            return xstrdup(paste_buffer_name(pb).cast()).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_buffer_sample(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).pb.is_null() {
            return paste_make_sample((*ft).pb).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_buffer_size(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).pb.is_null() {
            let mut size = 0usize;
            paste_buffer_data((*ft).pb, &mut size);
            return format_printf(c"%zu".as_ptr(), size).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_cell_height(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format_printf(c"%u".as_ptr(), (*(*ft).c).tty.ypixel).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_cell_width(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format_printf(c"%u".as_ptr(), (*(*ft).c).tty.xpixel).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_control_mode(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).flags.intersects(client_flag::CONTROL) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_discarded(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return format_printf(c"%zu".as_ptr(), (*(*ft).c).discarded).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_flags(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return xstrdup(server_client_get_flags((*ft).c)).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_height(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            return format_printf(c"%u".as_ptr(), (*(*ft).c).tty.sy).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_key_table(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return xstrdup((*(*(*ft).c).keytable).name).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_last_session(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() && !(*(*ft).c).last_session.is_null() && session_alive((*(*ft).c).last_session).as_bool() {
            return xstrdup((*(*(*ft).c).last_session).name).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_name(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return xstrdup((*(*ft).c).name).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_pid(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return format_printf(c"%ld".as_ptr(), (*(*ft).c).pid as c_long).cast();
        }
        null_mut()
    }
}

/// Callback for client_prefix.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_prefix(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            let name = server_client_get_key_table((*ft).c);
            if strcmp((*(*(*ft).c).keytable).name, name) == 0 {
                return xstrdup(c"0".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"1".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_readonly(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).flags.intersects(client_flag::READONLY) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_session(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() && !(*(*ft).c).session.is_null() {
            return xstrdup((*(*(*ft).c).session).name).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_termfeatures(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return xstrdup(tty_get_features((*(*ft).c).term_features)).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_termname(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return xstrdup((*(*ft).c).term_name).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_termtype(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).term_type.is_null() {
                return xstrdup(c"".as_ptr()).as_ptr().cast();
            }
            return xstrdup((*(*ft).c).term_type).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_tty(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return xstrdup((*(*ft).c).ttyname).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_uid(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            let uid = proc_get_peer_uid((*(*ft).c).peer);
            if uid != -1_i32 as uid_t {
                return format_printf(c"%ld".as_ptr(), uid as c_long).cast();
            }
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_user(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            let uid = proc_get_peer_uid((*(*ft).c).peer);
            if uid != -1_i32 as uid_t {
                if let Some(pw) = NonNull::new(libc::getpwuid(uid)) {
                    return xstrdup((*pw.as_ptr()).pw_name).as_ptr().cast();
                }
            }
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_utf8(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            if (*(*ft).c).flags.intersects(client_flag::UTF8) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_width(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).c).tty.sx).cast();
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_written(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return format_printf(c"%zu".as_ptr(), (*(*ft).c).written).cast();
        }
        null_mut()
    }
}

/// Callback for config_files.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_config_files(_ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut s: *mut c_char = null_mut();
        let mut slen: usize = 0;
        let mut n: usize = 0;

        for i in 0..(cfg_nfiles as usize) {
            let n = strlen(*cfg_files.add(i)) + 1;
            s = xrealloc(s.cast(), slen + n + 1).as_ptr() as *mut c_char;
            slen += xsnprintf(s.add(slen), n + 1, c"%s,".as_ptr(), *cfg_files.add(i)) as usize;
        }
        if s.is_null() {
            return xstrdup(c"".as_ptr()).as_ptr().cast();
        }
        *s.add(slen - 1) = 0;
        s.cast()
    }
}

/// Callback for cursor_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_cursor_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_CURSOR != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for cursor_x.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_cursor_x(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).base.cx).cast();
        }
        null_mut()
    }
}

/// Callback for cursor_y.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_cursor_y(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).base.cy).cast();
        }
        null_mut()
    }
}

/// Callback for history_limit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_history_limit(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*(*ft).wp).base.grid).hlimit).cast();
        }
        null_mut()
    }
}

/// Callback for history_size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_history_size(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*(*ft).wp).base.grid).hsize).cast();
        }
        null_mut()
    }
}

/// Callback for insert_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_insert_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_INSERT != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for keypad_cursor_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_keypad_cursor_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_KCURSOR != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for keypad_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_keypad_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_KKEYPAD != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_all_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_all_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_MOUSE_ALL != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_any_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_any_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & ALL_MOUSE_MODES != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_button_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_button_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_MOUSE_BUTTON != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_pane(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid != 0 {
            if let Some(wp) = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut()) {
                return format_printf(c"%%%u".as_ptr(), (*wp.as_ptr()).id).cast();
            }
            return null_mut();
        }
        null_mut()
    }
}

/// Callback for mouse_sgr_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_sgr_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_MOUSE_SGR != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_standard_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_standard_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_MOUSE_STANDARD != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_utf8_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_utf8_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_MOUSE_UTF8 != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for mouse_x.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_x(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        let wp = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut());
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        if let Some(wp) = wp
            && cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) == 0
        {
            return format_printf(c"%u".as_ptr(), x).cast();
        }
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
                return format_printf(c"%u".as_ptr(), (*ft).m.x).cast();
            }
            if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
                return format_printf(c"%u".as_ptr(), (*ft).m.x).cast();
            }
        }
        null_mut()
    }
}

/// Callback for mouse_y.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_mouse_y(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).m.valid == 0 {
            return null_mut();
        }
        let wp = cmd_mouse_pane(&raw mut (*ft).m, null_mut(), null_mut());
        let mut x: u32 = 0;
        let mut y: u32 = 0;
        if let Some(wp) = wp
            && cmd_mouse_at(wp.as_ptr(), &raw mut (*ft).m, &mut x, &mut y, 0) == 0
        {
            return format_printf(c"%u".as_ptr(), y).cast();
        }
        if !(*ft).c.is_null() && (*(*ft).c).tty.flags.intersects(tty_flags::TTY_STARTED) {
            if (*ft).m.statusat == 0 && (*ft).m.y < (*ft).m.statuslines {
                return format_printf(c"%u".as_ptr(), (*ft).m.y).cast();
            }
            if (*ft).m.statusat > 0 && (*ft).m.y >= (*ft).m.statusat as u32 {
                return format_printf(c"%u".as_ptr(), (*ft).m.y - (*ft).m.statusat as u32).cast();
            }
        }
        null_mut()
    }
}

/// Callback for next_session_id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_next_session_id(_ft: *mut format_tree) -> *mut c_void { unsafe { format_printf(c"$%u".as_ptr(), next_session_id).cast() } }

/// Callback for origin_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_origin_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.mode & MODE_ORIGIN != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_active.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_active(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*ft).wp == (*(*(*ft).wp).window).active {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_at_left.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_at_left(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).xoff == 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_at_right.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_at_right(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).xoff + (*(*ft).wp).sx == (*(*(*ft).wp).window).sx {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_bottom.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_bottom(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).yoff + (*(*ft).wp).sy - 1).cast();
        }
        null_mut()
    }
}

/// Callback for pane_dead.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_dead(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).fd == -1 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_dead_signal.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_dead_signal(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if !wp.is_null() {
            if (*wp).flags.intersects(window_pane_flags::PANE_STATUSREADY) && WIFSIGNALED((*wp).status) {
                let name = sig2name(WTERMSIG((*wp).status));
                return format_printf(c"%s".as_ptr(), name).cast();
            }
            return null_mut();
        }
        null_mut()
    }
}

/// Callback for pane_dead_status.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_dead_status(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if !wp.is_null() {
            if (*wp).flags.intersects(window_pane_flags::PANE_STATUSREADY) && WIFEXITED((*wp).status) {
                return format_printf(c"%d".as_ptr(), WEXITSTATUS((*wp).status)).cast();
            }
            return null_mut();
        }
        null_mut()
    }
}

/// Callback for pane_dead_time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_dead_time(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let wp = (*ft).wp;
        if !wp.is_null() {
            if (*wp).flags.intersects(window_pane_flags::PANE_STATUSDRAWN) {
                return &mut (*wp).dead_time as *mut _ as *mut c_void;
            }
            return null_mut();
        }
        null_mut()
    }
}

/// Callback for pane_format.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_format(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).type_ == format_type::FORMAT_TYPE_PANE {
            return xstrdup(c"1".as_ptr()).as_ptr().cast();
        }
        xstrdup(c"0".as_ptr()).as_ptr().cast()
    }
}

/// Callback for pane_height.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_height(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).sy).cast();
        }
        null_mut()
    }
}

/// Callback for pane_id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_id(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%%%u".as_ptr(), (*(*ft).wp).id).cast();
        }
        null_mut()
    }
}

/// Callback for pane_index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_index(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut idx: u32 = 0;
        if !(*ft).wp.is_null() && window_pane_index((*ft).wp, &mut idx) == 0 {
            return format_printf(c"%u".as_ptr(), idx).cast();
        }
        null_mut()
    }
}

/// Callback for pane_input_off.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_input_off(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).flags.intersects(window_pane_flags::PANE_INPUTOFF) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_unseen_changes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_unseen_changes(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).flags.intersects(window_pane_flags::PANE_UNSEENCHANGES) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_key_mode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_key_mode(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() && !(*(*ft).wp).screen.is_null() {
            match (*(*(*ft).wp).screen).mode & EXTENDED_KEY_MODES {
                MODE_KEYS_EXTENDED => return xstrdup(c"Ext 1".as_ptr()).as_ptr().cast(),
                MODE_KEYS_EXTENDED_2 => return xstrdup(c"Ext 2".as_ptr()).as_ptr().cast(),
                _ => return xstrdup(c"VT10x".as_ptr()).as_ptr().cast(),
            }
        }
        null_mut()
    }
}

/// Callback for pane_last.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_last(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*ft).wp == tailq_first(&raw mut (*(*(*ft).wp).window).last_panes) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_left.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_left(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).xoff).cast();
        }
        null_mut()
    }
}

/// Callback for pane_marked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_marked(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if server_check_marked().as_bool() && marked_pane.wp == (*ft).wp {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_marked_set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_marked_set(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if server_check_marked().as_bool() {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_mode.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_mode(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            let wme = tailq_first(&raw mut (*(*ft).wp).modes);
            if !wme.is_null() {
                return xstrdup((*(*wme).mode).name.as_ptr()).as_ptr().cast();
            }
            return null_mut();
        }
        null_mut()
    }
}

/// Callback for pane_path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_path(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).base.path.is_null() {
                return xstrdup(c"".as_ptr()).as_ptr().cast();
            }
            return xstrdup((*(*ft).wp).base.path).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_pid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_pid(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%ld".as_ptr(), (*(*ft).wp).pid as i64).cast();
        }
        null_mut()
    }
}

/// Callback for pane_pipe.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_pipe(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).pipe_fd != -1 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_right.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_right(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).xoff + (*(*ft).wp).sx - 1).cast();
        }
        null_mut()
    }
}

/// Callback for pane_search_string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_search_string(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if (*(*ft).wp).searchstr.is_null() {
                return xstrdup(c"".as_ptr()).as_ptr().cast();
            }
            return xstrdup((*(*ft).wp).searchstr).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_synchronized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_synchronized(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if options_get_number((*(*ft).wp).options, c"synchronize-panes".as_ptr()) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_title.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_title(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return xstrdup((*(*ft).wp).base.title).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_top.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_top(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).yoff).cast();
        }
        null_mut()
    }
}

/// Callback for pane_tty.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_tty(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return xstrdup((*(*ft).wp).tty.as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for pane_width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_pane_width(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).sx).cast();
        }
        null_mut()
    }
}

/// Callback for scroll_region_lower.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_scroll_region_lower(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).base.rlower).cast();
        }
        null_mut()
    }
}

/// Callback for scroll_region_upper.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_scroll_region_upper(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).wp).base.rupper).cast();
        }
        null_mut()
    }
}

/// Callback for server_sessions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_server_sessions(_ft: *mut format_tree) -> *mut c_void {
    unsafe {
        let mut n: u32 = rb_foreach(&raw mut sessions).count() as u32;
        format_printf(c"%u".as_ptr(), n).cast()
    }
}

/// Callback for session_attached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_attached(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).s).attached).cast();
        }
        null_mut()
    }
}

/// Callback for session_format.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_format(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).type_ == format_type::FORMAT_TYPE_SESSION {
            return xstrdup(c"1".as_ptr()).as_ptr().cast();
        }
        xstrdup(c"0".as_ptr()).as_ptr().cast()
    }
}

/// Callback for session_group.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_group(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                return xstrdup((*sg).name).as_ptr().cast();
            }
        }
        null_mut()
    }
}

/// Callback for session_group_attached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_group_attached(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                return format_printf(c"%u".as_ptr(), session_group_attached_count(sg)).cast();
            }
        }
        null_mut()
    }
}

/// Callback for session_group_many_attached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_group_many_attached(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                if session_group_attached_count(sg) > 1 {
                    return xstrdup(c"1".as_ptr()).as_ptr().cast();
                }
                return xstrdup(c"0".as_ptr()).as_ptr().cast();
            }
        }
        null_mut()
    }
}

/// Callback for session_group_size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_group_size(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            let sg = session_group_contains((*ft).s);
            if !sg.is_null() {
                return format_printf(c"%u".as_ptr(), session_group_count(sg)).cast();
            }
        }
        null_mut()
    }
}

/// Callback for session_grouped.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_grouped(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            if !session_group_contains((*ft).s).is_null() {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for session_id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_id(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return format_printf(c"$%u".as_ptr(), (*(*ft).s).id).cast();
        }
        null_mut()
    }
}

/// Callback for session_many_attached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_many_attached(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            if (*(*ft).s).attached > 1 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for session_marked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_marked(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            if server_check_marked().as_bool() && marked_pane.s == (*ft).s {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for session_name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_name(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return xstrdup((*(*ft).s).name).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for session_path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_path(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return xstrdup((*(*ft).s).cwd).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for session_windows.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_windows(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return format_printf(c"%u".as_ptr(), winlink_count(&raw mut (*(*ft).s).windows)).cast();
        }
        null_mut()
    }
}

/// Callback for socket_path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_socket_path(_ft: *mut format_tree) -> *mut c_void { unsafe { xstrdup(socket_path).as_ptr().cast() } }

/// Callback for version.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_version(_ft: *mut format_tree) -> *mut c_void { unsafe { xstrdup(getversion()).as_ptr().cast() } }

/// Callback for active_window_index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_active_window_index(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*(*ft).s).curw).idx).cast();
        }
        null_mut()
    }
}

/// Callback for last_window_index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_last_window_index(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            let wl = rb_max(&raw mut (*(*ft).s).windows);
            return format_printf(c"%u".as_ptr(), (*wl).idx).cast();
        }
        null_mut()
    }
}

/// Callback for window_active.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_active(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == (*(*(*ft).wl).session).curw {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_activity_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_activity_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if ((*(*ft).wl).flags & WINLINK_ACTIVITY) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_bell_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_bell_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if ((*(*ft).wl).flags & WINLINK_BELL) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_bigger.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_bigger(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            let mut ox: u32 = 0;
            let mut oy: u32 = 0;
            let mut sx: u32 = 0;
            let mut sy: u32 = 0;
            if tty_window_offset(&raw mut (*(*ft).c).tty, &mut ox, &mut oy, &mut sx, &mut sy) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_cell_height.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_cell_height(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).w).ypixel).cast();
        }
        null_mut()
    }
}

/// Callback for window_cell_width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_cell_width(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).w).xpixel).cast();
        }
        null_mut()
    }
}

/// Callback for window_end_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_end_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == rb_max(&raw mut (*(*(*ft).wl).session).windows) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_flags.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_flags(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            return xstrdup(window_printable_flags((*ft).wl, 1)).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_format.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_format(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if (*ft).type_ == format_type::FORMAT_TYPE_WINDOW {
            return xstrdup(c"1".as_ptr()).as_ptr().cast();
        }
        xstrdup(c"0".as_ptr()).as_ptr().cast()
    }
}

/// Callback for window_height.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_height(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).w).sy).cast();
        }
        null_mut()
    }
}

/// Callback for window_id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_id(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"@%u".as_ptr(), (*(*ft).w).id).cast();
        }
        null_mut()
    }
}

/// Callback for window_index.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_index(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            return format_printf(c"%d".as_ptr(), (*(*ft).wl).idx).cast();
        }
        null_mut()
    }
}

/// Callback for window_last_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_last_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == tailq_first(&raw mut (*(*(*ft).wl).session).lastw) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_linked.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_linked(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if session_is_linked((*(*ft).wl).session, (*(*ft).wl).window) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_linked_sessions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_linked_sessions(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*(*ft).wl).window).references).cast();
        }
        null_mut()
    }
}

/// Callback for window_marked_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_marked_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if server_check_marked().as_bool() && marked_pane.wl == (*ft).wl {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_name.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_name(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"%s".as_ptr(), (*(*ft).w).name).cast();
        }
        null_mut()
    }
}

/// Callback for window_offset_x.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_offset_x(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            let mut ox: u32 = 0;
            let mut oy: u32 = 0;
            let mut sx: u32 = 0;
            let mut sy: u32 = 0;
            if tty_window_offset(&raw mut (*(*ft).c).tty, &mut ox, &mut oy, &mut sx, &mut sy) != 0 {
                return format_printf(c"%u".as_ptr(), ox).cast();
            }
        }
        null_mut()
    }
}

/// Callback for window_offset_y.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_offset_y(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            let mut ox: u32 = 0;
            let mut oy: u32 = 0;
            let mut sx: u32 = 0;
            let mut sy: u32 = 0;
            if tty_window_offset(&raw mut (*(*ft).c).tty, &mut ox, &mut oy, &mut sx, &mut sy) != 0 {
                return format_printf(c"%u".as_ptr(), oy).cast();
            }
        }
        null_mut()
    }
}

/// Callback for window_panes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_panes(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"%u".as_ptr(), window_count_panes((*ft).w)).cast();
        }
        null_mut()
    }
}

/// Callback for window_raw_flags.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_raw_flags(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            return xstrdup(window_printable_flags((*ft).wl, 0)).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_silence_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_silence_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if ((*(*ft).wl).flags & WINLINK_SILENCE) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_start_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_start_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wl.is_null() {
            if (*ft).wl == rb_min(&raw mut (*(*(*ft).wl).session).windows) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for window_width.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_width(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return format_printf(c"%u".as_ptr(), (*(*ft).w).sx).cast();
        }
        null_mut()
    }
}

/// Callback for window_zoomed_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_zoomed_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            if (*(*ft).w).flags.intersects(window_flag::ZOOMED) {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for wrap_flag.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_wrap_flag(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).wp.is_null() {
            if ((*(*ft).wp).base.mode & MODE_WRAP) != 0 {
                return xstrdup(c"1".as_ptr()).as_ptr().cast();
            }
            return xstrdup(c"0".as_ptr()).as_ptr().cast();
        }
        null_mut()
    }
}

/// Callback for buffer_created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_buffer_created(ft: *mut format_tree) -> *mut c_void {
    static mut tv: timeval = timeval { tv_sec: 0, tv_usec: 0 };
    unsafe {
        if let Some(pb) = NonNull::new((*ft).pb) {
            timerclear(&raw mut tv);
            tv.tv_sec = paste_buffer_created(pb);
            return &raw mut tv as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for client_activity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_activity(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return &mut (*(*ft).c).activity_time as *mut _ as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for client_created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_created(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).c.is_null() {
            return &mut (*(*ft).c).creation_time as *mut _ as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for session_activity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_activity(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return &mut (*(*ft).s).activity_time as *mut _ as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for session_created.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_created(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return &mut (*(*ft).s).creation_time as *mut _ as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for session_last_attached.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_session_last_attached(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).s.is_null() {
            return &mut (*(*ft).s).last_attached_time as *mut _ as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for start_time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_start_time(_ft: *mut format_tree) -> *mut c_void { unsafe { &raw mut start_time as *mut c_void } }

/// Callback for window_activity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_window_activity(ft: *mut format_tree) -> *mut c_void {
    unsafe {
        if !(*ft).w.is_null() {
            return &mut (*(*ft).w).activity_time as *mut _ as *mut c_void;
        }
        null_mut()
    }
}

/// Callback for buffer_mode_format.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_buffer_mode_format(_ft: *mut format_tree) -> *mut c_void { unsafe { xstrdup(window_buffer_mode.default_format.0).as_ptr().cast() } }

/// Callback for client_mode_format.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_client_mode_format(_ft: *mut format_tree) -> *mut c_void { unsafe { xstrdup(window_client_mode.default_format.0).as_ptr().cast() } }

/// Callback for tree_mode_format.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_tree_mode_format(_ft: *mut format_tree) -> *mut c_void { unsafe { xstrdup(window_tree_mode.default_format.0).as_ptr().cast() } }

/// Callback for uid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_uid(_ft: *mut format_tree) -> *mut c_void { unsafe { format_printf(c"%ld".as_ptr(), getuid() as i64).cast() } }

/// Callback for user.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_cb_user(_ft: *mut format_tree) -> *mut c_void { unsafe { if let Some(pw) = NonNull::new(getpwuid(getuid())) { xstrdup((*pw.as_ptr()).pw_name).as_ptr().cast() } else { null_mut() } } }

/// Format table type.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum format_table_type {
    FORMAT_TABLE_STRING,
    FORMAT_TABLE_TIME,
}

/// Format table entry.
#[repr(C)]
pub struct format_table_entry {
    key: SyncCharPtr,
    type_: format_table_type,
    cb: format_cb,
}

impl format_table_entry {
    pub const fn new(key: &'static CStr, type_: format_table_type, cb: unsafe extern "C" fn(_: *mut format_tree) -> *mut c_void) -> Self { Self { key: SyncCharPtr::new(key), type_, cb: Some(cb) } }
}

/*
 * Format table. Default format variables (that are almost always in the tree
 * and where the value is expanded by a callback in this file) are listed
 * here. Only variables which are added by the caller go into the tree.
 */
#[rustfmt::skip]
static format_table: [format_table_entry ; 171] = [
    format_table_entry::new(c"active_window_index", format_table_type::FORMAT_TABLE_STRING, format_cb_active_window_index),
     format_table_entry::new(c"alternate_on", format_table_type::FORMAT_TABLE_STRING, format_cb_alternate_on),
     format_table_entry::new(c"alternate_saved_x", format_table_type::FORMAT_TABLE_STRING, format_cb_alternate_saved_x),
     format_table_entry::new(c"alternate_saved_y", format_table_type::FORMAT_TABLE_STRING, format_cb_alternate_saved_y),
     format_table_entry::new(c"buffer_created", format_table_type::FORMAT_TABLE_TIME, format_cb_buffer_created),
     format_table_entry::new(c"buffer_mode_format", format_table_type::FORMAT_TABLE_STRING, format_cb_buffer_mode_format),
     format_table_entry::new(c"buffer_name", format_table_type::FORMAT_TABLE_STRING, format_cb_buffer_name),
     format_table_entry::new(c"buffer_sample", format_table_type::FORMAT_TABLE_STRING, format_cb_buffer_sample),
     format_table_entry::new(c"buffer_size", format_table_type::FORMAT_TABLE_STRING, format_cb_buffer_size),
     format_table_entry::new(c"client_activity", format_table_type::FORMAT_TABLE_TIME, format_cb_client_activity),
     format_table_entry::new(c"client_cell_height", format_table_type::FORMAT_TABLE_STRING, format_cb_client_cell_height),
     format_table_entry::new(c"client_cell_width", format_table_type::FORMAT_TABLE_STRING, format_cb_client_cell_width),
     format_table_entry::new(c"client_control_mode", format_table_type::FORMAT_TABLE_STRING, format_cb_client_control_mode),
     format_table_entry::new(c"client_created", format_table_type::FORMAT_TABLE_TIME, format_cb_client_created),
     format_table_entry::new(c"client_discarded", format_table_type::FORMAT_TABLE_STRING, format_cb_client_discarded),
     format_table_entry::new(c"client_flags", format_table_type::FORMAT_TABLE_STRING, format_cb_client_flags),
     format_table_entry::new(c"client_height", format_table_type::FORMAT_TABLE_STRING, format_cb_client_height),
     format_table_entry::new(c"client_key_table", format_table_type::FORMAT_TABLE_STRING, format_cb_client_key_table),
     format_table_entry::new(c"client_last_session", format_table_type::FORMAT_TABLE_STRING, format_cb_client_last_session),
     format_table_entry::new(c"client_mode_format", format_table_type::FORMAT_TABLE_STRING, format_cb_client_mode_format),
     format_table_entry::new(c"client_name", format_table_type::FORMAT_TABLE_STRING, format_cb_client_name),
     format_table_entry::new(c"client_pid", format_table_type::FORMAT_TABLE_STRING, format_cb_client_pid),
     format_table_entry::new(c"client_prefix", format_table_type::FORMAT_TABLE_STRING, format_cb_client_prefix),
     format_table_entry::new(c"client_readonly", format_table_type::FORMAT_TABLE_STRING, format_cb_client_readonly),
     format_table_entry::new(c"client_session", format_table_type::FORMAT_TABLE_STRING, format_cb_client_session),
     format_table_entry::new(c"client_termfeatures", format_table_type::FORMAT_TABLE_STRING, format_cb_client_termfeatures),
     format_table_entry::new(c"client_termname", format_table_type::FORMAT_TABLE_STRING, format_cb_client_termname),
     format_table_entry::new(c"client_termtype", format_table_type::FORMAT_TABLE_STRING, format_cb_client_termtype),
     format_table_entry::new(c"client_tty", format_table_type::FORMAT_TABLE_STRING, format_cb_client_tty),
     format_table_entry::new(c"client_uid", format_table_type::FORMAT_TABLE_STRING, format_cb_client_uid),
     format_table_entry::new(c"client_user", format_table_type::FORMAT_TABLE_STRING, format_cb_client_user),
     format_table_entry::new(c"client_utf8", format_table_type::FORMAT_TABLE_STRING, format_cb_client_utf8),
     format_table_entry::new(c"client_width", format_table_type::FORMAT_TABLE_STRING, format_cb_client_width),
     format_table_entry::new(c"client_written", format_table_type::FORMAT_TABLE_STRING, format_cb_client_written),
     format_table_entry::new(c"config_files", format_table_type::FORMAT_TABLE_STRING, format_cb_config_files),
     format_table_entry::new(c"cursor_character", format_table_type::FORMAT_TABLE_STRING, format_cb_cursor_character),
     format_table_entry::new(c"cursor_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_cursor_flag),
     format_table_entry::new(c"cursor_x", format_table_type::FORMAT_TABLE_STRING, format_cb_cursor_x),
     format_table_entry::new(c"cursor_y", format_table_type::FORMAT_TABLE_STRING, format_cb_cursor_y),
     format_table_entry::new(c"history_all_bytes", format_table_type::FORMAT_TABLE_STRING, format_cb_history_all_bytes),
     format_table_entry::new(c"history_bytes", format_table_type::FORMAT_TABLE_STRING, format_cb_history_bytes),
     format_table_entry::new(c"history_limit", format_table_type::FORMAT_TABLE_STRING, format_cb_history_limit),
     format_table_entry::new(c"history_size", format_table_type::FORMAT_TABLE_STRING, format_cb_history_size),
     format_table_entry::new(c"host", format_table_type::FORMAT_TABLE_STRING, format_cb_host),
     format_table_entry::new(c"host_short", format_table_type::FORMAT_TABLE_STRING, format_cb_host_short),
     format_table_entry::new(c"insert_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_insert_flag),
     format_table_entry::new(c"keypad_cursor_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_keypad_cursor_flag),
     format_table_entry::new(c"keypad_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_keypad_flag),
     format_table_entry::new(c"last_window_index", format_table_type::FORMAT_TABLE_STRING, format_cb_last_window_index),
     format_table_entry::new(c"mouse_all_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_all_flag),
     format_table_entry::new(c"mouse_any_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_any_flag),
     format_table_entry::new(c"mouse_button_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_button_flag),
     format_table_entry::new(c"mouse_hyperlink", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_hyperlink),
     format_table_entry::new(c"mouse_line", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_line),
     format_table_entry::new(c"mouse_pane", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_pane),
     format_table_entry::new(c"mouse_sgr_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_sgr_flag),
     format_table_entry::new(c"mouse_standard_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_standard_flag),
     format_table_entry::new(c"mouse_status_line", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_status_line),
     format_table_entry::new(c"mouse_status_range", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_status_range),
     format_table_entry::new(c"mouse_utf8_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_utf8_flag),
     format_table_entry::new(c"mouse_word", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_word),
     format_table_entry::new(c"mouse_x", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_x),
     format_table_entry::new(c"mouse_y", format_table_type::FORMAT_TABLE_STRING, format_cb_mouse_y),
     format_table_entry::new(c"next_session_id", format_table_type::FORMAT_TABLE_STRING, format_cb_next_session_id),
     format_table_entry::new(c"origin_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_origin_flag),
     format_table_entry::new(c"pane_active", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_active),
     format_table_entry::new(c"pane_at_bottom", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_at_bottom),
     format_table_entry::new(c"pane_at_left", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_at_left),
     format_table_entry::new(c"pane_at_right", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_at_right),
     format_table_entry::new(c"pane_at_top", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_at_top),
     format_table_entry::new(c"pane_bg", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_bg),
     format_table_entry::new(c"pane_bottom", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_bottom),
     format_table_entry::new(c"pane_current_command", format_table_type::FORMAT_TABLE_STRING, format_cb_current_command),
     format_table_entry::new(c"pane_current_path", format_table_type::FORMAT_TABLE_STRING, format_cb_current_path),
     format_table_entry::new(c"pane_dead", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_dead),
     format_table_entry::new(c"pane_dead_signal", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_dead_signal),
     format_table_entry::new(c"pane_dead_status", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_dead_status),
     format_table_entry::new(c"pane_dead_time", format_table_type::FORMAT_TABLE_TIME, format_cb_pane_dead_time),
     format_table_entry::new(c"pane_fg", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_fg),
     format_table_entry::new(c"pane_format", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_format),
     format_table_entry::new(c"pane_height", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_height),
     format_table_entry::new(c"pane_id", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_id),
     format_table_entry::new(c"pane_in_mode", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_in_mode),
     format_table_entry::new(c"pane_index", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_index),
     format_table_entry::new(c"pane_input_off", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_input_off),
     format_table_entry::new(c"pane_key_mode", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_key_mode),
     format_table_entry::new(c"pane_last", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_last),
     format_table_entry::new(c"pane_left", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_left),
     format_table_entry::new(c"pane_marked", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_marked),
     format_table_entry::new(c"pane_marked_set", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_marked_set),
     format_table_entry::new(c"pane_mode", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_mode),
     format_table_entry::new(c"pane_path", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_path),
     format_table_entry::new(c"pane_pid", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_pid),
     format_table_entry::new(c"pane_pipe", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_pipe),
     format_table_entry::new(c"pane_right", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_right),
     format_table_entry::new(c"pane_search_string", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_search_string),
     format_table_entry::new(c"pane_start_command", format_table_type::FORMAT_TABLE_STRING, format_cb_start_command),
     format_table_entry::new(c"pane_start_path", format_table_type::FORMAT_TABLE_STRING, format_cb_start_path),
     format_table_entry::new(c"pane_synchronized", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_synchronized),
     format_table_entry::new(c"pane_tabs", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_tabs),
     format_table_entry::new(c"pane_title", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_title),
     format_table_entry::new(c"pane_top", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_top),
     format_table_entry::new(c"pane_tty", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_tty),
     format_table_entry::new(c"pane_unseen_changes", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_unseen_changes),
     format_table_entry::new(c"pane_width", format_table_type::FORMAT_TABLE_STRING, format_cb_pane_width),
     format_table_entry::new(c"pid", format_table_type::FORMAT_TABLE_STRING, format_cb_pid),
     format_table_entry::new(c"scroll_region_lower", format_table_type::FORMAT_TABLE_STRING, format_cb_scroll_region_lower),
     format_table_entry::new(c"scroll_region_upper", format_table_type::FORMAT_TABLE_STRING, format_cb_scroll_region_upper),
     format_table_entry::new(c"server_sessions", format_table_type::FORMAT_TABLE_STRING, format_cb_server_sessions),
     format_table_entry::new(c"session_activity", format_table_type::FORMAT_TABLE_TIME, format_cb_session_activity),
     format_table_entry::new(c"session_alerts", format_table_type::FORMAT_TABLE_STRING, format_cb_session_alerts),
     format_table_entry::new(c"session_attached", format_table_type::FORMAT_TABLE_STRING, format_cb_session_attached),
     format_table_entry::new(c"session_attached_list", format_table_type::FORMAT_TABLE_STRING, format_cb_session_attached_list),
     format_table_entry::new(c"session_created", format_table_type::FORMAT_TABLE_TIME, format_cb_session_created),
     format_table_entry::new(c"session_format", format_table_type::FORMAT_TABLE_STRING, format_cb_session_format),
     format_table_entry::new(c"session_group", format_table_type::FORMAT_TABLE_STRING, format_cb_session_group),
     format_table_entry::new(c"session_group_attached", format_table_type::FORMAT_TABLE_STRING, format_cb_session_group_attached),
     format_table_entry::new(c"session_group_attached_list", format_table_type::FORMAT_TABLE_STRING, format_cb_session_group_attached_list),
     format_table_entry::new(c"session_group_list", format_table_type::FORMAT_TABLE_STRING, format_cb_session_group_list),
     format_table_entry::new(c"session_group_many_attached", format_table_type::FORMAT_TABLE_STRING, format_cb_session_group_many_attached),
     format_table_entry::new(c"session_group_size", format_table_type::FORMAT_TABLE_STRING, format_cb_session_group_size),
     format_table_entry::new(c"session_grouped", format_table_type::FORMAT_TABLE_STRING, format_cb_session_grouped),
     format_table_entry::new(c"session_id", format_table_type::FORMAT_TABLE_STRING, format_cb_session_id),
     format_table_entry::new(c"session_last_attached", format_table_type::FORMAT_TABLE_TIME, format_cb_session_last_attached),
     format_table_entry::new(c"session_many_attached", format_table_type::FORMAT_TABLE_STRING, format_cb_session_many_attached),
     format_table_entry::new(c"session_marked", format_table_type::FORMAT_TABLE_STRING, format_cb_session_marked),
     format_table_entry::new(c"session_name", format_table_type::FORMAT_TABLE_STRING, format_cb_session_name),
     format_table_entry::new(c"session_path", format_table_type::FORMAT_TABLE_STRING, format_cb_session_path),
     format_table_entry::new(c"session_stack", format_table_type::FORMAT_TABLE_STRING, format_cb_session_stack),
     format_table_entry::new(c"session_windows", format_table_type::FORMAT_TABLE_STRING, format_cb_session_windows),
     format_table_entry::new(c"socket_path", format_table_type::FORMAT_TABLE_STRING, format_cb_socket_path),
     format_table_entry::new(c"start_time", format_table_type::FORMAT_TABLE_TIME, format_cb_start_time),
     format_table_entry::new(c"tree_mode_format", format_table_type::FORMAT_TABLE_STRING, format_cb_tree_mode_format),
     format_table_entry::new(c"uid", format_table_type::FORMAT_TABLE_STRING, format_cb_uid),
     format_table_entry::new(c"user", format_table_type::FORMAT_TABLE_STRING, format_cb_user),
     format_table_entry::new(c"version", format_table_type::FORMAT_TABLE_STRING, format_cb_version),
     format_table_entry::new(c"window_active", format_table_type::FORMAT_TABLE_STRING, format_cb_window_active),
     format_table_entry::new(c"window_active_clients", format_table_type::FORMAT_TABLE_STRING, format_cb_window_active_clients),
     format_table_entry::new(c"window_active_clients_list", format_table_type::FORMAT_TABLE_STRING, format_cb_window_active_clients_list),
     format_table_entry::new(c"window_active_sessions", format_table_type::FORMAT_TABLE_STRING, format_cb_window_active_sessions),
     format_table_entry::new(c"window_active_sessions_list", format_table_type::FORMAT_TABLE_STRING, format_cb_window_active_sessions_list),
     format_table_entry::new(c"window_activity", format_table_type::FORMAT_TABLE_TIME, format_cb_window_activity),
     format_table_entry::new(c"window_activity_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_activity_flag),
     format_table_entry::new(c"window_bell_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_bell_flag),
     format_table_entry::new(c"window_bigger", format_table_type::FORMAT_TABLE_STRING, format_cb_window_bigger),
     format_table_entry::new(c"window_cell_height", format_table_type::FORMAT_TABLE_STRING, format_cb_window_cell_height),
     format_table_entry::new(c"window_cell_width", format_table_type::FORMAT_TABLE_STRING, format_cb_window_cell_width),
     format_table_entry::new(c"window_end_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_end_flag),
     format_table_entry::new(c"window_flags", format_table_type::FORMAT_TABLE_STRING, format_cb_window_flags),
     format_table_entry::new(c"window_format", format_table_type::FORMAT_TABLE_STRING, format_cb_window_format),
     format_table_entry::new(c"window_height", format_table_type::FORMAT_TABLE_STRING, format_cb_window_height),
     format_table_entry::new(c"window_id", format_table_type::FORMAT_TABLE_STRING, format_cb_window_id),
     format_table_entry::new(c"window_index", format_table_type::FORMAT_TABLE_STRING, format_cb_window_index),
     format_table_entry::new(c"window_last_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_last_flag),
     format_table_entry::new(c"window_layout", format_table_type::FORMAT_TABLE_STRING, format_cb_window_layout),
     format_table_entry::new(c"window_linked", format_table_type::FORMAT_TABLE_STRING, format_cb_window_linked),
     format_table_entry::new(c"window_linked_sessions", format_table_type::FORMAT_TABLE_STRING, format_cb_window_linked_sessions),
     format_table_entry::new(c"window_linked_sessions_list", format_table_type::FORMAT_TABLE_STRING, format_cb_window_linked_sessions_list),
     format_table_entry::new(c"window_marked_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_marked_flag),
     format_table_entry::new(c"window_name", format_table_type::FORMAT_TABLE_STRING, format_cb_window_name),
     format_table_entry::new(c"window_offset_x", format_table_type::FORMAT_TABLE_STRING, format_cb_window_offset_x),
     format_table_entry::new(c"window_offset_y", format_table_type::FORMAT_TABLE_STRING, format_cb_window_offset_y),
     format_table_entry::new(c"window_panes", format_table_type::FORMAT_TABLE_STRING, format_cb_window_panes),
     format_table_entry::new(c"window_raw_flags", format_table_type::FORMAT_TABLE_STRING, format_cb_window_raw_flags),
     format_table_entry::new(c"window_silence_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_silence_flag),
     format_table_entry::new(c"window_stack_index", format_table_type::FORMAT_TABLE_STRING, format_cb_window_stack_index),
     format_table_entry::new(c"window_start_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_start_flag),
     format_table_entry::new(c"window_visible_layout", format_table_type::FORMAT_TABLE_STRING, format_cb_window_visible_layout),
     format_table_entry::new(c"window_width", format_table_type::FORMAT_TABLE_STRING, format_cb_window_width),
     format_table_entry::new(c"window_zoomed_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_window_zoomed_flag),
     format_table_entry::new(c"wrap_flag", format_table_type::FORMAT_TABLE_STRING, format_cb_wrap_flag)
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_table_compare(key0: *const c_void, entry0: *const c_void) -> i32 {
    unsafe {
        let key = key0 as *const c_char;
        let entry = entry0 as *const format_table_entry;
        strcmp(key, (*entry).key.as_ptr())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_table_get(key: *const c_char) -> *mut format_table_entry { unsafe { libc::bsearch(key as *const c_void, format_table.as_ptr().cast(), format_table.len(), std::mem::size_of::<format_table_entry>(), Some(format_table_compare)) as *mut format_table_entry } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_merge(ft: *mut format_tree, from: *mut format_tree) {
    unsafe {
        for fe in rb_foreach(&raw mut (*from).tree).map(NonNull::as_ptr) {
            if !(*fe).value.is_null() {
                format_add(ft, (*fe).key, c"%s".as_ptr(), (*fe).value);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_get_pane(ft: *mut format_tree) -> *mut window_pane { unsafe { (*ft).wp } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_create_add_item(ft: *mut format_tree, item: *mut cmdq_item) {
    unsafe {
        let event = cmdq_get_event(item);
        let m = &(*event).m;

        cmdq_merge_formats(item, ft);
        memcpy__(&raw mut (*ft).m, m);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_create(c: *mut client, item: *mut cmdq_item, tag: i32, flags: format_flags) -> *mut format_tree {
    unsafe {
        let ft = xcalloc1::<format_tree>() as *mut format_tree;
        rb_init(&raw mut (*ft).tree);

        if !c.is_null() {
            (*ft).client = c;
            (*c).references += 1;
        }
        (*ft).item = item;
        (*ft).tag = tag as u32;
        (*ft).flags = flags;

        if !item.is_null() {
            format_create_add_item(ft, item);
        }

        ft
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_free(ft: *mut format_tree) {
    unsafe {
        for fe in rb_foreach(&raw mut (*ft).tree).map(NonNull::as_ptr) {
            rb_remove(&raw mut (*ft).tree, fe);
            free_((*fe).value);
            free_((*fe).key);
            free_(fe);
        }

        if !(*ft).client.is_null() {
            server_client_unref((*ft).client);
        }
        free(ft as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_log_debug_cb(key: *const c_char, value: *const c_char, arg: *mut c_void) {
    unsafe {
        let prefix = arg as *const c_char;
        log_debug!("{}: {}={}", _s(prefix), _s(key), _s(value));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_log_debug(ft: *mut format_tree, prefix: *const c_char) {
    unsafe {
        format_each(ft, Some(format_log_debug_cb), prefix as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_each(ft: *mut format_tree, cb: Option<unsafe extern "C" fn(*const c_char, *const c_char, *mut c_void)>, arg: *mut c_void) {
    unsafe {
        let mut s = [0i8; 64];

        for fte in &format_table {
            let value = fte.cb.unwrap()(ft);

            if value.is_null() {
                continue;
            }

            if fte.type_ == format_table_type::FORMAT_TABLE_TIME {
                let tv = value as *const timeval;
                xsnprintf(s.as_mut_ptr(), s.len(), c"%lld".as_ptr(), (*tv).tv_sec);
                cb.unwrap()(fte.key.as_ptr(), s.as_ptr(), arg);
            } else {
                cb.unwrap()(fte.key.as_ptr(), value as *const c_char, arg);
                free(value);
            }
        }

        for fe in rb_foreach(&raw mut (*ft).tree).map(NonNull::as_ptr) {
            if (*fe).time != 0 {
                xsnprintf(s.as_mut_ptr(), s.len(), c"%lld".as_ptr(), (*fe).time);
                cb.unwrap()((*fe).key, s.as_ptr(), arg);
            } else {
                if (*fe).value.is_null() && (*fe).cb.is_some() {
                    (*fe).value = (*fe).cb.unwrap()(ft).cast();
                    if (*fe).value.is_null() {
                        (*fe).value = xstrdup(c"".as_ptr()).as_ptr().cast();
                    }
                }
                cb.unwrap()((*fe).key, (*fe).value, arg);
            }
        }
    }
}

/// Add a key-value pair.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_add(ft: *mut format_tree, key: *const c_char, fmt: *const c_char, mut ap: ...) {
    unsafe {
        let mut fe = xmalloc_::<format_entry>().as_ptr();

        (*fe).key = xstrdup(key).as_ptr();

        let fe_now = rb_insert(&raw mut (*ft).tree, fe);
        if !fe_now.is_null() {
            free_((*fe).key);
            free_(fe);
            free_((*fe_now).value);
            fe = fe_now;
        }

        (*fe).cb = None;
        (*fe).time = 0;

        xvasprintf(&raw mut (*fe).value, fmt, ap.as_va_list());
    }
}

/// Add a key and time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_add_tv(ft: *mut format_tree, key: *const c_char, tv: *const timeval) {
    unsafe {
        let mut fe = xmalloc_::<format_entry>().as_ptr();

        (*fe).key = xstrdup(key).as_ptr();

        let fe_now = rb_insert(&raw mut (*ft).tree, fe);
        if !fe_now.is_null() {
            free_((*fe).key);
            free_(fe);
            free_((*fe_now).value);
            fe = fe_now;
        }

        (*fe).cb = None;
        (*fe).time = (*tv).tv_sec;

        (*fe).value = null_mut();
    }
}

/// Add a key and function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_add_cb(ft: *mut format_tree, key: *const c_char, cb: format_cb) {
    unsafe {
        let mut fe = xmalloc_::<format_entry>().as_ptr();
        let mut fe_now;

        (*fe).key = xstrdup(key).as_ptr();

        fe_now = rb_insert(&raw mut (*ft).tree, fe);
        if !fe_now.is_null() {
            free_((*fe).key);
            free_(fe);
            free_((*fe_now).value);
            fe = fe_now;
        }

        (*fe).cb = cb;
        (*fe).time = 0;

        (*fe).value = null_mut();
    }
}

/// Quote shell special characters in string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_quote_shell(s: *const c_char) -> *mut c_char {
    unsafe {
        let mut out: *mut c_char = xmalloc(strlen(s) * 2 + 1).as_ptr().cast();
        let mut at = out;
        let mut cp = s;
        while *cp != b'\0' as c_char {
            if !strchr(c"|&;<>()$`\\\"'*?[# =%".as_ptr(), *cp as i32).is_null() {
                *at = b'\\' as c_char;
                at = at.add(1);
            }
            *at = *cp;
            at = at.add(1);
            cp = cp.add(1);
        }
        *at = b'\0' as c_char;
        out
    }
}

/// Quote #s in string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_quote_style(s: *const c_char) -> *mut c_char {
    unsafe {
        let mut out: *mut c_char = xmalloc(strlen(s) * 2 + 1).as_ptr().cast();
        let mut at = out;

        let mut cp = s;
        while *cp != b'\0' as c_char {
            if *cp == b'#' as c_char {
                *at = b'#' as c_char;
                at = at.add(1);
            }
            *at = *cp;
            at = at.add(1);
            cp = cp.add(1);
        }
        *at = b'\0' as c_char;
        out
    }
}

/// Make a prettier time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_pretty_time(t: time_t, seconds: i32) -> *mut c_char {
    unsafe {
        // struct tm now_tm, tm;
        // time_t now, age;
        // char s[9];

        let mut now: time_t = 0;
        libc::time(&raw mut now);
        if (now < t) {
            now = t;
        }
        let mut age = now - t;

        let mut now_tm = MaybeUninit::<tm>::uninit();
        let mut now_tm = now_tm.as_mut_ptr();
        let mut tm = MaybeUninit::<tm>::uninit();
        let mut tm = tm.as_mut_ptr();

        localtime_r(&raw const now, now_tm);
        localtime_r(&raw const t, tm);

        // Last 24 hours.
        const sizeof_s: usize = 9;
        let mut s = [0i8; 9];
        if (age < 24 * 3600) {
            if (seconds != 0) {
                strftime(s.as_mut_ptr(), sizeof_s, c"%H:%M:%S".as_ptr(), tm);
            } else {
                strftime(s.as_mut_ptr(), sizeof_s, c"%H:%M".as_ptr(), tm);
            }
            return xstrdup(s.as_ptr()).as_ptr();
        }

        // This month or last 28 days.
        if (((*tm).tm_year == (*now_tm).tm_year && (*tm).tm_mon == (*now_tm).tm_mon) || age < 28 * 24 * 3600) {
            strftime(s.as_mut_ptr(), sizeof_s, c"%a%d".as_ptr(), tm);
            return xstrdup(s.as_ptr()).as_ptr();
        }

        // Last 12 months.
        if (((*tm).tm_year == (*now_tm).tm_year && (*tm).tm_mon < (*now_tm).tm_mon) || ((*tm).tm_year == (*now_tm).tm_year - 1 && (*tm).tm_mon > (*now_tm).tm_mon)) {
            strftime(s.as_mut_ptr(), sizeof_s, c"%d%b".as_ptr(), tm);
            return xstrdup(s.as_ptr()).as_ptr();
        }

        // Older than that.
        strftime(s.as_mut_ptr(), sizeof_s, c"%h%y".as_ptr(), tm);
        xstrdup(s.as_ptr()).as_ptr()
    }
}

/* Find a format entry. */
#[unsafe(no_mangle)]
fn format_find(ft: *mut format_tree, key: *const c_char, modifiers: format_modifiers, time_format: *const c_char) -> *mut c_char {
    unsafe {
        // struct format_table_entry *fte;
        // void *value;
        // struct format_entry *fe, fe_find;
        // struct environ_entry *envent;
        // struct options_entry *o;
        // int idx;
        // char *found = NULL, *saved, s[512];
        // const char *errstr;
        // time_t t = 0;
        // struct tm tm;
        let mut s = MaybeUninit::<[i8; 512]>::uninit();
        let mut s = s.as_mut_ptr() as *mut i8;
        let mut errstr: *const c_char = null();
        let mut fe_find = MaybeUninit::<format_entry>::uninit();

        const sizeof_s: usize = 512;
        let mut t: time_t = 0;
        let mut idx = 0;
        let mut found = null_mut();

        'found: {
            let mut o = options_parse_get(global_options, key, &raw mut idx, 0);
            if o.is_null() && !(*ft).wp.is_null() {
                o = options_parse_get((*(*ft).wp).options, key, &raw mut idx, 0);
            }
            if o.is_null() && !(*ft).w.is_null() {
                o = options_parse_get((*(*ft).w).options, key, &raw mut idx, 0);
            }
            if o.is_null() {
                o = options_parse_get(global_w_options, key, &raw mut idx, 0);
            }
            if o.is_null() && !(*ft).s.is_null() {
                o = options_parse_get((*(*ft).s).options, key, &raw mut idx, 0);
            }
            if o.is_null() {
                o = options_parse_get(global_s_options, key, &raw mut idx, 0);
            }
            if !o.is_null() {
                found = options_to_string(o, idx, 1);
                break 'found;
            }

            let mut fte = format_table_get(key);
            if !fte.is_null() {
                let value = (*fte).cb.unwrap()(ft);
                if (*fte).type_ == format_table_type::FORMAT_TABLE_TIME && !value.is_null() {
                    t = (*value.cast::<timeval>()).tv_sec;
                } else {
                    found = value.cast();
                }
                break 'found;
            }
            (*fe_find.as_mut_ptr()).key = key.cast_mut(); // TODO: check if this is correct casting away const
            let mut fe = rb_find(&raw mut (*ft).tree, fe_find.as_mut_ptr());
            if !fe.is_null() {
                if (*fe).time != 0 {
                    t = (*fe).time;
                    break 'found;
                }
                if (*fe).value.is_null() && (*fe).cb.is_some() {
                    (*fe).value = (*fe).cb.unwrap()(ft).cast();
                    if (*fe).value.is_null() {
                        (*fe).value = xstrdup(c"".as_ptr()).as_ptr();
                    }
                }
                found = xstrdup((*fe).value).as_ptr();
                break 'found;
            }

            if !modifiers.intersects(format_modifiers::FORMAT_TIMESTRING) {
                let mut envent = null_mut();
                if !(*ft).s.is_null() {
                    envent = environ_find((*(*ft).s).environ, key);
                }
                if envent.is_null() {
                    envent = environ_find(global_environ, key);
                }
                if !envent.is_null() && (*envent).value.is_some() {
                    found = xstrdup((*envent).value.unwrap().as_ptr()).as_ptr();
                    break 'found;
                }
            }

            return null_mut();
        }
        // found
        if modifiers.intersects(format_modifiers::FORMAT_TIMESTRING) {
            if t == 0 && !found.is_null() {
                t = strtonum(found, 0, i64::MAX, &raw mut errstr);
                if !errstr.is_null() {
                    t = 0;
                }
                free_(found);
            }
            if (t == 0) {
                return null_mut();
            }
            if modifiers.intersects(format_modifiers::FORMAT_PRETTY) {
                found = format_pretty_time(t, 0);
            } else {
                if !time_format.is_null() {
                    let mut tm = MaybeUninit::<tm>::uninit();
                    let mut tm = tm.as_mut_ptr();

                    localtime_r(&raw const t, tm);
                    strftime(s, sizeof_s, time_format, tm);
                } else {
                    ctime_r(&raw const t, s);
                    *s.add(strcspn(s, c"\n".as_ptr())) = b'\0' as c_char;
                }
                found = xstrdup(s).as_ptr();
            }
            return found;
        }

        if t != 0 {
            xasprintf(&raw mut found, c"%lld".as_ptr(), t as i64);
        } else if found.is_null() {
            return null_mut();
        }
        let mut saved: *mut c_char = null_mut();
        if modifiers.intersects(format_modifiers::FORMAT_BASENAME) {
            saved = found;
            found = xstrdup(basename(saved)).as_ptr();
            free_(saved);
        }
        if modifiers.intersects(format_modifiers::FORMAT_DIRNAME) {
            saved = found;
            found = xstrdup(libc::dirname(saved)).as_ptr();
            free_(saved);
        }
        if modifiers.intersects(format_modifiers::FORMAT_QUOTE_SHELL) {
            saved = found;
            found = format_quote_shell(saved);
            free_(saved);
        }
        if modifiers.intersects(format_modifiers::FORMAT_QUOTE_STYLE) {
            saved = found;
            found = format_quote_style(saved);
            free_(saved);
        }
        found
    }
}

/* Unescape escaped characters. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_unescape(mut s: *const c_char) -> *mut c_char {
    unsafe {
        let mut cp = xmalloc(strlen(s) + 1).as_ptr().cast();
        let mut out = cp;
        let mut brackets = 0;
        while *s != b'\0' as c_char {
            if (*s == b'#' as c_char && *s.add(1) == b'{' as c_char) {
                brackets += 1;
            }
            if (brackets == 0 && *s == b'#' as c_char && !strchr(c",#{}:".as_ptr(), *s.add(1) as i32).is_null()) {
                s = s.add(1);
                *cp = *s;
                cp = cp.add(1);
                continue;
            }
            if (*s == b'}' as c_char) {
                brackets -= 1;
            }
            *cp = *s;
            cp = cp.add(1);
        }
        *cp = b'\0' as c_char;
        out
    }
}

/// Remove escaped characters.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_strip(mut s: *const c_char) -> *mut c_char {
    unsafe {
        let mut out = xmalloc(strlen(s) + 1).as_ptr().cast();
        let mut cp = out;
        let mut brackets = 0;

        while *s != b'\0' as c_char {
            if (*s == b'#' as c_char && *s.add(1) == b'{' as c_char) {
                brackets += 1;
            }
            if (*s == b'#' as c_char && !strchr(c",#{}:".as_ptr(), *s.add(1) as i32).is_null()) {
                if (brackets != 0) {
                    *cp = *s;
                    cp = cp.add(1);
                }
                s = s.add(1);
                continue;
            }
            if (*s == b'}' as c_char) {
                brackets -= 1;
            }
            *cp = *s;
            cp = cp.add(1);
            s = s.add(1);
        }
        *cp = b'\0' as c_char;
        out
    }
}

/* Skip until end. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_skip(mut s: *const c_char, end: *const c_char) -> *const c_char {
    unsafe {
        let mut brackets = 0;

        while *s != b'\0' as c_char {
            if (*s == b'#' as c_char && *s.add(1) == b'{' as c_char) {
                brackets += 1;
            }
            if *s == b'#' as c_char && *s.add(1) != b'\0' as c_char && !strchr(c",#{}:".as_ptr(), *s.add(1) as i32).is_null() {
                s = s.add(2);
                continue;
            }
            if (*s == b'}' as c_char) {
                brackets -= 1;
            }
            if !strchr(end, *s as i32).is_null() && brackets == 0 {
                break;
            }
            s = s.add(1);
        }
        if *s == b'\0' as c_char {
            return null_mut();
        }
        s
    }
}

/* Return left and right alternatives separated by commas. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_choose(es: *mut format_expand_state, s: *const c_char, left: *mut *mut c_char, right: *mut *mut c_char, expand: c_int) -> c_int {
    unsafe {
        let mut cp: *const c_char = format_skip(s, c",".as_ptr());
        if cp.is_null() {
            return -1;
        }
        let left0 = xstrndup(s, cp.offset_from(s) as usize).as_ptr();
        let right0 = xstrdup(cp.add(1)).as_ptr();

        if (expand != 0) {
            *left = format_expand1(es, left0);
            free_(left0);
            *right = format_expand1(es, right0);
            free_(right0);
        } else {
            *left = left0;
            *right = right0;
        }
        0
    }
}

/* Is this true? */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_true(s: *const c_char) -> c_int {
    unsafe {
        if !s.is_null() && *s != b'\0' as c_char && (*s != b'0' as c_char || *s.add(1) != b'\0' as c_char) {
            return 1;
        }
        0
    }
}

/* Check if modifier end. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_is_end(c: c_char) -> boolint { unsafe { boolint::from(c == b';' as c_char || c == b':' as c_char) } }

/* Add to modifier list. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_add_modifier(list: *mut *mut format_modifier, count: *mut u32, c: *const c_char, n: usize, argv: *mut *mut c_char, argc: i32) {
    unsafe {
        let mut fm: *mut format_modifier = null_mut();

        *list = xreallocarray_(*list, (*count) as usize + 1).as_ptr();
        fm = (*list).add(*count as usize);
        (*count) += 1;

        memcpy((*fm).modifier.as_mut_ptr().cast(), c.cast(), n);
        (*fm).modifier[n] = b'\0' as c_char;
        (*fm).size = n as u32;

        (*fm).argv = argv;
        (*fm).argc = argc;
    }
}

/// Free modifier list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_free_modifiers(list: *mut format_modifier, count: u32) {
    unsafe {
        for i in 0..count as usize {
            cmd_free_argv((*list.add(i)).argc, (*list.add(i)).argv);
        }
        free_(list);
    }
}

/// Build modifier list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_build_modifiers(es: *mut format_expand_state, s: *mut *const c_char, count: *mut u32) -> *mut format_modifier {
    unsafe {
        let mut cp = *s;
        let mut end: *const c_char = null();
        let mut list: *mut format_modifier = null_mut();

        let mut last: [c_char; 3] = [b'X' as c_char, b';' as c_char, b':' as c_char];
        let mut last: *mut c_char = last.as_mut_ptr();

        // char c, last[] = "X;:", **argv, *value;
        // int argc;
        let mut argv: *mut *mut c_char = null_mut();
        let mut argc = 0;
        let mut c: c_char = 0;

        /*
         * Modifiers are a ; separated list of the forms:
         *      l,m,C,a,b,c,d,n,t,w,q,E,T,S,W,P,<,>
         *	=a
         *	=/a
         *      =/a/
         *	s/a/b/
         *	s/a/b
         *	||,&&,!=,==,<=,>=
         */

        *count = 0;

        while (*cp != b'\0' as c_char && *cp != b':' as c_char) {
            /* Skip any separator character. */
            if (*cp == b';' as c_char) {
                cp = cp.add(1);
            }

            /* Check single character modifiers with no arguments. */
            if !strchr(c"labcdnwETSWPL<>".as_ptr(), *cp as i32).is_null() && format_is_end(*cp.add(1)).as_bool() {
                format_add_modifier(&raw mut list, count, cp, 1, null_mut(), 0);
                cp = cp.add(1);
                continue;
            }

            /* Then try double character with no arguments. */
            if ((memcmp(c"||".as_ptr().cast(), cp.cast(), 2) == 0
                || memcmp(c"&&".as_ptr().cast(), cp.cast(), 2) == 0
                || memcmp(c"!=".as_ptr().cast(), cp.cast(), 2) == 0
                || memcmp(c"==".as_ptr().cast(), cp.cast(), 2) == 0
                || memcmp(c"<=".as_ptr().cast(), cp.cast(), 2) == 0
                || memcmp(c">=".as_ptr().cast(), cp.cast(), 2) == 0)
                && format_is_end(*cp.add(2)).as_bool())
            {
                format_add_modifier(&raw mut list, count, cp, 2, null_mut(), 0);
                cp = cp.add(2);
                continue;
            }

            /* Now try single character with arguments. */
            if (strchr(c"mCNst=peq".as_ptr(), *cp as i32).is_null()) {
                break;
            }
            c = *cp;

            /* No arguments provided. */
            if (format_is_end(*cp.add(1)).as_bool()) {
                format_add_modifier(&raw mut list, count, cp, 1, null_mut(), 0);
                cp = cp.add(1);
                continue;
            }
            argv = null_mut();
            argc = 0;

            /* Single argument with no wrapper character. */
            if (ispunct(*cp.add(1) as i32) == 0 || *cp.add(1) == b'-' as c_char) {
                let end: *const c_char = format_skip(cp.add(1), c":;".as_ptr());
                if (end.is_null()) {
                    break;
                }

                argv = xcalloc1();
                let value = xstrndup(cp.add(1), end.offset_from(cp.add(1)) as usize).as_ptr();
                *argv = format_expand1(es, value);
                free_(value);
                argc = 1;

                format_add_modifier(&raw mut list, count, &raw mut c, 1, argv, argc);
                cp = end;
                continue;
            }

            /* Multiple arguments with a wrapper character. */
            *last = *cp.add(1);
            cp = cp.add(1);
            loop {
                if (*cp == *last && format_is_end(*cp.add(1)).as_bool()) {
                    cp = cp.add(1);
                    break;
                }
                end = format_skip(cp.add(1), last);
                if (end.is_null()) {
                    break;
                }
                cp = cp.add(1);

                argv = xreallocarray_(argv, argc as usize + 1).as_ptr();
                let value = xstrndup(cp, end.offset_from(cp) as usize).as_ptr();
                *argv.add(argc as usize) = format_expand1(es, value);
                argc += 1;
                free_(value);

                cp = end;
                if (format_is_end(*cp).as_bool()) {
                    break;
                }
            }
            format_add_modifier(&raw mut list, count, &raw mut c, 1, argv, argc);
        }
        if (*cp != b':' as c_char) {
            format_free_modifiers(list, *count);
            *count = 0;
            return null_mut();
        }
        *s = cp.add(1);
        list
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_match(fm: *mut format_modifier, pattern: *const c_char, text: *const c_char) -> *mut c_char {
    unsafe {
        let mut s = c"".as_ptr() as *const c_char;
        let mut r = MaybeUninit::<regex_t>::uninit();
        let mut r = r.as_mut_ptr();
        let mut flags: i32 = 0;

        if (*fm).argc >= 1 {
            s = *(*fm).argv;
        }
        if strchr(s, b'r' as i32).is_null() {
            if !strchr(s, b'i' as i32).is_null() {
                flags |= FNM_CASEFOLD;
            }
            if libc::fnmatch(pattern, text, flags) != 0 {
                return xstrdup(c"0".as_ptr()).as_ptr();
            }
        } else {
            flags = REG_EXTENDED | REG_NOSUB;
            if !strchr(s, b'i' as i32).is_null() {
                flags |= REG_ICASE;
            }
            if regcomp(r, pattern, flags) != 0 {
                return xstrdup(c"0".as_ptr()).as_ptr();
            }
            if regexec(r, text, 0, null_mut(), 0) != 0 {
                regfree(r);
                return xstrdup(c"0".as_ptr()).as_ptr();
            }
            regfree(r);
        }
        xstrdup(c"1".as_ptr()).as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_sub(fm: *mut format_modifier, text: *const c_char, pattern: *const c_char, with: *const c_char) -> *mut c_char {
    unsafe {
        let mut flags: i32 = REG_EXTENDED;

        if (*fm).argc >= 3 && !strchr(*(*fm).argv.add(2), b'i' as i32).is_null() {
            flags |= REG_ICASE;
        }
        let value = regsub(pattern, with, text, flags);
        if value.is_null() { xstrdup(text).as_ptr() } else { value }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_search(fm: *mut format_modifier, wp: *mut window_pane, s: *const c_char) -> *mut c_char {
    unsafe {
        let mut ignore = 0;
        let mut regex = 0;
        let mut value: *mut c_char = null_mut();

        if (*fm).argc >= 1 {
            if !strchr(*(*fm).argv, b'i' as i32).is_null() {
                ignore = 1;
            }
            if !strchr(*(*fm).argv, b'r' as i32).is_null() {
                regex = 1;
            }
        }
        xasprintf(&mut value, c"%u".as_ptr(), window_pane_search(wp, s, regex, ignore));
        value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_session_name(es: *mut format_expand_state, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let name = format_expand1(es, fmt);
        let mut s: *mut session = null_mut();

        for s in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            if strcmp((*s).name, name) == 0 {
                free_(name);
                return xstrdup(c"1".as_ptr()).as_ptr();
            }
        }

        free_(name);
        xstrdup(c"0".as_ptr()).as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_loop_sessions(es: *mut format_expand_state, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let ft = (*es).ft;
        let c = (*ft).client;
        let item = (*ft).item;
        let mut value: *mut c_char = xcalloc(1, 1).as_ptr().cast();
        let mut valuelen = 1;

        for s in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            format_log1(es, c"format_loop_sessions".as_ptr(), (c"session loop: $%u".as_ptr()), ((*s).id));
            let nft = format_create(c, item, FORMAT_NONE, (*ft).flags);
            format_defaults(nft, (*ft).c, NonNull::new(s), None, None);
            let mut next = zeroed();
            format_copy_state(&mut next, es, format_expand_flags::empty());
            next.ft = nft;
            let expanded = format_expand1(&mut next, fmt);
            format_free(next.ft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();
            strlcat(value, expanded, valuelen);
            free_(expanded);
        }

        value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_window_name(es: *mut format_expand_state, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let ft = (*es).ft;
        if (*ft).s.is_null() {
            format_log1(es, c"format_window_name".as_ptr(), (c"window name but no session".as_ptr()));
            return null_mut();
        }

        let name = format_expand1(es, fmt);
        for wl in rb_foreach(&raw mut (*(*ft).s).windows).map(NonNull::as_ptr) {
            if strcmp((*(*wl).window).name, name) == 0 {
                free_(name);
                return xstrdup(c"1".as_ptr()).as_ptr();
            }
        }
        free_(name);
        xstrdup(c"0".as_ptr()).as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_loop_windows(es: *mut format_expand_state, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let ft = (*es).ft;
        let c = (*ft).client;
        let item = (*ft).item;
        let mut all: *mut c_char = null_mut();
        let mut active: *mut c_char = null_mut();
        let mut value: *mut c_char = xcalloc(1, 1).as_ptr().cast();
        let mut valuelen = 1;

        if (*ft).s.is_null() {
            format_log1(es, c"format_loop_windows".as_ptr(), (c"window loop but no session".as_ptr()));
            return null_mut();
        }

        if format_choose(es, fmt, &mut all, &mut active, 0) != 0 {
            all = xstrdup(fmt).as_ptr();
            active = null_mut();
        }

        for wl in rb_foreach(&raw mut (*(*ft).s).windows).map(NonNull::as_ptr) {
            let w = (*wl).window;
            format_log1(es, c"format_loop_windows".as_ptr(), (c"window loop: %u @%u".as_ptr()), ((*wl).idx), ((*w).id));
            let use_ = if !active.is_null() && wl == (*(*ft).s).curw { active } else { all };

            let nft = format_create(c, item, FORMAT_WINDOW as i32 | (*w).id as i32, (*ft).flags);
            format_defaults(nft, (*ft).c, NonNull::new((*ft).s), NonNull::new(wl), None);
            let mut next = zeroed();
            format_copy_state(&raw mut next, es, format_expand_flags::empty());
            next.ft = nft;
            let expanded = format_expand1(&mut next, use_);
            format_free(nft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();
            strlcat(value, expanded, valuelen);
            free_(expanded);
        }

        free_(active);
        free_(all);
        value
    }
}

/// Loop over panes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_loop_panes(es: *mut format_expand_state, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let mut ft = (*es).ft;
        let mut c = (*ft).client;
        let mut item = (*ft).item;

        if ((*ft).w.is_null()) {
            format_log1(es, c"format_loop_panes".as_ptr(), c"pane loop but no window".as_ptr());
            return null_mut();
        }

        let mut all: *mut c_char = null_mut();
        let mut active: *mut c_char = null_mut();
        if (format_choose(es, fmt, &raw mut all, &raw mut active, 0) != 0) {
            all = xstrdup(fmt).as_ptr();
            active = null_mut();
        }

        let mut value: *mut c_char = xcalloc(1, 1).as_ptr().cast();
        let mut valuelen = 1;

        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let mut next = next.as_mut_ptr();
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*(*ft).w).panes).map(NonNull::as_ptr) {
            format_log1(es, c"format_loop_panes".as_ptr(), c"pane loop: %%%u".as_ptr(), (*wp).id);
            let use_ = if (!active.is_null() && wp == (*(*ft).w).active) { active } else { all };
            let nft = format_create(c, item, FORMAT_PANE as i32 | (*wp).id as i32, (*ft).flags);
            format_defaults(nft, (*ft).c, NonNull::new((*ft).s), NonNull::new((*ft).wl), NonNull::new(wp));
            format_copy_state(next, es, format_expand_flags::empty());
            (*next).ft = nft;
            let expanded = format_expand1(next, use_);
            format_free(nft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();

            strlcat(value, expanded, valuelen);
            free_(expanded);
        }

        free_(active);
        free_(all);

        value
    }
}

/// Loop over clients.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_loop_clients(es: *mut format_expand_state, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let mut ft = (*es).ft;
        let mut item = (*ft).item;
        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();

        let mut value = xcalloc(1, 1).as_ptr();
        let mut valuelen = 1;

        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            format_log1(es, c"format_loop_clients".as_ptr(), c"client loop: %s".as_ptr(), (*c).name);
            let nft = format_create(c, item, 0, (*ft).flags);
            format_defaults(nft, c, NonNull::new((*ft).s), NonNull::new((*ft).wl), NonNull::new((*ft).wp));
            format_copy_state(next, es, format_expand_flags::empty());
            (*next).ft = nft;
            let expanded = format_expand1(next, fmt);
            format_free(nft);

            valuelen += strlen(expanded);
            value = xrealloc(value.cast(), valuelen).as_ptr().cast();

            strlcat(value.cast(), expanded, valuelen);
            free_(expanded);
        }

        value.cast()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_replace_expression(mexp: *mut format_modifier, es: *mut format_expand_state, copy: *const c_char) -> *mut c_char {
    unsafe {
        let argc = (*mexp).argc;
        let mut errstr = null();

        let mut endch: *mut c_char = null_mut();
        let mut value: *mut c_char = null_mut();

        let mut left: *mut c_char = null_mut();
        let mut right: *mut c_char = null_mut();

        'fail: {
            let mut use_fp: i32 = 0;
            let mut prec: u32 = 0;

            let mut mleft: f64 = 0.0;
            let mut mright: f64 = 0.0;
            let mut result: f64 = 0.0;

            enum Operator {
                Add,
                Subtract,
                Multiply,
                Divide,
                Modulus,
                Equal,
                NotEqual,
                GreaterThan,
                GreaterThanEqual,
                LessThan,
                LessThanEqual,
            }

            let mut operator;

            if strcmp(*(*mexp).argv, c"+".as_ptr()) == 0 {
                operator = Operator::Add;
            } else if strcmp(*(*mexp).argv, c"-".as_ptr()) == 0 {
                operator = Operator::Subtract;
            } else if strcmp(*(*mexp).argv, c"*".as_ptr()) == 0 {
                operator = Operator::Multiply;
            } else if strcmp(*(*mexp).argv, c"/".as_ptr()) == 0 {
                operator = Operator::Divide;
            } else if strcmp(*(*mexp).argv, c"%".as_ptr()) == 0 || strcmp(*(*mexp).argv, c"m".as_ptr()) == 0 {
                operator = Operator::Modulus;
            } else if strcmp(*(*mexp).argv, c"==".as_ptr()) == 0 {
                operator = Operator::Equal;
            } else if strcmp(*(*mexp).argv, c"!=".as_ptr()) == 0 {
                operator = Operator::NotEqual;
            } else if strcmp(*(*mexp).argv, c">".as_ptr()) == 0 {
                operator = Operator::GreaterThan;
            } else if strcmp(*(*mexp).argv, c"<".as_ptr()) == 0 {
                operator = Operator::LessThan;
            } else if strcmp(*(*mexp).argv, c">=".as_ptr()) == 0 {
                operator = Operator::GreaterThanEqual;
            } else if strcmp(*(*mexp).argv, c"<=".as_ptr()) == 0 {
                operator = Operator::LessThanEqual;
            } else {
                format_log1(es, c"format_replace_expression".as_ptr(), c"expression has no valid operator: '%s'".as_ptr(), *(*mexp).argv);
                break 'fail;
            }

            /* The second argument may be flags. */
            if (argc >= 2 && !strchr(*(*mexp).argv.add(1), b'f' as i32).is_null()) {
                use_fp = 1;
                prec = 2;
            }

            /* The third argument may be precision. */
            if (argc >= 3) {
                prec = strtonum(*(*mexp).argv.add(2), i32::MIN as i64, i32::MAX as i64, &raw mut errstr) as u32;
                if (!errstr.is_null()) {
                    format_log1(es, c"format_replace_expression".as_ptr(), c"expression precision %s: %s".as_ptr(), errstr, *(*mexp).argv.add(2));
                    break 'fail;
                }
            }

            if (format_choose(es, copy, &raw mut left, &raw mut right, 1) != 0) {
                format_log1(es, c"format_replace_expression".as_ptr(), c"expression syntax error".as_ptr());
                break 'fail;
            }

            mleft = strtod(left, &raw mut endch);
            if (*endch != b'\0' as c_char) {
                format_log1(es, c"format_replace_expression".as_ptr(), c"expression left side is invalid: %s".as_ptr(), left);
                break 'fail;
            }

            mright = strtod(right, &raw mut endch);
            if (*endch != b'\0' as c_char) {
                format_log1(es, c"format_replace_expression".as_ptr(), c"expression right side is invalid: %s".as_ptr(), right);
                break 'fail;
            }

            if use_fp == 0 {
                mleft = (mleft as c_longlong) as f64;
                mright = (mright as c_longlong) as f64;
            }
            format_log1(es, c"format_replace_expression".as_ptr(), (c"expression left side is: %.*f".as_ptr()), prec, mleft);
            format_log1(es, c"format_replace_expression".as_ptr(), (c"expression right side is: %.*f".as_ptr()), prec, mright);

            result = match operator {
                Operator::Add => mleft + mright,
                Operator::Subtract => mleft - mright,
                Operator::Multiply => mleft * mright,
                Operator::Divide => mleft / mright,
                Operator::Modulus => mleft % mright,
                Operator::Equal => ((mleft - mright).abs() < 1e-9) as i32 as f64,
                Operator::NotEqual => ((mleft - mright).abs() > 1e-9) as i32 as f64,
                Operator::GreaterThan => (mleft > mright) as i32 as f64,
                Operator::GreaterThanEqual => (mleft >= mright) as i32 as f64,
                Operator::LessThan => (mleft < mright) as i32 as f64,
                Operator::LessThanEqual => (mleft <= mright) as i32 as f64,
            };

            if (use_fp != 0) {
                xasprintf(&raw mut value, c"%.*f".as_ptr(), prec, result);
            } else {
                xasprintf(&raw mut value, c"%.*f".as_ptr(), prec, (result as c_longlong) as f64);
            }
            format_log1(es, c"format_replace_expression".as_ptr(), c"expression result is %s".as_ptr(), value);

            free_(right);
            free_(left);
            return value;
        }

        // fail:
        free_(right);
        free_(left);
        null_mut()
    }
}

/// Replace a key.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_replace(es: *mut format_expand_state, key: *const c_char, keylen: usize, buf: *mut *mut c_char, len: *mut usize, off: *mut usize) -> i32 {
    let __func__: *const c_char = c"format_replace".as_ptr();

    unsafe {
        let mut ft = (*es).ft;
        let mut wp = (*ft).wp;
        let mut errstr: *const c_char = null();
        let mut copy: *const c_char = null();
        let mut cp: *const c_char = null();
        let mut marker: *const c_char = null();

        let mut time_format: *const c_char = null();

        let mut copy0: *mut c_char = null_mut();
        let mut condition: *mut c_char = null_mut();
        let mut found: *mut c_char = null_mut();
        let mut new: *mut c_char = null_mut();
        let mut value: *mut c_char = null_mut();
        let mut left: *mut c_char = null_mut();
        let mut right: *mut c_char = null_mut();

        let mut valuelen = 0;

        let mut modifiers: format_modifiers = format_modifiers::empty();
        let mut limit: i32 = 0;
        let mut width: i32 = 0;

        //let mut j = 0i32;
        let mut c = 0i32;

        let mut list: *mut format_modifier = null_mut();
        let mut cmp: *mut format_modifier = null_mut();
        let mut search: *mut format_modifier = null_mut();

        let mut sub: *mut *mut format_modifier = null_mut();
        let mut mexp: *mut format_modifier = null_mut();
        let mut fm: *mut format_modifier = null_mut();

        //let mut i = 0u32;
        let mut count = 0u32;
        let mut nsub = 0u32;

        let mut next = MaybeUninit::<format_expand_state>::uninit();
        let next = next.as_mut_ptr();

        'fail: {
            'done: {
                // Make a copy of the key.
                copy0 = xstrndup(key, keylen).as_ptr();
                copy = copy0;

                // Process modifier list.
                list = format_build_modifiers(es, &raw mut copy, &raw mut count);
                for i in 0..count {
                    fm = list.add(i as usize);
                    if format_logging(ft).as_bool() {
                        format_log1(es, __func__, c"modifier %u is %s".as_ptr(), i, (*fm).modifier);
                        for j in 0..(*fm).argc {
                            format_log1(es, __func__, c"modifier %u argument %d: %s".as_ptr(), i, j, *(*fm).argv.add(j as usize));
                        }
                    }
                    if ((*fm).size == 1) {
                        match (*fm).modifier[0] as u8 {
                            b'm' | b'<' | b'>' => cmp = fm,
                            b'C' => search = fm,
                            b's' => {
                                if (*fm).argc < 2 {
                                } else {
                                    sub = xreallocarray_(sub, nsub as usize + 1).as_ptr();
                                    *sub.add(nsub as usize) = fm;
                                    nsub += 1;
                                }
                            }
                            b'=' => {
                                if (*fm).argc < 1 {
                                } else {
                                    limit = strtonum(*(*fm).argv, i32::MIN as i64, i32::MAX as i64, &raw mut errstr) as i32;
                                    if (!errstr.is_null()) {
                                        limit = 0;
                                    }
                                    if ((*fm).argc >= 2 && !(*(*fm).argv.add(1)).is_null()) {
                                        marker = *(*fm).argv.add(1);
                                    }
                                }
                            }
                            b'p' => {
                                if ((*fm).argc < 1) {
                                    break;
                                } else {
                                    width = strtonum(*(*fm).argv, i32::MIN as i64, i32::MAX as i64, &raw mut errstr) as i32;
                                    if (!errstr.is_null()) {
                                        width = 0;
                                    }
                                }
                            }
                            b'w' => modifiers |= format_modifiers::FORMAT_WIDTH,
                            b'e' => {
                                if ((*fm).argc < 1 || (*fm).argc > 3) {
                                } else {
                                    mexp = fm;
                                }
                            }
                            b'l' => modifiers |= format_modifiers::FORMAT_LITERAL,
                            b'a' => modifiers |= format_modifiers::FORMAT_CHARACTER,
                            b'b' => modifiers |= format_modifiers::FORMAT_BASENAME,
                            b'c' => modifiers |= format_modifiers::FORMAT_COLOUR,
                            b'd' => modifiers |= format_modifiers::FORMAT_DIRNAME,
                            b'n' => modifiers |= format_modifiers::FORMAT_LENGTH,
                            b't' => {
                                modifiers |= format_modifiers::FORMAT_TIMESTRING;
                                if ((*fm).argc < 1) {
                                } else {
                                    if (!strchr(*(*fm).argv, b'p' as i32).is_null()) {
                                        modifiers |= format_modifiers::FORMAT_PRETTY;
                                    } else if ((*fm).argc >= 2 && !strchr(*(*fm).argv, b'f' as i32).is_null()) {
                                        time_format = format_strip(*(*fm).argv.add(1));
                                    }
                                }
                            }
                            b'q' => {
                                if ((*fm).argc < 1) {
                                    modifiers |= format_modifiers::FORMAT_QUOTE_SHELL;
                                } else if (!strchr(*(*fm).argv, b'e' as i32).is_null() || !strchr(*(*fm).argv, b'h' as i32).is_null()) {
                                    modifiers |= format_modifiers::FORMAT_QUOTE_STYLE;
                                }
                            }
                            b'E' => modifiers |= format_modifiers::FORMAT_EXPAND,
                            b'T' => modifiers |= format_modifiers::FORMAT_EXPANDTIME,
                            b'N' => {
                                if ((*fm).argc < 1 || !strchr(*(*fm).argv, b'w' as i32).is_null()) {
                                    modifiers |= format_modifiers::FORMAT_WINDOW_NAME;
                                } else if (!strchr(*(*fm).argv, b's' as i32).is_null()) {
                                    modifiers |= format_modifiers::FORMAT_SESSION_NAME;
                                }
                            }
                            b'S' => modifiers |= format_modifiers::FORMAT_SESSIONS,
                            b'W' => modifiers |= format_modifiers::FORMAT_WINDOWS,
                            b'P' => modifiers |= format_modifiers::FORMAT_PANES,
                            b'L' => modifiers |= format_modifiers::FORMAT_CLIENTS,
                            _ => (),
                        }
                    } else if ((*fm).size == 2) {
                        if (strcmp((*fm).modifier.as_ptr(), c"||".as_ptr()) == 0
                            || strcmp((*fm).modifier.as_ptr(), c"&&".as_ptr()) == 0
                            || strcmp((*fm).modifier.as_ptr(), c"==".as_ptr()) == 0
                            || strcmp((*fm).modifier.as_ptr(), c"!=".as_ptr()) == 0
                            || strcmp((*fm).modifier.as_ptr(), c">=".as_ptr()) == 0
                            || strcmp((*fm).modifier.as_ptr(), c"<=".as_ptr()) == 0)
                        {
                            cmp = fm;
                        }
                    }
                }

                /* Is this a literal string? */
                if (modifiers.intersects(format_modifiers::FORMAT_LITERAL)) {
                    format_log1(es, __func__, c"literal string is '%s'".as_ptr(), copy);
                    value = format_unescape(copy);
                    break 'done;
                }

                /* Is this a character? */
                if (modifiers.intersects(format_modifiers::FORMAT_CHARACTER)) {
                    new = format_expand1(es, copy);
                    c = strtonum(new, 32, 126, &raw mut errstr) as i32;
                    if (!errstr.is_null()) {
                        value = xstrdup(c"".as_ptr()).as_ptr();
                    } else {
                        xasprintf(&raw mut value, c"%c".as_ptr(), c);
                    }
                    free_(new);
                    break 'done;
                }

                // Is this a colour?
                if (modifiers.intersects(format_modifiers::FORMAT_COLOUR)) {
                    new = format_expand1(es, copy);
                    c = colour_fromstring(new);
                    if c == -1
                        || ({
                            c = colour_force_rgb(c);
                            c == -1
                        })
                    {
                        value = xstrdup(c"".as_ptr()).as_ptr();
                    } else {
                        xasprintf(&raw mut value, c"%06x".as_ptr(), c & 0xffffff);
                    }
                    free_(new);
                    break 'done;
                }

                /* Is this a loop, comparison or condition? */
                if (modifiers.intersects(format_modifiers::FORMAT_SESSIONS)) {
                    value = format_loop_sessions(es, copy);
                    if (value.is_null()) {
                        break 'fail;
                    }
                } else if (modifiers.intersects(format_modifiers::FORMAT_WINDOWS)) {
                    value = format_loop_windows(es, copy);
                    if (value.is_null()) {
                        break 'fail;
                    }
                } else if (modifiers.intersects(format_modifiers::FORMAT_PANES)) {
                    value = format_loop_panes(es, copy);
                    if (value.is_null()) {
                        break 'fail;
                    }
                } else if (modifiers.intersects(format_modifiers::FORMAT_CLIENTS)) {
                    value = format_loop_clients(es, copy);
                    if (value.is_null()) {
                        break 'fail;
                    }
                } else if (modifiers.intersects(format_modifiers::FORMAT_WINDOW_NAME)) {
                    value = format_window_name(es, copy);
                    if (value.is_null()) {
                        break 'fail;
                    }
                } else if (modifiers.intersects(format_modifiers::FORMAT_SESSION_NAME)) {
                    value = format_session_name(es, copy);
                    if (value.is_null()) {
                        break 'fail;
                    }
                } else if (!search.is_null()) {
                    /* Search in pane. */
                    new = format_expand1(es, copy);
                    if (wp.is_null()) {
                        format_log1(es, __func__, c"search '%s' but no pane".as_ptr(), new);
                        value = xstrdup(c"0".as_ptr()).as_ptr();
                    } else {
                        format_log1(es, __func__, c"search '%s' pane %%%u".as_ptr(), new, (*wp).id);
                        value = format_search(search, wp, new);
                    }
                    free_(new);
                } else if (!cmp.is_null()) {
                    /* Comparison of left and right. */
                    if (format_choose(es, copy, &raw mut left, &raw mut right, 1) != 0) {
                        format_log1(es, __func__, c"compare %s syntax error: %s".as_ptr(), (*cmp).modifier, copy);
                        break 'fail;
                    }
                    format_log1(es, __func__, c"compare %s left is: %s".as_ptr(), (*cmp).modifier, left);
                    format_log1(es, __func__, c"compare %s right is: %s".as_ptr(), (*cmp).modifier, right);

                    if (strcmp((*cmp).modifier.as_ptr(), c"||".as_ptr()) == 0) {
                        if (format_true(left) != 0 || format_true(right) != 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c"&&".as_ptr()) == 0) {
                        if (format_true(left) != 0 && format_true(right) != 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c"==".as_ptr()) == 0) {
                        if (strcmp(left, right) == 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c"!=".as_ptr()) == 0) {
                        if (strcmp(left, right) != 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c"<".as_ptr()) == 0) {
                        if (strcmp(left, right) < 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c">".as_ptr()) == 0) {
                        if (strcmp(left, right) > 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c"<=".as_ptr()) == 0) {
                        if (strcmp(left, right) <= 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c">=".as_ptr()) == 0) {
                        if (strcmp(left, right) >= 0) {
                            value = xstrdup(c"1".as_ptr()).as_ptr();
                        } else {
                            value = xstrdup(c"0".as_ptr()).as_ptr();
                        }
                    } else if (strcmp((*cmp).modifier.as_ptr(), c"m".as_ptr()) == 0) {
                        value = format_match(cmp, left, right);
                    }

                    free_(right);
                    free_(left);
                } else if (*copy == b'?' as c_char) {
                    /* Conditional: check first and choose second or third. */
                    cp = format_skip(copy.add(1), c",".as_ptr());
                    if (cp.is_null()) {
                        format_log1(es, __func__, c"condition syntax error: %s".as_ptr(), copy.add(1));
                        break 'fail;
                    }
                    condition = xstrndup(copy.add(1), cp.offset_from(copy.add(1)) as usize).as_ptr();
                    format_log1(es, __func__, c"condition is: %s".as_ptr(), condition);

                    found = format_find(ft, condition, modifiers, time_format);
                    if (found.is_null()) {
                        /*
                         * If the condition not found, try to expand it. If
                         * the expansion doesn't have any effect, then assume
                         * false.
                         */
                        found = format_expand1(es, condition);
                        if (strcmp(found, condition) == 0) {
                            free_(found);
                            found = xstrdup(c"".as_ptr()).as_ptr();
                            format_log1(es, __func__, c"condition '%s' not found; assuming false".as_ptr(), condition);
                        }
                    } else {
                        format_log1(es, __func__, c"condition '%s' found: %s".as_ptr(), condition, found);
                    }

                    if (format_choose(es, cp.add(1), &raw mut left, &raw mut right, 0) != 0) {
                        format_log1(es, __func__, c"condition '%s' syntax error: %s".as_ptr(), condition, cp.add(1));
                        free_(found);
                        break 'fail;
                    }
                    if (format_true(found) != 0) {
                        format_log1(es, __func__, c"condition '%s' is true".as_ptr(), condition);
                        value = format_expand1(es, left);
                    } else {
                        format_log1(es, __func__, c"condition '%s' is false".as_ptr(), condition);
                        value = format_expand1(es, right);
                    }
                    free_(right);
                    free_(left);

                    free_(condition);
                    free_(found);
                } else if (!mexp.is_null()) {
                    value = format_replace_expression(mexp, es, copy);
                    if (value.is_null()) {
                        value = xstrdup(c"".as_ptr()).as_ptr();
                    }
                } else {
                    if (!strstr(copy, c"#{".as_ptr()).is_null()) {
                        format_log1(es, __func__, c"expanding inner format '%s'".as_ptr(), copy);
                        value = format_expand1(es, copy);
                    } else {
                        value = format_find(ft, copy, modifiers, time_format);
                        if (value.is_null()) {
                            format_log1(es, __func__, c"format '%s' not found".as_ptr(), copy);
                            value = xstrdup(c"".as_ptr()).as_ptr();
                        } else {
                            format_log1(es, __func__, c"format '%s' found: %s".as_ptr(), copy, value);
                        }
                    }
                }
            }
            // done:

            // Expand again if required.
            if (modifiers.intersects(format_modifiers::FORMAT_EXPAND)) {
                new = format_expand1(es, value);
                free_(value);
                value = new;
            } else if (modifiers.intersects(format_modifiers::FORMAT_EXPANDTIME)) {
                format_copy_state(next, es, format_expand_flags::FORMAT_EXPAND_TIME);
                new = format_expand1(next, value);
                free_(value);
                value = new;
            }

            // Perform substitution if any.
            for i in 0..nsub {
                left = format_expand1(es, *(**sub.add(i as usize)).argv);
                right = format_expand1(es, *(**sub.add(i as usize)).argv.add(1));
                new = format_sub(*sub.add(i as usize), value, left, right);
                format_log1(es, __func__, c"substitute '%s' to '%s': %s".as_ptr(), left, right, new);
                free_(value);
                value = new;
                free_(right);
                free_(left);
            }

            // Truncate the value if needed.
            if (limit > 0) {
                new = format_trim_left(value, limit as u32);
                if (!marker.is_null() && strcmp(new, value) != 0) {
                    free_(value);
                    xasprintf(&raw mut value, c"%s%s".as_ptr(), new, marker);
                } else {
                    free_(value);
                    value = new;
                }
                format_log1(es, __func__, c"applied length limit %d: %s".as_ptr(), limit, value);
            } else if (limit < 0) {
                new = format_trim_right(value, (-limit) as u32);
                if (!marker.is_null() && strcmp(new, value) != 0) {
                    free_(value);
                    xasprintf(&raw mut value, c"%s%s".as_ptr(), marker, new);
                } else {
                    free_(value);
                    value = new;
                }
                format_log1(es, __func__, c"applied length limit %d: %s".as_ptr(), limit, value);
            }

            /* Pad the value if needed. */
            if (width > 0) {
                new = utf8_padcstr(value, width as u32);
                free_(value);
                value = new;
                format_log1(es, __func__, c"applied padding width %d: %s".as_ptr(), width, value);
            } else if (width < 0) {
                new = utf8_rpadcstr(value, (-width) as u32);
                free_(value);
                value = new;
                format_log1(es, __func__, c"applied padding width %d: %s".as_ptr(), width, value);
            }

            /* Replace with the length or width if needed. */
            if (modifiers.intersects(format_modifiers::FORMAT_LENGTH)) {
                xasprintf(&raw mut new, c"%zu".as_ptr(), strlen(value));
                free_(value);
                value = new;
                format_log1(es, __func__, c"replacing with length: %s".as_ptr(), new);
            }
            if (modifiers.intersects(format_modifiers::FORMAT_WIDTH)) {
                xasprintf(&raw mut new, c"%u".as_ptr(), format_width(value));
                free_(value);
                value = new;
                format_log1(es, __func__, c"replacing with width: %s".as_ptr(), new);
            }

            // Expand the buffer and copy in the value.
            valuelen = strlen(value);
            while (*len - *off < valuelen + 1) {
                *buf = xreallocarray((*buf).cast(), 2, *len).as_ptr().cast();
                *len *= 2;
            }
            memcpy((*buf).add(*off).cast(), value.cast(), valuelen);
            *off += valuelen;

            format_log1(es, __func__, c"replaced '%s' with '%s'".as_ptr(), copy0, value);
            free_(value);

            free_(sub);
            format_free_modifiers(list, count);
            free_(copy0);
            return 0;
        }

        // fail:
        format_log1(es, __func__, c"failed %s".as_ptr(), copy0);

        free_(sub);
        format_free_modifiers(list, count);
        free_(copy0);
        -1
    }
}

/// Expand keys in a template.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_expand1(es: *mut format_expand_state, mut fmt: *const c_char) -> *mut c_char {
    unsafe {
        let mut ft = (*es).ft;
        // char *buf, *out, *name;
        // const char *ptr, *s, *style_end = NULL;
        // size_t off, len, n, outlen;
        // int ch, brackets;
        let mut buf: *mut c_char = null_mut();
        let mut out: *mut c_char = null_mut();

        let mut ptr: *const c_char = null();
        let mut s: *const c_char = null();
        let mut style_end: *const c_char = null();

        const sizeof_expanded: usize = 8192;
        let mut expanded = MaybeUninit::<[c_char; sizeof_expanded]>::uninit();
        let mut expanded = expanded.as_mut_ptr() as *mut c_char;

        if (fmt.is_null() || *fmt == b'\0' as c_char) {
            return xstrdup(c"".as_ptr()).as_ptr();
        }

        if ((*es).loop_ == FORMAT_LOOP_LIMIT as u32) {
            format_log1(es, c"format_expand1".as_ptr(), c"reached loop limit (%u)".as_ptr(), FORMAT_LOOP_LIMIT);
            return xstrdup(c"".as_ptr()).as_ptr();
        }
        (*es).loop_ += 1;

        format_log1(es, c"format_expand1".as_ptr(), c"expanding format: %s".as_ptr(), fmt);

        if (((*es).flags.intersects(format_expand_flags::FORMAT_EXPAND_TIME)) && !strchr(fmt, b'%' as i32).is_null()) {
            if ((*es).time == 0) {
                (*es).time = libc::time(null_mut());
                localtime_r(&raw mut (*es).time, &raw mut (*es).tm);
            }
            if (strftime(expanded, sizeof_expanded, fmt, &raw mut (*es).tm) == 0) {
                format_log1(es, c"format_expand1".as_ptr(), c"format is too long".as_ptr());
                return xstrdup(c"".as_ptr()).as_ptr();
            }
            if (format_logging(ft).as_bool() && strcmp(expanded, fmt) != 0) {
                format_log1(es, c"format_expand1".as_ptr(), c"after time expanded: %s".as_ptr(), expanded);
            }
            fmt = expanded;
        }

        let mut len = 64;
        let mut buf: *mut c_char = xmalloc(len).as_ptr().cast();
        let mut off = 0;
        let mut n = 0;

        while (*fmt != b'\0' as c_char) {
            if (*fmt != b'#' as c_char) {
                while (len - off < 2) {
                    buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                    len *= 2;
                }
                *buf.add(off) = *fmt;
                off += 1;
                fmt = fmt.add(1);
                continue;
            }
            fmt = fmt.add(1);

            let ch: u8 = (*fmt) as u8;
            fmt = fmt.add(1);
            let mut brackets = 0;

            let mut ptr: *const c_char = null_mut();
            match ch {
                b'(' => {
                    brackets = 1;
                    ptr = fmt;
                    while *ptr != b'\0' as c_char {
                        if (*ptr == b'(' as c_char) {
                            brackets += 1;
                        }
                        if (*ptr == b')' as c_char
                            && ({
                                brackets -= 1;
                                brackets == 0
                            }))
                        {
                            break;
                        }
                        ptr = ptr.add(1);
                    }
                    if (*ptr != b')' as c_char || brackets != 0) {
                        break;
                    }
                    n = ptr.offset_from(fmt) as usize;

                    let name = xstrndup(fmt, n).as_ptr();
                    format_log1(es, c"format_expand1".as_ptr(), c"found #(): %s".as_ptr(), name);

                    if (((*ft).flags.intersects(format_flags::FORMAT_NOJOBS)) || ((*es).flags.intersects(format_expand_flags::FORMAT_EXPAND_NOJOBS))) {
                        out = xstrdup(c"".as_ptr()).as_ptr();
                        format_log1(es, c"format_expand1".as_ptr(), c"#() is disabled".as_ptr());
                    } else {
                        out = format_job_get(es, name);
                        format_log1(es, c"format_expand1".as_ptr(), c"#() result: %s".as_ptr(), out);
                    }
                    free_(name);

                    let outlen = strlen(out);
                    while (len - off < outlen + 1) {
                        buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                        len *= 2;
                    }
                    memcpy(buf.add(off).cast(), out.cast(), outlen);
                    off += outlen;

                    free_(out);

                    fmt = fmt.add(n + 1);
                    continue;
                }
                b'{' => {
                    ptr = format_skip(fmt.offset(-2), c"}".as_ptr());
                    if (ptr.is_null()) {
                        break;
                    }
                    n = ptr.offset_from(fmt) as usize;

                    format_log1(es, c"format_expand1".as_ptr(), c"found #{}: %.*s".as_ptr(), n as i32, fmt);
                    if format_replace(es, fmt, n, &raw mut buf, &raw mut len, &raw mut off) != 0 {
                        break;
                    }
                    fmt = fmt.add(n + 1);
                    continue;
                }
                b'[' | b'#' => {
                    /*
                     * If ##[ (with two or more #s), then it is a style and
                     * can be left for format_draw to handle.
                     */
                    ptr = fmt.sub((ch == b'[') as usize);
                    n = 2 - (ch == b'[') as usize;
                    while (*ptr == b'#' as c_char) {
                        ptr = ptr.add(1);
                        n += 1;
                    }
                    if (*ptr == b'[' as c_char) {
                        style_end = format_skip(fmt.offset(-2), c"]".as_ptr());
                        format_log1(es, c"format_expand1".as_ptr(), c"found #*%zu[".as_ptr(), n);
                        while (len - off < n + 2) {
                            buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                            len *= 2;
                        }
                        memcpy(buf.add(off).cast(), fmt.offset(-2).cast(), n + 1);
                        off += n + 1;
                        fmt = ptr.add(1);
                        continue;
                    }
                    /* FALLTHROUGH */
                    format_log1(es, c"format_expand1".as_ptr(), (c"found #%c".as_ptr()), ch as u32);
                    while (len - off < 2) {
                        buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                        len *= 2;
                    }
                    *buf.add(off) = ch as c_char;
                    off += 1;
                    continue;
                }
                /* FALLTHROUGH */
                b'}' | b',' => {
                    format_log1(es, c"format_expand1".as_ptr(), (c"found #%c".as_ptr()), ch as u32);
                    while (len - off < 2) {
                        buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                        len *= 2;
                    }
                    *buf.add(off) = ch as c_char;
                    off += 1;
                    continue;
                }
                _ => {
                    s = null_mut();
                    if (fmt > style_end) {
                        /* skip inside #[] */
                        if (ch >= b'A' && ch <= b'Z') {
                            s = format_upper[(ch - b'A') as usize].as_ptr();
                        } else if (ch >= b'a' && ch <= b'z') {
                            s = format_lower[(ch - b'a') as usize].as_ptr();
                        }
                    }
                    if (s.is_null()) {
                        while (len - off < 3) {
                            buf = xreallocarray(buf.cast(), 2, len).as_ptr().cast();
                            len *= 2;
                        }
                        *buf.add(off) = b'#' as c_char;
                        off += 1;
                        *buf.add(off) = ch as c_char;
                        off += 1;

                        continue;
                    }
                    n = strlen(s);
                    format_log1(es, c"format_expand1".as_ptr(), c"found #%c: %s".as_ptr(), ch as u32, s);
                    if format_replace(es, s, n, &raw mut buf, &raw mut len, &raw mut off) != 0 {
                        break;
                    }
                    continue;
                }
            }

            break;
        }
        *buf.add(off) = b'\0' as c_char;

        format_log1(es, c"format_expand1".as_ptr(), c"result is: %s".as_ptr(), buf);
        (*es).loop_ -= 1;

        buf
    }
}

/// Expand keys in a template, passing through strftime first.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_expand_time(ft: *mut format_tree, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let mut es = MaybeUninit::<format_expand_state>::uninit();
        let es = es.as_mut_ptr();

        memset0(es);
        (*es).ft = ft;
        (*es).flags = format_expand_flags::FORMAT_EXPAND_TIME;
        format_expand1(es, fmt)
    }
}

/// Expand keys in a template.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_expand(ft: *mut format_tree, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let mut es = MaybeUninit::<format_expand_state>::uninit();
        let es = es.as_mut_ptr();

        memset0(es);
        (*es).ft = ft;
        (*es).flags = format_expand_flags::empty();
        format_expand1(es, fmt)
    }
}

/// Expand a single string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_single(item: *mut cmdq_item, fmt: *const c_char, c: *mut client, s: *mut session, wl: *mut winlink, wp: *mut window_pane) -> *mut c_char {
    unsafe {
        let ft = format_create_defaults(item, c, s, wl, wp);
        let expanded: *mut c_char = format_expand(ft, fmt);
        format_free(ft);
        expanded
    }
}

/// Expand a single string using state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_single_from_state(item: *mut cmdq_item, fmt: *const c_char, c: *mut client, fs: *mut cmd_find_state) -> *mut c_char { unsafe { format_single(item, fmt, c, (*fs).s, (*fs).wl, (*fs).wp) } }

/// Expand a single string using target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_single_from_target(item: *mut cmdq_item, fmt: *const c_char) -> *mut c_char {
    unsafe {
        let tc = cmdq_get_target_client(item);

        format_single_from_state(item, fmt, tc, cmdq_get_target(item))
    }
}

/// Create and add defaults.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_create_defaults(item: *mut cmdq_item, c: *mut client, s: *mut session, wl: *mut winlink, wp: *mut window_pane) -> *mut format_tree {
    unsafe {
        let ft = if !item.is_null() {
            format_create(cmdq_get_client(item), item, FORMAT_NONE, format_flags::empty())
        } else {
            format_create(null_mut(), item, FORMAT_NONE, format_flags::empty())
        };
        format_defaults(ft, c, NonNull::new(s), NonNull::new(wl), NonNull::new(wp));
        ft
    }
}

/// Create and add defaults using state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_create_from_state(item: *mut cmdq_item, c: *mut client, fs: *mut cmd_find_state) -> *mut format_tree { unsafe { format_create_defaults(item, c, (*fs).s, (*fs).wl, (*fs).wp) } }

/// Create and add defaults using target.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_create_from_target(item: *mut cmdq_item) -> *mut format_tree {
    unsafe {
        let tc = cmdq_get_target_client(item);

        format_create_from_state(item, tc, cmdq_get_target(item))
    }
}

/// Set defaults for any of arguments that are not NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults(ft: *mut format_tree, c: *mut client, s: Option<NonNull<session>>, wl: Option<NonNull<winlink>>, wp: Option<NonNull<window_pane>>) {
    unsafe {
        let mut s = transmute_ptr(s);
        let mut wl = transmute_ptr(wl);
        let mut wp = transmute_ptr(wp);

        if (!c.is_null() && !(*c).name.is_null()) {
            log_debug!("{}: c={}", function_name!(), _s((*c).name));
        } else {
            log_debug!("{}: c=none", function_name!());
        }
        if (!s.is_null()) {
            log_debug!("{}: s=${}", function_name!(), (*s).id);
        } else {
            log_debug!("{}: s=none", function_name!());
        }
        if (!wl.is_null()) {
            log_debug!("{}: wl={}", function_name!(), (*wl).idx);
        } else {
            log_debug!("{}: wl=none", function_name!());
        }
        if (!wp.is_null()) {
            log_debug!("{}: wp=%%{}", function_name!(), (*wp).id);
        } else {
            log_debug!("{}: wp=none", function_name!());
        }

        if (!c.is_null() && !s.is_null() && (*c).session != s) {
            log_debug!("{}: session does not match", function_name!());
        }

        (*ft).type_ = if (!wp.is_null()) {
            format_type::FORMAT_TYPE_PANE
        } else if (!wl.is_null()) {
            format_type::FORMAT_TYPE_WINDOW
        } else if (!s.is_null()) {
            format_type::FORMAT_TYPE_SESSION
        } else {
            format_type::FORMAT_TYPE_UNKNOWN
        };

        if (s.is_null() && !c.is_null()) {
            s = (*c).session;
        }
        if (wl.is_null() && !s.is_null()) {
            wl = (*s).curw;
        }
        if (wp.is_null() && !wl.is_null()) {
            wp = (*(*wl).window).active;
        }

        if (!c.is_null()) {
            format_defaults_client(ft, c);
        }
        if (!s.is_null()) {
            format_defaults_session(ft, s);
        }
        if (!wl.is_null()) {
            format_defaults_winlink(ft, wl);
        }
        if (!wp.is_null()) {
            format_defaults_pane(ft, wp);
        }

        let pb = paste_get_top(null_mut());
        if (!pb.is_null()) {
            format_defaults_paste_buffer(ft, pb);
        }
    }
}

/// Set default format keys for a session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults_session(ft: *mut format_tree, s: *mut session) {
    unsafe {
        (*ft).s = s;
    }
}

/// Set default format keys for a client.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults_client(ft: *mut format_tree, c: *mut client) {
    unsafe {
        if (*ft).s.is_null() {
            (*ft).s = (*c).session;
        }
        (*ft).c = c;
    }
}

/// Set default format keys for a window.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults_window(ft: *mut format_tree, w: *mut window) {
    unsafe {
        (*ft).w = w;
    }
}

/// Set default format keys for a winlink.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults_winlink(ft: *mut format_tree, wl: *mut winlink) {
    unsafe {
        if ((*ft).w.is_null()) {
            format_defaults_window(ft, (*wl).window);
        }
        (*ft).wl = wl;
    }
}

/// Set default format keys for a window pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults_pane(ft: *mut format_tree, wp: *mut window_pane) {
    unsafe {
        if ((*ft).w.is_null()) {
            format_defaults_window(ft, (*wp).window);
        }
        (*ft).wp = wp;

        if let Some(wme) = NonNull::new(tailq_first(&raw mut (*wp).modes)) {
            if let Some(formats) = (*(*wme.as_ptr()).mode).formats {
                formats(wme.as_ptr(), ft);
            }
        }
    }
}

/// Set default format keys for paste buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_defaults_paste_buffer(ft: *mut format_tree, pb: *mut paste_buffer) {
    unsafe {
        (*ft).pb = pb;
    }
}

/// Return word at given coordinates. Caller frees.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_grid_word(gd: *mut grid, mut x: u32, mut y: u32) -> *mut c_char {
    unsafe {
        let mut size = 0;
        let mut ud: *mut utf8_data = null_mut();
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let mut gc = gc.as_mut_ptr();
        let mut found = false;
        let mut s: *mut c_char = null_mut();

        let ws: *const c_char = options_get_string(global_s_options, c"word-separators".as_ptr());

        loop {
            grid_get_cell(gd, x, y, gc);
            if (*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            if utf8_cstrhas(ws, &raw mut (*gc).data) != 0 || ((*gc).data.size == 1 && (*gc).data.data[0] == b' ') {
                found = true;
                break;
            }

            if (x == 0) {
                if (y == 0) {
                    break;
                }
                let gl = grid_peek_line(gd, y - 1);
                if (!(*gl).flags.intersects(grid_line_flag::WRAPPED)) {
                    break;
                }
                y -= 1;
                x = grid_line_length(gd, y);
                if (x == 0) {
                    break;
                }
            }
            x -= 1;
        }
        loop {
            if (found) {
                let end = grid_line_length(gd, y);
                if (end == 0 || x == end - 1) {
                    if (y == (*gd).hsize + (*gd).sy - 1) {
                        break;
                    }
                    let gl = grid_peek_line(gd, y);
                    if (!(*gl).flags.intersects(grid_line_flag::WRAPPED)) {
                        break;
                    }
                    y += 1;
                    x = 0;
                } else {
                    x += 1;
                }
            }
            found = true;

            grid_get_cell(gd, x, y, gc);
            if ((*gc).flags.intersects(grid_flag::PADDING)) {
                break;
            }
            if (utf8_cstrhas(ws, &raw mut (*gc).data) != 0 || ((*gc).data.size == 1 && (*gc).data.data[0] == b' ')) {
                break;
            }

            ud = xreallocarray_(ud, size + 2).as_ptr();
            memcpy__(ud.add(size), &raw mut (*gc).data);
            size += 1;
        }
        if (size != 0) {
            (*ud.add(size)).size = 0;
            s = utf8_tocstr(ud);
            free_(ud);
        }
        s
    }
}

/// Return line at given coordinates. Caller frees.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_grid_line(gd: *mut grid, y: u32) -> *mut c_char {
    unsafe {
        let mut ud: *mut utf8_data = null_mut();
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let mut gc = gc.as_mut_ptr();
        let mut size = 0;
        let mut s: *mut c_char = null_mut();
        for x in 0..grid_line_length(gd, y) {
            grid_get_cell(gd, x, y, gc);
            if ((*gc).flags.intersects(grid_flag::PADDING)) {
                break;
            }

            ud = xreallocarray_(ud, size + 2).as_ptr();
            memcpy__(ud.add(size), &raw mut (*gc).data);
            size += 1;
        }
        if (size != 0) {
            (*ud.add(size)).size = 0;
            s = utf8_tocstr(ud);
            free_(ud);
        }
        s
    }
}

/// Return hyperlink at given coordinates. Caller frees.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn format_grid_hyperlink(gd: *mut grid, x: u32, y: u32, s: *mut screen) -> *mut c_char {
    unsafe {
        let mut uri: *const c_char = null();
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let mut gc = gc.as_mut_ptr();

        grid_get_cell(gd, x, y, gc);
        if ((*gc).flags.intersects(grid_flag::PADDING)) {
            return null_mut();
        }
        if ((*s).hyperlinks.is_null() || (*gc).link == 0) {
            return null_mut();
        }
        if hyperlinks_get((*s).hyperlinks, (*gc).link, &mut uri, null_mut(), null_mut()) == 0 {
            return null_mut();
        }
        xstrdup(uri).as_ptr()
    }
}
