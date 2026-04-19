//! Re-export shim for the `tmux-protocol` crate.
//!
//! The wire-format types (`msgtype`, `msg_*` payloads, `PROTOCOL_VERSION`)
//! live in the standalone `tmux-protocol` crate. Re-exported here so
//! existing `use crate::msgtype` / `use crate::tmux_protocol::*` call
//! sites throughout tmux-rs keep resolving unchanged.

pub use tmux_protocol::{
    PROTOCOL_VERSION, msg_command, msg_read_cancel, msg_read_data, msg_read_done, msg_read_open,
    msg_write_close, msg_write_data, msg_write_open, msg_write_ready, msgtype,
};
