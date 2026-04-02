// Copyright (c) 2013 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2013 Thiago de Arruda <tpadilha84@gmail.com>
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

pub static CMD_WAIT_FOR_ENTRY: cmd_entry = cmd_entry {
    name: "wait-for",
    alias: Some("wait"),

    args: args_parse::new("LSU", 1, 1, None),
    usage: "[-L|-S|-U] channel",

    flags: cmd_flag::empty(),
    exec: cmd_wait_for_exec,
    source: cmd_entry_flag::zeroed(),
    target: cmd_entry_flag::zeroed(),
};

pub struct wait_channel {
    pub name: *mut u8,
    pub locked: bool,
    pub woken: bool,

    pub waiters: Vec<*mut cmdq_item>,
    pub lockers: Vec<*mut cmdq_item>,
}

static mut WAIT_CHANNELS: BTreeMap<String, Box<wait_channel>> = BTreeMap::new();

unsafe fn wait_channel_find(name: *const u8) -> *mut wait_channel {
    unsafe {
        let key = cstr_to_str(name);
        (*(&raw mut WAIT_CHANNELS))
            .get_mut(key)
            .map_or(null_mut(), |wc| &mut **wc as *mut wait_channel)
    }
}

pub unsafe fn cmd_wait_for_add(name: *const u8) -> *mut wait_channel {
    unsafe {
        let key = cstr_to_str(name).to_string();
        let wc = Box::new(wait_channel {
            name: xstrdup(name).as_ptr(),
            locked: false,
            woken: false,
            waiters: Vec::new(),
            lockers: Vec::new(),
        });

        log_debug!("add wait channel {}", _s(wc.name));

        (*(&raw mut WAIT_CHANNELS)).insert(key.clone(), wc);
        &mut **(*(&raw mut WAIT_CHANNELS)).get_mut(&key).unwrap() as *mut wait_channel
    }
}

pub unsafe fn cmd_wait_for_remove(wc: *mut wait_channel) {
    unsafe {
        if (*wc).locked {
            return;
        }
        if !(*wc).waiters.is_empty() || !(*wc).woken {
            return;
        }

        let key = cstr_to_str((*wc).name).to_string();
        log_debug!("remove wait channel {}", _s((*wc).name));

        if let Some(removed) = (*(&raw mut WAIT_CHANNELS)).remove(&key) {
            free_(removed.name);
        }
    }
}

pub unsafe fn cmd_wait_for_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let name = args_string(args, 0);

        let wc = wait_channel_find(name);

        if args_has(args, 'S') {
            return cmd_wait_for_signal(item, name, wc);
        }
        if args_has(args, 'L') {
            return cmd_wait_for_lock(item, name, wc);
        }
        if args_has(args, 'U') {
            return cmd_wait_for_unlock(item, name, wc);
        }

        cmd_wait_for_wait(item, name, wc)
    }
}

pub unsafe fn cmd_wait_for_signal(
    _item: *const cmdq_item,
    name: *const u8,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if (*wc).waiters.is_empty() && !(*wc).woken {
            log_debug!("signal wait channel {}, no waiters", _s((*wc).name));
            (*wc).woken = true;
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        log_debug!("signal wait channel {}, with waiters", _s((*wc).name));

        for &wi in &(*wc).waiters {
            cmdq_continue(wi);
        }
        (*wc).waiters.clear();

        cmd_wait_for_remove(wc);

        cmd_retval::CMD_RETURN_NORMAL
    }
}

pub unsafe fn cmd_wait_for_wait(
    item: *mut cmdq_item,
    name: *const u8,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        let c = cmdq_get_client(item);

        if c.is_null() {
            cmdq_error!(item, "not able to wait");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if (*wc).woken {
            log_debug!("wait channel {} already woken ({:p})", _s((*wc).name), c);
            cmd_wait_for_remove(wc);
            return cmd_retval::CMD_RETURN_NORMAL;
        }
        log_debug!("wait channel {} not woken ({:p})", _s((*wc).name), c);

        (*wc).waiters.push(item);
    }
    cmd_retval::CMD_RETURN_WAIT
}

pub unsafe fn cmd_wait_for_lock(
    item: *mut cmdq_item,
    name: *const u8,
    mut wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if cmdq_get_client(item).is_null() {
            cmdq_error!(item, "not able to lock");
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if wc.is_null() {
            wc = cmd_wait_for_add(name);
        }

        if (*wc).locked {
            (*wc).lockers.push(item);
            return cmd_retval::CMD_RETURN_WAIT;
        }
        (*wc).locked = true;
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_wait_for_unlock(
    item: *mut cmdq_item,
    name: *const u8,
    wc: *mut wait_channel,
) -> cmd_retval {
    unsafe {
        if wc.is_null() || !(*wc).locked {
            cmdq_error!(item, "channel {} not locked", _s(name));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if !(*wc).lockers.is_empty() {
            let wi = (*wc).lockers.remove(0);
            cmdq_continue(wi);
        } else {
            (*wc).locked = false;
            cmd_wait_for_remove(wc);
        }
    }
    cmd_retval::CMD_RETURN_NORMAL
}

pub unsafe fn cmd_wait_for_flush() {
    unsafe {
        let keys: Vec<String> = (*(&raw mut WAIT_CHANNELS)).keys().cloned().collect();
        for key in keys {
            let Some(wc) = (*(&raw mut WAIT_CHANNELS)).get_mut(&key) else {
                continue;
            };
            let wc = &mut **wc as *mut wait_channel;
            for &wi in &(*wc).waiters {
                cmdq_continue(wi);
            }
            (*wc).waiters.clear();
            (*wc).woken = true;
            for &wi in &(*wc).lockers {
                cmdq_continue(wi);
            }
            (*wc).lockers.clear();
            (*wc).locked = false;
            cmd_wait_for_remove(wc);
        }
    }
}
