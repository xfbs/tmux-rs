// Copyright (c) 2012 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2012 George Nachman <tmux@georgester.com>
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
use std::time::Duration;

use crate::*;

pub struct control_block {
    pub size: usize,
    pub line: *mut u8,
    pub t: u64,
}

pub const CONTROL_PANE_OFF: i32 = 1;
pub const CONTROL_PANE_PAUSED: i32 = 2;

pub struct control_pane {
    pub pane: u32,

    pub offset: window_pane_offset,
    pub queued: window_pane_offset,

    pub flags: i32,

    pub pending_flag: i32,

    pub blocks: Vec<*mut control_block>,
}
pub type control_panes = BTreeMap<u32, Box<control_pane>>;


pub struct control_sub_pane {
    last: *mut u8,
}
pub type control_sub_panes = BTreeMap<(u32, u32), control_sub_pane>;

pub struct control_sub_window {
    last: *mut u8,
}
pub type control_sub_windows = BTreeMap<(u32, u32), control_sub_window>;

pub struct control_sub {
    pub name: *mut u8,
    pub format: *mut u8,
    pub type_: control_sub_type,
    pub id: u32,

    pub last: *mut u8,

    pub panes: control_sub_panes,
    pub windows: control_sub_windows,
}
pub type control_subs = BTreeMap<String, Box<control_sub>>;

pub struct control_state {
    pub panes: control_panes,

    pub pending_list: Vec<*mut control_pane>,

    pub pending_count: u32,

    pub all_blocks: Vec<*mut control_block>,

    /// Buffered data read from the control client fd.
    pub read_input: crate::evbuffer_::Evbuffer,
    /// Calloop read registration for the control client fd.
    pub read_io: Option<IoHandle>,
    /// Buffered data to write to the control client (or `out_fd`).
    pub write_output: crate::evbuffer_::Evbuffer,
    /// Calloop write registration for the control client (or `out_fd`).
    pub write_io: Option<IoHandle>,

    pub subs: control_subs,
    pub subs_timer: Option<TimerHandle>,
}

/// Low and high watermarks.
pub const CONTROL_BUFFER_LOW: i32 = 512;
pub const CONTROL_BUFFER_HIGH: i32 = 8192;

/// Minimum to write to each client.
pub const CONTROL_WRITE_MINIMUM: i32 = 32;

/// Write data to a control client's output buffer and arm the write `IoHandle`.
unsafe fn control_write_bytes(c: *mut client, data: &[u8]) {
    unsafe {
        let cs = (*c).control_state;
        (*cs).write_output.add(data);
        control_arm_write(c);
    }
}

/// Arm the write `IoHandle` if it's not already registered.
unsafe fn control_arm_write(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;
        if (*cs).write_io.is_none() {
            let write_fd = if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
                (*c).fd
            } else {
                (*c).out_fd
            };
            if write_fd >= 0 {
                let cid = (*c).id;
                (*cs).write_io = io_register(
                    write_fd,
                    EV_WRITE,
                    Box::new(move |_fd, _events| control_write_fire(cid)),
                );
            }
        }
    }
}

/// Maximum age for clients that are not using pause mode.
pub const CONTROL_MAXIMUM_AGE: u64 = 300000;

pub const CONTROL_IGNORE_FLAGS: client_flag =
    client_flag::CONTROL_NOOUTPUT.union(CLIENT_UNATTACHEDFLAGS);


pub unsafe fn control_free_sub(cs: *mut control_state, name: &str) {
    unsafe {
        if let Some(mut csub) = (*cs).subs.remove(name) {
            for (_, csp) in std::mem::take(&mut csub.panes) {
                free_(csp.last);
            }
            for (_, csw) in std::mem::take(&mut csub.windows) {
                free_(csw.last);
            }
            free_(csub.last);
            free_(csub.name);
            free_(csub.format);
        }
    }
}

pub unsafe fn control_free_block(cs: *mut control_state, cb: *mut control_block) {
    unsafe {
        free_((*cb).line);
        (*cs).all_blocks.retain(|&p| p != cb);
        free_(cb);
    }
}

