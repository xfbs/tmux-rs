use super::*;
unsafe extern "C" {
    pub fn key_string_lookup_string(_: *const c_char) -> key_code;
    pub fn key_string_lookup_key(_: key_code, _: c_int) -> *const c_char;
}
