use ::core::ffi::c_char;

pub const ERR: i32 = -1;
pub const OK: i32 = 0;

#[allow(clippy::upper_case_acronyms)]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TERMINAL {
    _opaque: [u8; 0],
}

#[link(name = "ncurses")]
unsafe extern "C" {
    pub fn setupterm(term: *const c_char, filedes: i32, errret: *mut i32) -> i32;

    pub fn tiparm_s(expected: i32, mask: i32, str: *const c_char, ...) -> *mut c_char;
    pub fn tiparm(str: *const c_char, ...) -> *mut c_char;
    pub fn tparm(str: *const c_char, ...) -> *mut c_char;

    pub fn tigetflag(cap_code: *const c_char) -> i32;
    pub fn tigetnum(cap_code: *const c_char) -> i32;
    pub fn tigetstr(cap_code: *const c_char) -> *mut c_char;

    pub fn del_curterm(oterm: *mut TERMINAL) -> i32;
    pub static mut cur_term: *mut TERMINAL;
}
