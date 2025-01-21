use super::*;
unsafe extern "C" {
    pub unsafe fn utf8_towc(_: *const utf8_data, _: *mut wchar_t) -> utf8_state;
    pub unsafe fn utf8_fromwc(wc: wchar_t, _: *mut utf8_data) -> utf8_state;
    pub unsafe fn utf8_in_table(_: wchar_t, _: *const wchar_t, _: c_uint) -> c_int;
    pub unsafe fn utf8_build_one(_: c_uchar) -> utf8_char;
    pub unsafe fn utf8_from_data(_: *const utf8_data, _: *mut utf8_char) -> utf8_state;
    pub unsafe fn utf8_to_data(_: utf8_char, _: *mut utf8_data);
    pub unsafe fn utf8_set(_: *mut utf8_data, _: c_uchar);
    pub unsafe fn utf8_copy(_: *mut utf8_data, _: *const utf8_data);
    pub unsafe fn utf8_open(_: *mut utf8_data, _: c_uchar) -> utf8_state;
    pub unsafe fn utf8_append(_: *mut utf8_data, _: c_uchar) -> utf8_state;
    pub unsafe fn utf8_isvalid(_: *const c_char) -> c_int;
    pub unsafe fn utf8_strvis(_: *mut c_char, _: *const c_char, _: usize, _: c_int) -> c_int;
    pub unsafe fn utf8_stravis(_: *mut *mut c_char, _: *const c_char, _: c_int) -> c_int;
    pub unsafe fn utf8_stravisx(_: *mut *mut c_char, _: *const c_char, _: usize, _: c_int) -> c_int;
    pub unsafe fn utf8_sanitize(_: *const c_char) -> *mut c_char;
    pub unsafe fn utf8_strlen(_: *const utf8_data) -> usize;
    pub unsafe fn utf8_strwidth(_: *const utf8_data, _: isize) -> c_uint;
    pub unsafe fn utf8_fromcstr(_: *const c_char) -> *mut utf8_data;
    pub unsafe fn utf8_tocstr(_: *mut utf8_data) -> *mut c_char;
    pub unsafe fn utf8_cstrwidth(_: *const c_char) -> c_uint;
    pub unsafe fn utf8_padcstr(_: *const c_char, _: c_uint) -> *mut c_char;
    pub unsafe fn utf8_rpadcstr(_: *const c_char, _: c_uint) -> *mut c_char;
    pub unsafe fn utf8_cstrhas(_: *const c_char, _: *const utf8_data) -> c_int;
}