pub unsafe fn control_get_pane(c: *mut client, wp: *mut window_pane) -> *mut control_pane {
    unsafe {
        let cs = (*c).control_state;
        if cs.is_null() {
            return null_mut();
        }
        match (*cs).panes.get_mut(&(*wp).id) {
            Some(cp) => &mut **cp as *mut control_pane,
            None => null_mut(),
        }
    }
}

pub unsafe fn control_add_pane(c: *mut client, wp: *mut window_pane) -> NonNull<control_pane> {
    unsafe {
        let cs = (*c).control_state;
        let id = (*wp).id;

        let cp = (*cs).panes.entry(id).or_insert_with(|| {
            Box::new(control_pane {
                pane: id,
                offset: (*wp).offset,
                queued: (*wp).offset,
                flags: 0,
                pending_flag: 0,
                blocks: Vec::new(),
            })
        });

        NonNull::new_unchecked(&mut **cp as *mut control_pane)
    }
}

pub unsafe fn control_discard_pane(c: *mut client, cp: *mut control_pane) {
    unsafe {
        let cs = (*c).control_state;

        for &cb in &(*cp).blocks {
            control_free_block(cs, cb);
        }
        (*cp).blocks.clear();
    }
}

pub unsafe fn control_window_pane(c: *mut client, pane: u32) -> Option<NonNull<window_pane>> {
    unsafe {
        if client_get_session(c).is_null() {
            return None;
        }
        let wp = NonNull::new(window_pane_find_by_id(pane))?;

        winlink_find_by_window(&raw mut (*client_get_session(c)).windows, window_pane_window(wp.as_ptr()))?;

        Some(wp)
    }
}

pub unsafe fn control_reset_offsets(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;

        (*cs).panes.clear();

        (*cs).pending_list.clear();
        (*cs).pending_count = 0;
    }
}

pub unsafe fn control_pane_offset(
    c: *mut client,
    wp: *mut window_pane,
    off: *mut i32,
) -> *mut window_pane_offset {
    unsafe {
        let cs = (*c).control_state;

        if (*c).flags.intersects(client_flag::CONTROL_NOOUTPUT) {
            *off = 0;
            return null_mut();
        }

        let cp = control_get_pane(c, wp);
        if cp.is_null() || ((*cp).flags & CONTROL_PANE_PAUSED != 0) {
            *off = 0;
            return null_mut();
        }
        if (*cp).flags & CONTROL_PANE_OFF != 0 {
            *off = 1;
            return null_mut();
        }
        *off = ((*cs).write_output.len() >= CONTROL_BUFFER_LOW as usize) as i32;
        &raw mut (*cp).offset
    }
}

pub unsafe fn control_set_pane_on(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_get_pane(c, wp);
        if !cp.is_null() && (*cp).flags & CONTROL_PANE_OFF != 0 {
            (*cp).flags &= !CONTROL_PANE_OFF;
            memcpy__(&raw mut (*cp).offset, &raw mut (*wp).offset);
            memcpy__(&raw mut (*cp).queued, &raw mut (*wp).offset);
        }
    }
}

pub unsafe fn control_set_pane_off(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_add_pane(c, wp);
        (*cp.as_ptr()).flags |= CONTROL_PANE_OFF;
    }
}

pub unsafe fn control_continue_pane(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_get_pane(c, wp);
        if !cp.is_null() && ((*cp).flags & CONTROL_PANE_PAUSED) != 0 {
            (*cp).flags &= !CONTROL_PANE_PAUSED;
            memcpy__(&raw mut (*cp).offset, &raw const (*wp).offset);
            memcpy__(&raw mut (*cp).queued, &raw const (*wp).offset);
            control_write!(c, "%continue %{}", (*wp).id);
        }
    }
}

pub unsafe fn control_pause_pane(c: *mut client, wp: *mut window_pane) {
    unsafe {
        let cp = control_add_pane(c, wp).as_ptr();
        if !(*cp).flags & CONTROL_PANE_PAUSED != 0 {
            (*cp).flags |= CONTROL_PANE_PAUSED;
            control_discard_pane(c, cp);
            control_write!(c, "%pause %{}", (*wp).id);
        }
    }
}

