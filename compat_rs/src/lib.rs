pub mod fdforkpty;
pub mod getdtablecount;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
pub mod systemd;
pub mod tree;
pub mod vis;

pub use crate::systemd::systemd_create_socket;

pub use bsd_sys::recallocarray;
pub use bsd_sys::strlcpy;
// pub use bsd_sys::{bsd_getopt, optarg as BSDoptarg, optind as BSDoptind};
pub use bsd_sys::{bsd_getopt, optarg, optind, strtonum};

pub const HOST_NAME_MAX: usize = 255;

pub const WAIT_ANY: libc::pid_t = -1;

pub const ACCESSPERMS: libc::mode_t = (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO);

extern crate compat_derive;
pub use compat_derive::TailQEntry;
