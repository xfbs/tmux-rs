use super::*;

pub type popup_close_cb =
    ::std::option::Option<unsafe extern "C" fn(_: ::std::os::raw::c_int, _: *mut ::std::os::raw::c_void)>;
pub type popup_finish_edit_cb = ::std::option::Option<
    unsafe extern "C" fn(_: *mut ::std::os::raw::c_char, _: usize, _: *mut ::std::os::raw::c_void),
>;
unsafe extern "C" {
    pub unsafe fn popup_display(
        _: c_int,
        _: box_lines,
        _: *mut cmdq_item,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *mut environ,
        _: *const c_char,
        _: c_int,
        _: *mut *mut c_char,
        _: *const c_char,
        _: *const c_char,
        _: *mut client,
        _: *mut session,
        _: *const c_char,
        _: *const c_char,
        _: popup_close_cb,
        _: *mut c_void,
    ) -> c_int;
    pub unsafe fn popup_editor(
        _: *mut client,
        _: *const c_char,
        _: usize,
        _: popup_finish_edit_cb,
        _: *mut c_void,
    ) -> c_int;
}