pub unsafe fn control_vwrite(c: *mut client, args: std::fmt::Arguments) {
    unsafe {
        let cs = (*c).control_state;
        if cs.is_null() {
            return;
        }

        let mut s = args.to_string();
        s.push('\0');
        let s_ptr: *mut u8 = s.as_mut_ptr().cast();

        log_debug!(
            "{}: {}: writing line: {}",
            "control_vwrite",
            _s((*c).name),
            _s(s_ptr)
        );

        let len = strlen(s_ptr);
        let data = std::slice::from_raw_parts(s_ptr, len);
        control_write_bytes(c, data);
        control_write_bytes(c, b"\n");
        // `s` (the String) is dropped here, freeing the buffer.
        // Do NOT call free_() — the memory is owned by the Rust String.
    }
}

macro_rules! control_write {
   ($c:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::control::control_write_($c, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use control_write;

pub unsafe fn control_write_(c: *mut client, args: std::fmt::Arguments) {
    unsafe {
        let cs = (*c).control_state;
        if cs.is_null() {
            return;
        }

        if (*cs).all_blocks.is_empty() {
            control_vwrite(c, args);
            return;
        }

        let cb = xcalloc_::<control_block>(1).as_ptr();
        let mut value = args.to_string();
        value.push('\0');
        (*cb).line = value.leak().as_mut_ptr().cast();
        (*cs).all_blocks.push(cb);
        (*cb).t = get_timer();

        log_debug!(
            "{}: {}: storing line: {}",
            "control_write",
            _s((*c).name),
            _s((*cb).line)
        );
        control_arm_write(c);
    }
}

pub unsafe fn control_check_age(
    c: *mut client,
    wp: *mut window_pane,
    cp: *mut control_pane,
) -> i32 {
    let __func__ = "control_check_age";
    unsafe {
        let cb = (*cp).blocks.first().copied().unwrap_or(null_mut());
        if cb.is_null() {
            return 0;
        }
        let t = get_timer();
        if (*cb).t >= t {
            return 0;
        }

        let age = t - (*cb).t;
        log_debug!(
            "{}: {}: %%{} is {} behind",
            __func__,
            _s((*c).name),
            (*wp).id,
            age as c_ulonglong,
        );

        if (*c).flags.intersects(client_flag::CONTROL_PAUSEAFTER) {
            if age < (*c).pause_age as u64 {
                return 0;
            }
            (*cp).flags |= CONTROL_PANE_PAUSED;
            control_discard_pane(c, cp);
            control_write!(c, "%pause %{}", (*wp).id);
        } else {
            if age < CONTROL_MAXIMUM_AGE {
                return 0;
            }
            (*c).exit_message = Some("too far behind".to_string());
            (*c).flags |= client_flag::EXIT;
            control_discard(c);
        }
    }

    1
}

pub unsafe fn control_write_output(c: *mut client, wp: *mut window_pane) {
    let __func__ = "control_write_output";
    unsafe {
        let cs = (*c).control_state;
        let cp: *mut control_pane;
        let mut new_size = 0usize;

        'ignore: {
            if winlink_find_by_window(&raw mut (*client_get_session(c)).windows, window_pane_window(wp)).is_none() {
                return;
            }

            if (*c).flags.intersects(CONTROL_IGNORE_FLAGS) {
                cp = control_get_pane(c, wp);
                if !cp.is_null() {
                    break 'ignore;
                }
                return;
            }
            cp = control_add_pane(c, wp).as_ptr();
            if (*cp).flags & (CONTROL_PANE_OFF | CONTROL_PANE_PAUSED) != 0 {
                break 'ignore;
            }
            if control_check_age(c, wp, cp) != 0 {
                return;
            }

            window_pane_get_new_data(wp, &raw mut (*cp).queued, &raw mut new_size);
            if new_size == 0 {
                return;
            }
            window_pane_update_used_data(wp, &raw mut (*cp).queued, new_size);

            let cb = xcalloc_::<control_block>(1).as_ptr();
            (*cb).size = new_size;
            (*cs).all_blocks.push(cb);
            (*cb).t = get_timer();

            (*cp).blocks.push(cb);
            log_debug!(
                "{}: {}: new output block of {} for %%{}",
                __func__,
                _s((*c).name),
                (*cb).size,
                (*wp).id,
            );

            if (*cp).pending_flag == 0 {
                log_debug!(
                    "{}: {}: %%{} now pending",
                    __func__,
                    _s((*c).name),
                    (*wp).id
                );
                (*cs).pending_list.push(cp);
                (*cp).pending_flag = 1;
                (*cs).pending_count += 1;
            }
            control_arm_write(c);
            return;
        }
        // ignore:
        log_debug!(
            "{}: {}: ignoring pane %%{}",
            __func__,
            _s((*c).name),
            (*wp).id
        );
        window_pane_update_used_data(wp, &raw mut (*cp).offset, usize::MAX);
        window_pane_update_used_data(wp, &raw mut (*cp).queued, usize::MAX);
    }
}

pub unsafe fn control_error(item: *mut cmdq_item, data: *mut c_void) -> cmd_retval {
    unsafe {
        let c = cmdq_get_client(item);
        let error = data as *mut u8;

        cmdq_guard(item, c!("begin"), true);
        control_write!(c, "parse error: {}", _s(error));
        cmdq_guard(item, c!("error"), true);

        free_(error);
    }
    cmd_retval::CMD_RETURN_NORMAL
}


/// Read callback: reads data from the control client fd, parses lines as commands.
unsafe fn control_read_fire(cid: ClientId) {
    let __func__ = "control_read_callback";

    unsafe {
        let Some(c) = client_from_id(cid) else { return };
        let cs = (*c).control_state;

        let n = (*cs).read_input.read_from_fd((*c).fd, 4096);
        if n <= 0 {
            if n < 0
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock
            {
                return;
            }
            // EOF or error.
            (*c).flags |= client_flag::EXIT;
            return;
        }

        let buffer = &raw mut (*cs).read_input;
        let mut error = null_mut();

        loop {
            let line = evbuffer_readln(buffer, null_mut(), evbuffer_eol_style::EVBUFFER_EOL_LF);
            if line.is_null() {
                break;
            }
            log_debug!("{}: {}: {}", __func__, _s((*c).name), _s(line));
            if *line == b'\0' {
                free_(line);
                (*c).flags |= client_flag::EXIT;
                break;
            }

            let state =
                cmdq_new_state(null_mut(), null_mut(), cmdq_state_flags::CMDQ_STATE_CONTROL);
            let status = cmd_parse_and_append(cstr_to_str(line), None, c, state, &raw mut error);
            if status == cmd_parse_status::CMD_PARSE_ERROR {
                cmdq_append(c, cmdq_get_callback!(control_error, error).as_ptr());
            }
            cmdq_free_state(state);

            free_(line);
        }
    }
}

pub unsafe fn control_all_done(c: *mut client) -> i32 {
    unsafe {
        let cs = (*c).control_state;

        if !(*cs).all_blocks.is_empty() {
            return 0;
        }
        (*cs).write_output.is_empty() as i32
    }
}

pub unsafe fn control_flush_all_blocks(c: *mut client) {
    let __func__ = "control_flush_all_blocks";
    unsafe {
        let cs = (*c).control_state;

        while let Some(&cb) = (*cs).all_blocks.first() {
            if (*cb).size != 0 {
                break;
            }
            log_debug!(
                "{}: {}: flushing line: {}",
                __func__,
                _s((*c).name),
                _s((*cb).line)
            );

            let line_data = std::slice::from_raw_parts((*cb).line, strlen((*cb).line));
            (*cs).write_output.add(line_data);
            (*cs).write_output.add(b"\n");
            free_((*cb).line);
            (*cs).all_blocks.remove(0);
            free_(cb);
        }
    }
}

pub unsafe fn control_append_data(
    c: *mut client,
    cp: *mut control_pane,
    age: u64,
    mut message: *mut evbuffer,
    wp: *mut window_pane,
    size: usize,
) -> *mut evbuffer {
    unsafe {
        if message.is_null() {
            message = evbuffer_new();
            if message.is_null() {
                fatalx("out of memory");
            }
            if (*c).flags.intersects(client_flag::CONTROL_PAUSEAFTER) {
                evbuffer_add_printf!(message, "%extended-output %{} {} : ", (*wp).id, age);
            } else {
                evbuffer_add_printf!(message, "%output %{} ", (*wp).id);
            }
        }

        let mut new_size = 0usize;
        let new_data: *mut c_uchar =
            window_pane_get_new_data(wp, &raw mut (*cp).offset, &raw mut new_size).cast();
        if new_size < size {
            fatalx_!("not enough data: {} < {}", new_size, size);
        }
        for i in 0..size {
            if *new_data.add(i) < b' ' || *new_data.add(i) == b'\\' {
                evbuffer_add_printf!(message, "\\{:03o}", *new_data.add(i) as i32);
            } else {
                evbuffer_add_printf!(message, "{}", *new_data.add(i) as char);
            }
        }
        window_pane_update_used_data(wp, &raw mut (*cp).offset, size);
        message
    }
}

pub unsafe fn control_write_data(c: *mut client, message: *mut evbuffer) {
    unsafe {
        let cs = (*c).control_state;

        log_debug!(
            "control_write_data: {0}: {2:1$}",
            _s((*c).name),
            EVBUFFER_LENGTH(message),
            _s(EVBUFFER_DATA(message).cast::<u8>()),
        );

        evbuffer_add(message, c!("\n").cast(), 1);
        let data = std::slice::from_raw_parts(EVBUFFER_DATA(message), EVBUFFER_LENGTH(message));
        (*cs).write_output.add(data);
        control_arm_write(c);
        evbuffer_free(message);
    }
}

pub unsafe fn control_write_pending(c: *mut client, cp: *mut control_pane, limit: usize) -> i32 {
    unsafe {
        let cs = (*c).control_state;
        let mut message: *mut evbuffer = null_mut();
        let mut used = 0;
        let mut size;
        let mut cb;
        let t = get_timer();

        let wp = control_window_pane(c, (*cp).pane);
        if wp.is_none() || (*wp.unwrap().as_ptr()).fd == -1 {
            for &cb in &(*cp).blocks {
                control_free_block(cs, cb);
            }
            (*cp).blocks.clear();
            control_flush_all_blocks(c);
            return 0;
        }

        while used != limit && !(*cp).blocks.is_empty() {
            if control_check_age(c, transmute_ptr(wp), cp) != 0 {
                if !message.is_null() {
                    evbuffer_free(message);
                }
                message = null_mut();
                break;
            }

            cb = (&(*cp).blocks)[0];
            let age = t.saturating_sub((*cb).t);
            log_debug!(
                "{}: {}: output block {} (age {}) for %%{} (used {}/{})",
                "control_write_pending",
                _s((*c).name),
                (*cb).size,
                age,
                (*cp).pane,
                used,
                limit,
            );

            size = (*cb).size;
            if size > limit - used {
                size = limit - used;
            }
            used += size;

            message = control_append_data(c, cp, age, message, transmute_ptr(wp), size);

            (*cb).size -= size;
            if (*cb).size == 0 {
                (*cp).blocks.remove(0);
                control_free_block(cs, cb);

                let first_all = (*cs).all_blocks.first().copied().unwrap_or(null_mut());
                if !first_all.is_null() && (*first_all).size == 0 {
                    if wp.is_some() && !message.is_null() {
                        control_write_data(c, message);
                        message = null_mut();
                    }
                    control_flush_all_blocks(c);
                }
            }
        }
        if !message.is_null() {
            control_write_data(c, message);
        }
        !(*cp).blocks.is_empty() as i32
    }
}

/// Write callback: drains `write_output` to the control client fd,
/// then flushes pending pane output blocks if there's room.
unsafe fn control_write_fire(cid: ClientId) {
    unsafe {
        let Some(c) = client_from_id(cid) else { return };
        let cs = (*c).control_state;

        // Drain buffered data to the fd.
        if !(*cs).write_output.is_empty() {
            let write_fd = if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
                (*c).fd
            } else {
                (*c).out_fd
            };
            let n = (*cs).write_output.write_to_fd(write_fd);
            if n < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
                    return;
                }
                // Write error — exit.
                (*c).flags |= client_flag::EXIT;
                return;
            }
        }

        control_flush_all_blocks(c);

        while (*cs).write_output.len() < CONTROL_BUFFER_HIGH as usize {
            if (*cs).pending_count == 0 {
                break;
            }
            let space = CONTROL_BUFFER_HIGH as usize - (*cs).write_output.len();
            log_debug!(
                "{}: {}: {} bytes available, {} panes",
                "control_write_callback",
                _s((*c).name),
                space,
                (*cs).pending_count,
            );

            let mut limit: usize = space / (*cs).pending_count as usize / 3;
            if limit < CONTROL_WRITE_MINIMUM as usize {
                limit = CONTROL_WRITE_MINIMUM as usize;
            }

            let pending = &mut (*cs).pending_list;
            let mut i = 0;
            while i < pending.len() {
                if (*cs).write_output.len() >= CONTROL_BUFFER_HIGH as usize {
                    break;
                }
                let cp = pending[i];
                if control_write_pending(c, cp, limit) != 0 {
                    i += 1;
                    continue;
                }
                pending.remove(i);
                (*cp).pending_flag = 0;
                (*cs).pending_count -= 1;
            }
        }
        if (*cs).write_output.is_empty() {
            (*cs).write_io = None; // deregisters from calloop
        }
    }
}

