use super::*;

unsafe extern "C" {
    pub unsafe fn cmdq_new_state(_: *mut cmd_find_state, _: *mut key_event, _: c_int) -> *mut cmdq_state;
    pub unsafe fn cmdq_link_state(_: *mut cmdq_state) -> *mut cmdq_state;
    pub unsafe fn cmdq_copy_state(_: *mut cmdq_state, _: *mut cmd_find_state) -> *mut cmdq_state;
    pub unsafe fn cmdq_free_state(_: *mut cmdq_state);
    pub unsafe fn cmdq_add_format(_: *mut cmdq_state, _: *const c_char, _: *const c_char, ...);
    pub unsafe fn cmdq_add_formats(_: *mut cmdq_state, _: *mut format_tree);
    pub unsafe fn cmdq_merge_formats(_: *mut cmdq_item, _: *mut format_tree);
    pub unsafe fn cmdq_new() -> *mut cmdq_list;
    pub unsafe fn cmdq_free(_: *mut cmdq_list);
    pub unsafe fn cmdq_get_name(_: *mut cmdq_item) -> *const c_char;
    pub unsafe fn cmdq_get_client(_: *mut cmdq_item) -> *mut client;
    pub unsafe fn cmdq_get_target_client(_: *mut cmdq_item) -> *mut client;
    pub unsafe fn cmdq_get_state(_: *mut cmdq_item) -> *mut cmdq_state;
    pub unsafe fn cmdq_get_target(_: *mut cmdq_item) -> *mut cmd_find_state;
    pub unsafe fn cmdq_get_source(_: *mut cmdq_item) -> *mut cmd_find_state;
    pub unsafe fn cmdq_get_event(_: *mut cmdq_item) -> *mut key_event;
    pub unsafe fn cmdq_get_current(_: *mut cmdq_item) -> *mut cmd_find_state;
    pub unsafe fn cmdq_get_flags(_: *mut cmdq_item) -> c_int;
    pub unsafe fn cmdq_get_command(_: *mut cmd_list, _: *mut cmdq_state) -> *mut cmdq_item;
    pub unsafe fn cmdq_get_callback1(_: *const c_char, _: cmdq_cb, _: *mut c_void) -> *mut cmdq_item;
    pub unsafe fn cmdq_get_error(_: *const c_char) -> *mut cmdq_item;
    pub unsafe fn cmdq_insert_after(_: *mut cmdq_item, _: *mut cmdq_item) -> *mut cmdq_item;
    pub unsafe fn cmdq_append(_: *mut client, _: *mut cmdq_item) -> *mut cmdq_item;
    pub unsafe fn cmdq_insert_hook(_: *mut session, _: *mut cmdq_item, _: *mut cmd_find_state, _: *const c_char, ...);
    pub unsafe fn cmdq_continue(_: *mut cmdq_item);
    pub unsafe fn cmdq_next(_: *mut client) -> c_uint;
    pub unsafe fn cmdq_running(_: *mut client) -> *mut cmdq_item;
    pub unsafe fn cmdq_guard(_: *mut cmdq_item, _: *const c_char, _: c_int);
    pub unsafe fn cmdq_print(_: *mut cmdq_item, _: *const c_char, ...);
    pub unsafe fn cmdq_print_data(_: *mut cmdq_item, _: c_int, _: *mut evbuffer);
    pub unsafe fn cmdq_error(_: *mut cmdq_item, _: *const c_char, ...);
}

macro_rules! cstringify {
    ($e:expr) => {
        unsafe { ::core::ffi::CStr::from_bytes_with_nul_unchecked(concat!(stringify!($e), "\0").as_bytes()).as_ptr() }
    };
}
pub(crate) use cstringify;

// #define cmdq_get_callback(cb, data) cmdq_get_callback1(#cb, cb, data)
#[macro_export]
macro_rules! cmdq_get_callback {
    ($cb:ident, $data:expr) => {
        $crate::cmd_::cmd_queue::cmdq_get_callback1($crate::cmd_::cmd_queue::cstringify!($cb), Some($cb), $data)
    };
}
pub use cmdq_get_callback;
