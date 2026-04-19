//! Re-export shim for the `tmux-compat` crate.
//!
//! Most BSD/libc compatibility helpers (b64, vis/unvis, strlcat/strlcpy,
//! strtonum, reallocarray, recallocarray, closefrom, getpeereid,
//! setproctitle, getprogname, fdforkpty, ntohll, systemd) live in the
//! `tmux-compat` workspace crate. Re-exported here so existing
//! `use crate::compat::*` call sites resolve unchanged.
//!
//! Two submodules remain in-tree: `imsg` / `imsg_buffer` are thin
//! re-exports of the `tmux-protocol` crate (see `protocol/`), and
//! `getopt` still uses `crate::*` and is easier to leave in place for
//! now.

// Public submodules for callers that reach in via
// `crate::compat::b64::b64_ntop`, `crate::compat::recallocarray::recallocarray`, etc.
pub use tmux_compat::{
    b64, fdforkpty, getprogname, reallocarray, recallocarray, systemd,
};

// Flattened function re-exports preserving the old `crate::compat::foo`
// spellings from the pre-extraction src/compat/mod.rs.
pub use tmux_compat::{
    closefrom::closefrom,
    getpeereid::getpeereid,
    setproctitle::setproctitle_,
    strlcat::{strlcat, strlcat_},
    strlcpy::strlcpy,
    strtonum::{strtonum, strtonum_},
    unvis::strunvis,
    vis::*,
};

pub mod imsg;
pub mod imsg_buffer;
pub mod getopt;

pub const HOST_NAME_MAX: usize = 255;

pub const WAIT_ANY: libc::pid_t = -1;

pub const ACCESSPERMS: libc::mode_t = libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO;

// #define S_ISDIR(mode)  (((mode) & S_IFMT) == S_IFDIR)
// TODO move this to a better spot
#[expect(non_snake_case)]
#[inline]
pub fn S_ISDIR(mode: libc::mode_t) -> bool {
    mode & libc::S_IFMT == libc::S_IFDIR
}