pub unsafe fn control_start(c: *mut client) {
    unsafe {
        if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
            libc::close((*c).out_fd);
            (*c).out_fd = -1;
        } else {
            setblocking((*c).out_fd, 0);
        }
        setblocking((*c).fd, 0);

        (*c).control_state = xcalloc_::<control_state>(1).as_ptr();
        let cs = (*c).control_state;
        std::ptr::write(&raw mut (*cs).panes, BTreeMap::new());
        std::ptr::write(&raw mut (*cs).pending_list, Vec::new());
        std::ptr::write(&raw mut (*cs).all_blocks, Vec::new());
        std::ptr::write(&raw mut (*cs).subs, BTreeMap::new());
        std::ptr::write(&raw mut (*cs).subs_timer, None);
        std::ptr::write(&raw mut (*cs).read_input, crate::evbuffer_::Evbuffer::new());
        std::ptr::write(&raw mut (*cs).read_io, None);
        std::ptr::write(&raw mut (*cs).write_output, crate::evbuffer_::Evbuffer::new());
        std::ptr::write(&raw mut (*cs).write_io, None);

        // Read IoHandle is armed later in control_ready(), after the client
        // is fully set up and config is loaded.
        // Write IoHandle is armed on demand (control_arm_write).

        if (*c).flags.intersects(client_flag::CONTROLCONTROL) {
            control_write_bytes(c, b"\x1bP1000p");
        }
    }
}

