// C-ported code keeps its original snake_case type names (vis_flags,
// uint32_t, …). A crate-wide allow is cleaner than sprinkling attributes
// across every module.
#![allow(non_camel_case_types)]
// Several of the C ports use nested unsafe pointer ops inside
// `unsafe fn`; keep the 2024-edition "explicit unsafe block" lint off
// at the crate level so the minimal-churn ports don't need to be
// rewritten.
#![allow(unsafe_op_in_unsafe_fn)]

//! BSD/libc compatibility helpers used by tmux-rs.
//!
//! Small, standalone ports of OpenBSD/portable-libc routines that
//! aren't uniformly available across target platforms or whose
//! semantics the C tmux codebase relies on. Each submodule is
//! self-contained; they don't share state.
//!
//! Most are ported verbatim from upstream tmux's `compat/` directory
//! (originally from OpenBSD sources), preserving the C-shaped APIs
//! (raw pointers, NUL-terminated strings) that the rest of the
//! codebase already calls.
//!
//! - `b64` — `b64_ntop` / `b64_pton` base64 (en|de)coding
//! - `closefrom`, `fdforkpty`, `getpeereid`, `getprogname`,
//!   `setproctitle`, `systemd` — platform feature shims
//! - `ntohll` — 64-bit network-to-host
//! - `reallocarray`, `recallocarray` — overflow-checked allocation
//! - `strlcat`, `strlcpy`, `strtonum` — safer string/number helpers
//! - `vis`, `unvis` — BSD `vis(3)` / `unvis(3)` escape format

// Platform-specific errno accessor, used by reallocarray and vis. Matches
// the main crate's `crate::errno!()` macro.
#[cfg(target_os = "linux")]
macro_rules! errno {
    () => {
        *::libc::__errno_location()
    };
}
#[cfg(target_os = "macos")]
macro_rules! errno {
    () => {
        *::libc::__error()
    };
}
pub(crate) use errno;

pub mod b64;
pub mod closefrom;
pub mod fdforkpty;
pub mod getpeereid;
pub mod getprogname;
pub mod ntohll;
pub mod reallocarray;
pub mod recallocarray;
pub mod setproctitle;
pub mod strlcat;
pub mod strlcpy;
pub mod strtonum;
pub mod systemd;
pub mod unvis;
pub mod vis;

pub use closefrom::closefrom;
pub use getpeereid::getpeereid;
pub use setproctitle::setproctitle_;
pub use strlcat::{strlcat, strlcat_};
pub use strlcpy::strlcpy;
pub use strtonum::{strtonum, strtonum_};
pub use unvis::strunvis;
pub use vis::*;
