use compat_rs::RB_GENERATE;

use crate::*;

unsafe extern "C" {
    pub unsafe static mut sessions: sessions;
    pub unsafe static mut next_session_id: c_uint;
    pub unsafe fn session_cmp(_: *const session, _: *const session) -> c_int;
    pub unsafe fn sessions_RB_INSERT_COLOR(_: *mut sessions, _: *mut session);
    pub unsafe fn sessions_RB_REMOVE_COLOR(_: *mut sessions, _: *mut session, _: *mut session);
    pub unsafe fn sessions_RB_REMOVE(_: *mut sessions, _: *mut session) -> *mut session;
    pub unsafe fn sessions_RB_INSERT(_: *mut sessions, _: *mut session) -> *mut session;
    pub unsafe fn sessions_RB_FIND(_: *mut sessions, _: *mut session) -> *mut session;
    pub unsafe fn sessions_RB_NFIND(_: *mut sessions, _: *mut session) -> *mut session;
    pub unsafe fn session_alive(_: *mut session) -> c_int;
    pub unsafe fn session_find(_: *const c_char) -> *mut session;
    pub unsafe fn session_find_by_id_str(_: *const c_char) -> *mut session;
    pub unsafe fn session_find_by_id(_: c_uint) -> *mut session;
    pub unsafe fn session_create(
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut environ,
        _: *mut options,
        _: *mut termios,
    ) -> *mut session;
    pub unsafe fn session_destroy(_: *mut session, _: c_int, _: *const c_char);
    pub unsafe fn session_add_ref(_: *mut session, _: *const c_char);
    pub unsafe fn session_remove_ref(_: *mut session, _: *const c_char);
    pub unsafe fn session_check_name(_: *const c_char) -> *mut c_char;
    pub unsafe fn session_update_activity(_: *mut session, _: *mut timeval);
    pub unsafe fn session_next_session(_: *mut session) -> *mut session;
    pub unsafe fn session_previous_session(_: *mut session) -> *mut session;
    pub unsafe fn session_attach(_: *mut session, _: *mut window, _: c_int, _: *mut *mut c_char) -> *mut winlink;
    pub unsafe fn session_detach(_: *mut session, _: *mut winlink) -> c_int;
    pub unsafe fn session_has(_: *mut session, _: *mut window) -> c_int;
    pub unsafe fn session_is_linked(_: *mut session, _: *mut window) -> c_int;
    pub unsafe fn session_next(_: *mut session, _: c_int) -> c_int;
    pub unsafe fn session_previous(_: *mut session, _: c_int) -> c_int;
    pub unsafe fn session_select(_: *mut session, _: c_int) -> c_int;
    pub unsafe fn session_last(_: *mut session) -> c_int;
    pub unsafe fn session_set_current(_: *mut session, _: *mut winlink) -> c_int;
    pub unsafe fn session_group_cmp(_: *const session_group, _: *const session_group) -> c_int;
    pub unsafe fn session_group_contains(_: *mut session) -> *mut session_group;
    pub unsafe fn session_group_find(_: *const c_char) -> *mut session_group;
    pub unsafe fn session_group_new(_: *const c_char) -> *mut session_group;
    pub unsafe fn session_group_add(_: *mut session_group, _: *mut session);
    pub unsafe fn session_group_synchronize_to(_: *mut session);
    pub unsafe fn session_group_synchronize_from(_: *mut session);
    pub unsafe fn session_group_count(_: *mut session_group) -> c_uint;
    pub unsafe fn session_group_attached_count(_: *mut session_group) -> c_uint;
    pub unsafe fn session_renumber_windows(_: *mut session);
}

RB_GENERATE!(sessions, session, entry, session_cmp);
RB_GENERATE!(session_groups, session_group, entry, session_group_cmp);