pub unsafe fn control_ready(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;
        // Re-arm the read IoHandle if it was dropped by control_discard.
        if (*cs).read_io.is_none() {
            let cid = (*c).id;
            (*cs).read_io = io_register(
                (*c).fd,
                EV_READ,
                Box::new(move |_fd, _events| control_read_fire(cid)),
            );
        }
    }
}

pub unsafe fn control_discard(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;
        for cp in (*cs).panes.values_mut() {
            control_discard_pane(c, &mut **cp as *mut control_pane);
        }
        // Drop the read IoHandle to stop reading.
        (*cs).read_io = None;
    }
}

pub unsafe fn control_stop(c: *mut client) {
    unsafe {
        let cs = (*c).control_state;
        // Drop IoHandles (deregisters from calloop).
        (*cs).read_io = None;
        (*cs).write_io = None;

        let sub_keys: Vec<String> = (*cs).subs.keys().cloned().collect();
        for key in sub_keys {
            control_free_sub(cs, &key);
        }
        (*cs).subs_timer = None;

        for &cb in &(*cs).all_blocks {
            free_((*cb).line);
            free_(cb);
        }
        (*cs).all_blocks.clear();
        control_reset_offsets(c);

        std::ptr::drop_in_place(&raw mut (*cs).read_input);
        std::ptr::drop_in_place(&raw mut (*cs).read_io);
        std::ptr::drop_in_place(&raw mut (*cs).write_output);
        std::ptr::drop_in_place(&raw mut (*cs).write_io);
        std::ptr::drop_in_place(&raw mut (*cs).all_blocks);
        std::ptr::drop_in_place(&raw mut (*cs).panes);
        std::ptr::drop_in_place(&raw mut (*cs).pending_list);
        std::ptr::drop_in_place(&raw mut (*cs).subs);
        free_(cs);
        (*c).control_state = null_mut();
    }
}

