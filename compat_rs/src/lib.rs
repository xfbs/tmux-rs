#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
pub mod fdforkpty;
pub mod getdtablecount;
pub mod imsg;
pub mod imsg_buffer;
pub mod queue;
pub mod tree;
pub mod vis;

pub use bsd_sys::recallocarray;
// pub use bsd_sys::{bsd_getopt, optarg as BSDoptarg, optind as BSDoptind};
pub use bsd_sys::{bsd_getopt, optarg, optind, strtonum};

pub const HOST_NAME_MAX: usize = 255;
