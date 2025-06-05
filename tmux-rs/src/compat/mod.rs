use core::ffi::{c_char, c_int, c_longlong, c_void};

use libc::{S_IRWXG, S_IRWXO, S_IRWXU, gid_t, mode_t, pid_t, uid_t};

pub mod fdforkpty;
pub mod getdtablecount;
pub mod getprogname;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
pub mod systemd;
pub mod tree;
pub mod vis_;

pub use systemd::systemd_create_socket;

pub(crate) use queue::{TAILQ_HEAD_INITIALIZER, impl_tailq_entry, tailq_insert_head};
pub(crate) use tree::{RB_GENERATE, RB_GENERATE_STATIC};

// pub use bsd_sys::{bsd_getopt, optarg as BSDoptarg, optind as BSDoptind};
// bsd_getopt, closefrom, getpeereid, optarg, optind, recallocarray, setproctitle, strlcat, strlcpy, strtonum, vis,

pub const VIS_OCTAL: i32 = 1;
pub const VIS_CSTYLE: i32 = 2;
pub const VIS_TAB: i32 = 8;
pub const VIS_NL: i32 = 16;
pub const VIS_GLOB: i32 = 4096;
pub const VIS_DQ: i32 = 32768;

// from libbsd
unsafe extern "C" {
    pub fn fgetln(stream: *mut libc::FILE, len: *mut usize) -> *mut c_char;

    pub fn getprogname() -> *const c_char;
    pub fn recallocarray(ptr: *mut c_void, oldnmemb: usize, nmemb: usize, size: usize) -> *mut c_void;
    pub fn freezero(ptr: *mut c_void, size: usize);
    pub fn strtonum(nptr: *const c_char, minval: c_longlong, maxval: c_longlong, errstr: *mut *const c_char) -> c_longlong;
    pub fn strlcpy(dst: *mut c_char, src: *const c_char, siz: usize) -> usize;
    pub fn strlcat(dst: *mut c_char, src: *const c_char, siz: usize) -> usize;
    pub static mut optarg: *mut c_char;
    pub static mut optind: c_int;
    pub fn getopt(___argc: c_int, ___argv: *const *mut c_char, __shortopts: *const c_char) -> c_int;
    pub fn closefrom(__lowfd: c_int);
    pub static mut optreset: c_int;
    pub fn bsd_getopt(argc: c_int, argv: *const *mut c_char, shortopts: *const c_char) -> c_int;
    pub fn setproctitle(fmt: *const c_char, ...);
    pub fn getpeereid(s: c_int, euid: *mut uid_t, egid: *mut gid_t) -> c_int;
    pub fn vis(arg1: *mut c_char, arg2: c_int, arg3: c_int, arg4: c_int) -> *mut c_char;
    pub fn stravis(arg1: *mut *mut c_char, arg2: *const c_char, arg3: c_int) -> c_int;
    pub fn strnvis(arg1: *mut c_char, arg2: *const c_char, arg3: usize, arg4: c_int) -> c_int;
    pub fn strunvis(arg1: *mut c_char, arg2: *const c_char) -> c_int;

    pub fn __b64_ntop(src: *const u8, srclength: usize, target: *mut c_char, targsize: usize) -> i32;
    pub fn __b64_pton(src: *const c_char, target: *mut u8, targsize: usize) -> i32;
}
// TODO switch to using the base64 crate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn b64_ntop(src: *const u8, srclength: usize, target: *mut c_char, targsize: usize) -> i32 { unsafe { __b64_ntop(src, srclength, target, targsize) } }

// skips all whitespace anywhere.
// converts characters, four at a time, starting at (or after)
// src from base - 64 numbers into three 8 bit bytes in the target area.
// it returns the number of data bytes stored at the target, or -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn b64_pton(src: *const c_char, target: *mut u8, targsize: usize) -> i32 { unsafe { __b64_pton(src, target, targsize) } }

pub const HOST_NAME_MAX: usize = 255;

pub const WAIT_ANY: pid_t = -1;

pub const ACCESSPERMS: mode_t = S_IRWXU | S_IRWXG | S_IRWXO;

// extern crate compat_derive;
// pub use compat_derive::TailQEntry;

macro_rules! errno {
    () => {
        *::libc::__errno_location()
    };
}
use errno;