pub unsafe fn control_check_subs_session(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = client_get_session(c);

        let ft = format_create_defaults(null_mut(), c, s, null_mut(), null_mut());
        let value = format_expand(ft, (*csub).format);
        format_free(ft);

        if !(*csub).last.is_null() && libc::strcmp(value, (*csub).last) == 0 {
            free_(value);
            return;
        }
        control_write!(
            c,
            "%subscription-changed {} ${} - - - : {}",
            _s((*csub).name),
            (*s).id,
            _s(value),
        );
        free_((*csub).last);
        (*csub).last = value;
    }
}

pub unsafe fn control_check_subs_pane(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = client_get_session(c);

        let wp = window_pane_find_by_id((*csub).id);
        if wp.is_null() || (*wp).fd == -1 {
            return;
        }
        let w = window_pane_window(wp);

        for &wl in &(*w).winlinks {
            if (*wl).session != (if s.is_null() { None } else { Some(SessionId((*s).id)) }) {
                continue;
            }

            let ft = format_create_defaults(null_mut(), c, s, wl, wp);
            let value = format_expand(ft, (*csub).format);
            format_free(ft);

            let key = ((*wp).id, (*wl).idx as u32);
            let csp = (*csub).panes.entry(key).or_insert_with(|| control_sub_pane { last: null_mut(),
            });

            if !csp.last.is_null() && libc::strcmp(value, csp.last) == 0 {
                free_(value);
                continue;
            }
            control_write!(
                c,
                "%subscription-changed {} ${} @{} {} %{} : {}",
                _s((*csub).name),
                (*s).id,
                (*w).id,
                (*wl).idx,
                (*wp).id,
                _s(value),
            );
            free_(csp.last);
            csp.last = value;
        }
    }
}

