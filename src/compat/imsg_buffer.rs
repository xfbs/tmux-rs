//! Re-export shim for the imsg buffer primitives, now living in
//! `tmux-protocol`. See `protocol/src/imsg_buffer.rs` for the real
//! implementation.

// Same rationale as `compat::imsg`: the entire ibuf API is
// re-exported even where a given symbol has no active tmux-rs caller.
#![allow(unused_imports)]

pub use tmux_protocol::imsg_buffer::{
    ibuf_add, ibuf_add_buf, ibuf_add_h16, ibuf_add_h32, ibuf_add_h64, ibuf_add_ibuf, ibuf_add_n16,
    ibuf_add_n32, ibuf_add_n64, ibuf_add_n8, ibuf_add_zero, ibuf_close, ibuf_data, ibuf_dynamic,
    ibuf_fd_avail, ibuf_fd_get, ibuf_fd_set, ibuf_free, ibuf_from_buffer, ibuf_from_ibuf, ibuf_get,
    ibuf_get_h16, ibuf_get_h32, ibuf_get_h64, ibuf_get_ibuf, ibuf_get_n16, ibuf_get_n32,
    ibuf_get_n64, ibuf_get_n8, ibuf_left, ibuf_open, ibuf_realloc, ibuf_reserve, ibuf_rewind,
    ibuf_seek, ibuf_set, ibuf_set_h16, ibuf_set_h32, ibuf_set_h64, ibuf_set_n16, ibuf_set_n32,
    ibuf_set_n64, ibuf_set_n8, ibuf_size, ibuf_skip, ibuf_truncate, ibuf_write, msgbuf_clear,
    msgbuf_init, msgbuf_queuelen, msgbuf_write,
};
