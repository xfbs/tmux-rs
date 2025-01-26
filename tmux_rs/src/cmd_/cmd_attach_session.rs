use super::*;

unsafe extern "C" {
    pub unsafe fn cmd_attach_session(
        _: *mut cmdq_item,
        _: *const c_char,
        _: c_int,
        _: c_int,
        _: c_int,
        _: *const c_char,
        _: c_int,
        _: *const c_char,
    ) -> cmd_retval;
}