pub unsafe fn control_check_subs_all_panes(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = client_get_session(c);

        for &wl in (*(&raw mut (*s).windows)).values() {
            let w = winlink_window(wl);
            for &wp in &(*w).panes {
                let ft = format_create_defaults(null_mut(), c, s, wl, wp);
                let value = format_expand(ft, (*csub).format);
                format_free(ft);

                let key = ((*wp).id, (*wl).idx as u32);
                let csp = (*csub).panes.entry(key).or_insert_with(|| control_sub_pane { last: null_mut(),
                });

                if !csp.last.is_null() && libc::strcmp(value, csp.last) == 0 {
                    free_(value);
                    continue;
                }
                control_write!(
                    c,
                    "%subscription-changed {} ${} @{} {} %{} : {}",
                    _s((*csub).name),
                    (*s).id,
                    (*w).id,
                    (*wl).idx,
                    (*wp).id,
                    _s(value),
                );
                free_(csp.last);
                csp.last = value;
            }
        }
    }
}

pub unsafe fn control_check_subs_window(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = client_get_session(c);

        let w = window_find_by_id((*csub).id);
        if w.is_null() {
            return;
        }

        for &wl in &(*w).winlinks {
            if (*wl).session != (if s.is_null() { None } else { Some(SessionId((*s).id)) }) {
                continue;
            }

            let ft = format_create_defaults(null_mut(), c, s, wl, null_mut());
            let value = format_expand(ft, (*csub).format);
            format_free(ft);

            let key = ((*w).id, (*wl).idx as u32);
            let csw = (*csub).windows.entry(key).or_insert_with(|| control_sub_window { last: null_mut(),
            });

            if !csw.last.is_null() && libc::strcmp(value, csw.last) == 0 {
                free_(value);
                continue;
            }
            control_write!(
                c,
                "%subscription-changed {} ${} @{} {} - : {}",
                _s((*csub).name),
                (*s).id,
                (*w).id,
                (*wl).idx,
                _s(value),
            );
            free_(csw.last);
            csw.last = value;
        }
    }
}

