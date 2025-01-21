use super::*;

unsafe extern "C" {
    pub static mut tty_terms: tty_terms;
    pub fn tty_term_ncodes() -> c_uint;
    pub fn tty_term_apply(_: *mut tty_term, _: *const c_char, _: c_int);
    pub fn tty_term_apply_overrides(_: *mut tty_term);
    pub fn tty_term_create(
        _: *mut tty,
        _: *mut c_char,
        _: *mut *mut c_char,
        _: c_uint,
        _: *mut c_int,
        _: *mut *mut c_char,
    ) -> *mut tty_term;
    pub fn tty_term_free(_: *mut tty_term);
    pub fn tty_term_read_list(
        _: *const c_char,
        _: c_int,
        _: *mut *mut *mut c_char,
        _: *mut c_uint,
        _: *mut *mut c_char,
    ) -> c_int;
    pub fn tty_term_free_list(_: *mut *mut c_char, _: c_uint);
    pub fn tty_term_has(_: *mut tty_term, _: tty_code_code) -> c_int;
    pub fn tty_term_string(_: *mut tty_term, _: tty_code_code) -> *const c_char;
    pub fn tty_term_string_i(_: *mut tty_term, _: tty_code_code, _: c_int) -> *const c_char;
    pub fn tty_term_string_ii(_: *mut tty_term, _: tty_code_code, _: c_int, _: c_int) -> *const c_char;
    pub fn tty_term_string_iii(_: *mut tty_term, _: tty_code_code, _: c_int, _: c_int, _: c_int) -> *const c_char;
    pub fn tty_term_string_s(_: *mut tty_term, _: tty_code_code, _: *const c_char) -> *const c_char;
    pub fn tty_term_string_ss(_: *mut tty_term, _: tty_code_code, _: *const c_char, _: *const c_char) -> *const c_char;
    pub fn tty_term_number(_: *mut tty_term, _: tty_code_code) -> c_int;
    pub fn tty_term_flag(_: *mut tty_term, _: tty_code_code) -> c_int;
    pub fn tty_term_describe(_: *mut tty_term, _: tty_code_code) -> *const c_char;
}
