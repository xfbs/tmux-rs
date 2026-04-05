// Copyright (c) 2021 Holland Schutte, Jayson Morberg
// Copyright (c) 2021 Dallas Lyons <dallasdlyons@gmail.com>
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

use crate::libc::{getpwuid, getuid};
use crate::*;

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Eq, PartialEq)]
    pub struct server_acl_user_flags: i32 {
        const SERVER_ACL_READONLY = 0x1;
    }
}

pub struct server_acl_user {
    pub uid: uid_t,
    pub flags: server_acl_user_flags,
}

static mut SERVER_ACL_ENTRIES: BTreeMap<uid_t, server_acl_user> = BTreeMap::new();

pub unsafe fn server_acl_init() {
    unsafe {
        if getuid() != 0 {
            server_acl_user_allow(0);
        }
        server_acl_user_allow(getuid());
    }
}

pub unsafe fn server_acl_user_find(uid: uid_t) -> *mut server_acl_user {
    unsafe {
        (*(&raw mut SERVER_ACL_ENTRIES))
            .get_mut(&uid)
            .map_or(null_mut(), |u| u as *mut server_acl_user)
    }
}

pub unsafe fn server_acl_display(item: *mut cmdq_item) {
    unsafe {
        for user in (*(&raw mut SERVER_ACL_ENTRIES)).values() {
            if user.uid == 0 {
                continue;
            }
            let pw = getpwuid(user.uid);
            let name: *const u8 = if !pw.is_null() {
                (*pw).pw_name.cast()
            } else {
                c!("unknown")
            };
            if user.flags == server_acl_user_flags::SERVER_ACL_READONLY {
                cmdq_print!(item, "{} (R)", _s(name));
            } else {
                cmdq_print!(item, "{} (W)", _s(name));
            }
        }
    }
}

pub unsafe fn server_acl_user_allow(uid: uid_t) {
    unsafe {
        (*(&raw mut SERVER_ACL_ENTRIES)).entry(uid).or_insert(server_acl_user {
            uid,
            flags: server_acl_user_flags::empty(),
        });
    }
}

pub unsafe fn server_acl_user_deny(uid: uid_t) {
    unsafe {
        (*(&raw mut SERVER_ACL_ENTRIES)).remove(&uid);
    }
}

pub unsafe fn server_acl_user_allow_write(mut uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if user.is_null() {
            return;
        }
        (*user).flags &= !server_acl_user_flags::SERVER_ACL_READONLY;

        for c in clients_iter() {
            uid = proc_get_peer_uid((*c).peer);
            if uid != -1i32 as uid_t && uid == (*user).uid {
                (*c).flags &= !client_flag::READONLY;
            }
        }
    }
}

pub unsafe fn server_acl_user_deny_write(mut uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if user.is_null() {
            return;
        }
        (*user).flags |= server_acl_user_flags::SERVER_ACL_READONLY;

        for c in clients_iter() {
            uid = proc_get_peer_uid((*c).peer);
            if uid != -1i32 as uid_t && uid == (*user).uid {
                (*c).flags &= !client_flag::READONLY;
            }
        }
    }
}

pub unsafe fn server_acl_join(c: *mut client) -> c_int {
    unsafe {
        let uid = proc_get_peer_uid((*c).peer);
        if uid == -1i32 as uid_t {
            return 0;
        }

        let user = server_acl_user_find(uid);
        if user.is_null() {
            return 0;
        }
        if (*user)
            .flags
            .contains(server_acl_user_flags::SERVER_ACL_READONLY)
        {
            (*c).flags |= client_flag::READONLY;
        }
        1
    }
}

pub unsafe fn server_acl_get_uid(user: *mut server_acl_user) -> uid_t {
    unsafe { (*user).uid }
}
