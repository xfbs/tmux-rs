//! Re-export shim for the imsg layer, now living in `tmux-protocol`.
//!
//! The previous in-tree implementation moved verbatim to
//! `protocol/src/imsg.rs` during the P2 extraction step. This shim
//! keeps `use crate::compat::imsg::*` call sites throughout tmux-rs
//! resolving unchanged.

// `pub use` here is a transitional re-export surface; not every symbol
// is consumed inside tmux-rs today, but all are part of the imsg API
// the rest of the codebase is entitled to reach for.
#![allow(unused_imports)]

pub use tmux_protocol::imsg::{
    IBUF_READ_SIZE, IMSG_HEADER_SIZE, MAX_IMSGSIZE, ibuf, ibuf_read, imsg, imsg_add, imsg_clear,
    imsg_close, imsg_compose, imsg_compose_ibuf, imsg_composev, imsg_create, imsg_fd, imsg_flush,
    imsg_forward, imsg_free, imsg_get, imsg_get_data, imsg_get_fd, imsg_get_ibuf, imsg_get_id,
    imsg_get_len, imsg_get_pid, imsg_get_type, imsg_hdr, imsg_init, imsg_read, imsgbuf, msgbuf,
};
