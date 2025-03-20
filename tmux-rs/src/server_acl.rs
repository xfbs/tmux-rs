use compat_rs::{
    queue::tailq_foreach,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_remove},
};
use libc::{getpwuid, getuid};

use crate::{xmalloc::Zeroable, *};

unsafe extern "C" {
    // pub unsafe fn server_acl_init();
    // pub unsafe fn server_acl_user_find(_: uid_t) -> *mut server_acl_user;
    // pub unsafe fn server_acl_display(_: *mut cmdq_item);
    // pub unsafe fn server_acl_user_allow(_: uid_t);
    // pub unsafe fn server_acl_user_deny(_: uid_t);
    // pub unsafe fn server_acl_user_allow_write(_: uid_t);
    // pub unsafe fn server_acl_user_deny_write(_: uid_t);
    // pub unsafe fn server_acl_join(_: *mut client) -> c_int;
    // pub unsafe fn server_acl_get_uid(_: *mut server_acl_user) -> uid_t;
}

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Eq, PartialEq)]
    struct server_acl_user_flags: i32 {
        const SERVER_ACL_READONLY = 0x1;
    }
}

unsafe impl Zeroable for server_acl_user {}
pub struct server_acl_user {
    pub uid: uid_t,

    pub flags: server_acl_user_flags,

    pub entry: rb_entry<server_acl_user>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_cmp(user1: *const server_acl_user, user2: *const server_acl_user) -> i32 {
    unsafe {
        if ((*user1).uid < (*user2).uid) {
            return -1;
        }
        ((*user1).uid > (*user2).uid) as i32
    }
}

pub type server_acl_entries = rb_head<server_acl_user>;
static mut server_acl_entries: server_acl_entries = unsafe { zeroed() };

RB_GENERATE!(server_acl_entries, server_acl_user, entry, server_acl_cmp);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_init() {
    unsafe {
        rb_init(&raw mut server_acl_entries);

        if (getuid() != 0) {
            server_acl_user_allow(0);
        }
        server_acl_user_allow(getuid());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_user_find(uid: uid_t) -> *mut server_acl_user {
    unsafe {
        let mut find: server_acl_user = server_acl_user { uid, ..zeroed() };

        rb_find::<_, _>(&raw mut server_acl_entries, &raw mut find)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_display(item: *mut cmdq_item) {
    unsafe {
        // struct server_acl_user *loop_;
        // struct passwd *pw;
        // const char *name;

        // server_acl_entries
        for loop_ in rb_foreach(&raw mut server_acl_entries).map(NonNull::as_ptr) {
            if ((*loop_).uid == 0) {
                continue;
            }
            let pw = getpwuid((*loop_).uid);
            let name = if (!pw.is_null()) {
                (*pw).pw_name
            } else {
                c"unknown".as_ptr()
            };
            if ((*loop_).flags == server_acl_user_flags::SERVER_ACL_READONLY) {
                cmdq_print(item, c"%s (R)".as_ptr(), name);
            } else {
                cmdq_print(item, c"%s (W)".as_ptr(), name);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_user_allow(uid: uid_t) {
    unsafe {
        let mut user = server_acl_user_find(uid);
        if (user.is_null()) {
            user = xcalloc1();
            (*user).uid = uid;
            // server_acl_entries
            rb_insert(&raw mut server_acl_entries, user);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_user_deny(uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if (!user.is_null()) {
            // server_acl_entries
            rb_remove(&raw mut server_acl_entries, user);
            free_(user);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_user_allow_write(mut uid: uid_t) {
    unsafe {
        let user = server_acl_user_find(uid);
        if (user.is_null()) {
            return;
        }
        (*user).flags &= !server_acl_user_flags::SERVER_ACL_READONLY;

        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            uid = proc_get_peer_uid((*c).peer);
            if (uid != -1i32 as uid_t && uid == (*user).uid) {
                (*c).flags &= !client_flag::READONLY;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_user_deny_write(mut uid: uid_t) {
    unsafe {
        unsafe {
            let user = server_acl_user_find(uid);
            if (user.is_null()) {
                return;
            }
            (*user).flags |= server_acl_user_flags::SERVER_ACL_READONLY;

            for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                uid = proc_get_peer_uid((*c).peer);
                if (uid != -1i32 as uid_t && uid == (*user).uid) {
                    (*c).flags &= !client_flag::READONLY;
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_join(c: *mut client) -> c_int {
    unsafe {
        let uid = proc_get_peer_uid((*c).peer);
        if (uid == -1i32 as uid_t) {
            return 0;
        }

        let user = server_acl_user_find(uid);
        if (user.is_null()) {
            return 0;
        }
        if (*user).flags.contains(server_acl_user_flags::SERVER_ACL_READONLY) {
            (*c).flags |= client_flag::READONLY;
        }
        return 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_acl_get_uid(user: *mut server_acl_user) -> uid_t { unsafe { (*user).uid } }
