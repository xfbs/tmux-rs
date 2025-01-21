use super::*;

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
