use super::*;
unsafe extern "C" {
    pub unsafe fn server_acl_init();
    pub unsafe fn server_acl_user_find(_: uid_t) -> *mut server_acl_user;
    pub unsafe fn server_acl_display(_: *mut cmdq_item);
    pub unsafe fn server_acl_user_allow(_: uid_t);
    pub unsafe fn server_acl_user_deny(_: uid_t);
    pub unsafe fn server_acl_user_allow_write(_: uid_t);
    pub unsafe fn server_acl_user_deny_write(_: uid_t);
    pub unsafe fn server_acl_join(_: *mut client) -> c_int;
    pub unsafe fn server_acl_get_uid(_: *mut server_acl_user) -> uid_t;
}
