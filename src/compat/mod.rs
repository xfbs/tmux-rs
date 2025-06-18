use core::ffi::{c_char, c_int, c_void};

pub mod b64;
pub mod fdforkpty;
pub mod getdtablecount;
pub mod getprogname;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
mod strtonum;
pub mod systemd;
pub mod tree;
pub mod vis_;

pub use strtonum::{strtonum, strtonum_};
pub use systemd::systemd_create_socket;

pub(crate) use queue::{TAILQ_HEAD_INITIALIZER, impl_tailq_entry, tailq_insert_head};
pub(crate) use tree::RB_GENERATE;

pub const VIS_OCTAL: i32 = 1;
pub const VIS_CSTYLE: i32 = 2;
pub const VIS_TAB: i32 = 8;
pub const VIS_NL: i32 = 16;
pub const VIS_GLOB: i32 = 4096;
pub const VIS_DQ: i32 = 32768;

// from libbsd
#[rustfmt::skip]
unsafe extern "C" {
    pub fn fgetln(stream: *mut libc::FILE, len: *mut usize) -> *mut c_char;

    pub fn strlcpy(dst: *mut c_char, src: *const c_char, siz: usize) -> usize;
    pub fn strlcat(dst: *mut c_char, src: *const c_char, siz: usize) -> usize;

    pub fn setproctitle(fmt: *const c_char, ...);
    pub fn getprogname() -> *const c_char;

    pub fn recallocarray(ptr: *mut c_void, oldnmemb: usize, nmemb: usize, size: usize) -> *mut c_void;
    pub fn freezero(ptr: *mut c_void, size: usize);

    pub fn getpeereid(s: c_int, euid: *mut libc::uid_t, egid: *mut libc::gid_t) -> c_int;

    pub fn closefrom(__lowfd: c_int);

    pub fn vis(arg1: *mut c_char, arg2: c_int, arg3: c_int, arg4: c_int) -> *mut c_char;
    pub fn stravis(arg1: *mut *mut c_char, arg2: *const c_char, arg3: c_int) -> c_int;
    pub fn strnvis(arg1: *mut c_char, arg2: *const c_char, arg3: usize, arg4: c_int) -> c_int;
    pub fn strunvis(arg1: *mut c_char, arg2: *const c_char) -> c_int;

    pub static mut optreset: c_int;
    pub static mut optarg: *mut c_char;
    pub static mut optind: c_int;
    pub fn getopt(___argc: c_int, ___argv: *const *mut c_char, __shortopts: *const c_char) -> c_int;
    pub fn bsd_getopt(argc: c_int, argv: *const *mut c_char, shortopts: *const c_char) -> c_int;
}

pub const HOST_NAME_MAX: usize = 255;

pub const WAIT_ANY: libc::pid_t = -1;

pub const ACCESSPERMS: libc::mode_t = libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO;

// #define S_ISDIR(mode)  (((mode) & S_IFMT) == S_IFDIR)
// TODO move this to a better spot
#[allow(non_snake_case)]
#[inline]
pub fn S_ISDIR(mode: u32) -> bool {
    mode & libc::S_IFMT == libc::S_IFDIR
}

// extern crate compat_derive;
// pub use compat_derive::TailQEntry;

macro_rules! errno {
    () => {
        *::libc::__errno_location()
    };
}
use errno;

mod bsd {
    // symbols used by libbsd:
    // - setproctitle
    // - strlcpy
    // - setproctitle_init
    // - strunvis
    // - recallocarray
    // - freezero
    // - strnvis
    // - vis
    // - fgetln
    // - stravis
}