pub unsafe fn control_check_subs_all_windows(c: *mut client, csub: *mut control_sub) {
    unsafe {
        let s = client_get_session(c);

        for &wl in (*(&raw mut (*s).windows)).values() {
            let w = winlink_window(wl);

            let ft = format_create_defaults(null_mut(), c, s, wl, null_mut());
            let value = format_expand(ft, (*csub).format);
            format_free(ft);

            let key = ((*w).id, (*wl).idx as u32);
            let csw = (*csub).windows.entry(key).or_insert_with(|| control_sub_window { last: null_mut(),
            });

            if !csw.last.is_null() && libc::strcmp(value, csw.last) == 0 {
                free_(value);
                continue;
            }
            control_write!(
                c,
                "%subscription-changed {} ${} @{} {} - : {}",
                _s((*csub).name),
                (*s).id,
                (*w).id,
                (*wl).idx,
                _s(value),
            );
            free_(csw.last);
            csw.last = value;
        }
    }
}

/// Control subscription timer callback: check subs and re-arm.
unsafe fn control_check_subs_timer_fire(cid: ClientId) {
    unsafe {
        let Some(c) = client_from_id(cid) else { return };
        let cs = (*c).control_state;

        log_debug!("{}: timer fired", "control_check_subs_timer");

        // Re-arm for next check.
        (*cs).subs_timer = timer_add(
            Duration::from_secs(1),
            Box::new(move || control_check_subs_timer_fire(cid)),
        );

        for csub in (*cs).subs.values_mut() {
            let csub: *mut control_sub = &mut **csub;
            match (*csub).type_ {
                control_sub_type::CONTROL_SUB_SESSION => control_check_subs_session(c, csub),
                control_sub_type::CONTROL_SUB_PANE => control_check_subs_pane(c, csub),
                control_sub_type::CONTROL_SUB_ALL_PANES => control_check_subs_all_panes(c, csub),
                control_sub_type::CONTROL_SUB_WINDOW => control_check_subs_window(c, csub),
                control_sub_type::CONTROL_SUB_ALL_WINDOWS => {
                    control_check_subs_all_windows(c, csub);
                }
            }
        }
    }
}

pub unsafe fn control_add_sub(
    c: *mut client,
    name: *mut u8,
    type_: control_sub_type,
    id: i32,
    format: *const u8,
) {
    unsafe {
        let cs = (*c).control_state;
        let _tv = timeval {
            tv_sec: 1,
            tv_usec: 0,
        };

        let key = cstr_to_str(name).to_string();
        control_free_sub(cs, &key);

        let csub = Box::new(control_sub {
            name: xstrdup(name).as_ptr(),
            format: xstrdup(format).as_ptr(),
            type_,
            id: id as u32,
            last: null_mut(),
            panes: BTreeMap::new(),
            windows: BTreeMap::new(),
        });
        (*cs).subs.insert(key, csub);

        if (*cs).subs_timer.is_none() {
            let cid = (*c).id;
            (*cs).subs_timer = timer_add(
                Duration::from_secs(1),
                Box::new(move || control_check_subs_timer_fire(cid)),
            );
        }
    }
}

pub unsafe fn control_remove_sub(c: *mut client, name: *mut u8) {
    unsafe {
        let cs = (*c).control_state;

        let key = cstr_to_str(name);
        control_free_sub(cs, key);
        if (*cs).subs.is_empty() {
            (*cs).subs_timer = None;
        }
    }
}
