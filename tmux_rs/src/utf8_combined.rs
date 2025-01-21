use super::*;
unsafe extern "C" {
    pub unsafe fn utf8_has_zwj(_: *const utf8_data) -> c_int;
    pub unsafe fn utf8_is_zwj(_: *const utf8_data) -> c_int;
    pub unsafe fn utf8_is_vs(_: *const utf8_data) -> c_int;
    pub unsafe fn utf8_is_modifier(_: *const utf8_data) -> c_int;
}
