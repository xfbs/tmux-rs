//! Client/server IPC protocol for tmux-rs.
//!
//! This crate defines the wire format used between the tmux server and
//! its clients: the [`msgtype`] enum of recognized message kinds, the
//! `msg_*` payload structs that accompany them, and the
//! [`PROTOCOL_VERSION`] constant that gates handshake compatibility.
//!
//! The imsg framing and fd-passing helpers will move in as follow-up
//! extraction steps; for now this crate holds the type vocabulary only.
//!
//! Consumers (tmux-rs) re-export these through `crate::tmux_protocol`
//! so existing `use crate::msgtype` style call sites keep resolving.

// Names mirror the upstream tmux C header wire definitions.
#![allow(non_camel_case_types)]

/// Wire-format version. Bumped when a breaking change lands in
/// [`msgtype`] variants or `msg_*` payload shapes. The client and
/// server exchange this in the `MSG_VERSION` handshake and refuse to
/// continue on mismatch.
pub const PROTOCOL_VERSION: i32 = 8;

/// Message types exchanged on the client/server imsg socket.
///
/// The three numeric ranges group messages by lifecycle:
/// - `100..=112` — client identification (sent during the handshake).
/// - `200..=218` — runtime commands and lifecycle events (attach,
///   detach, resize, shutdown, etc.).
/// - `300..=307` — file read/write streaming for commands that proxy
///   I/O between client and server (`load-buffer`, `save-buffer`, ...).
///
/// Zero is reserved so a zero-initialized `msgtype` isn't undefined
/// behavior (Rust forbids out-of-range enum values).
#[repr(i32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum msgtype {
    MSG_ZERO = 0, // TODO rust added so not ub on static init
    MSG_VERSION = 12,

    MSG_IDENTIFY_FLAGS = 100,
    MSG_IDENTIFY_TERM,
    MSG_IDENTIFY_TTYNAME,
    MSG_IDENTIFY_OLDCWD, // unused
    MSG_IDENTIFY_STDIN,
    MSG_IDENTIFY_ENVIRON,
    MSG_IDENTIFY_DONE,
    MSG_IDENTIFY_CLIENTPID,
    MSG_IDENTIFY_CWD,
    MSG_IDENTIFY_FEATURES,
    MSG_IDENTIFY_STDOUT,
    MSG_IDENTIFY_LONGFLAGS,
    MSG_IDENTIFY_TERMINFO,

    MSG_COMMAND = 200,
    MSG_DETACH,
    MSG_DETACHKILL,
    MSG_EXIT,
    MSG_EXITED,
    MSG_EXITING,
    MSG_LOCK,
    MSG_READY,
    MSG_RESIZE,
    MSG_SHELL,
    MSG_SHUTDOWN,
    MSG_OLDSTDERR, // unused
    MSG_OLDSTDIN,  // unused
    MSG_OLDSTDOUT, // unused
    MSG_SUSPEND,
    MSG_UNLOCK,
    MSG_WAKEUP,
    MSG_EXEC,
    MSG_FLAGS,

    MSG_READ_OPEN = 300,
    MSG_READ,
    MSG_READ_DONE,
    MSG_WRITE_OPEN,
    MSG_WRITE,
    MSG_WRITE_READY,
    MSG_WRITE_CLOSE,
    MSG_READ_CANCEL,
}

/// Error returned by [`msgtype::try_from`] when the numeric value on
/// the wire doesn't correspond to any known variant. Appears when a
/// peer advertises a protocol version we recognize but sends a message
/// kind that was added in a later revision — handled by dropping the
/// message.
#[derive(Debug)]
pub struct InvalidEnumValue;

impl TryFrom<u32> for msgtype {
    type Error = InvalidEnumValue;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => msgtype::MSG_ZERO,
            12 => msgtype::MSG_VERSION,
            100 => msgtype::MSG_IDENTIFY_FLAGS,
            101 => msgtype::MSG_IDENTIFY_TERM,
            102 => msgtype::MSG_IDENTIFY_TTYNAME,
            103 => msgtype::MSG_IDENTIFY_OLDCWD,
            104 => msgtype::MSG_IDENTIFY_STDIN,
            105 => msgtype::MSG_IDENTIFY_ENVIRON,
            106 => msgtype::MSG_IDENTIFY_DONE,
            107 => msgtype::MSG_IDENTIFY_CLIENTPID,
            108 => msgtype::MSG_IDENTIFY_CWD,
            109 => msgtype::MSG_IDENTIFY_FEATURES,
            110 => msgtype::MSG_IDENTIFY_STDOUT,
            111 => msgtype::MSG_IDENTIFY_LONGFLAGS,
            112 => msgtype::MSG_IDENTIFY_TERMINFO,
            200 => msgtype::MSG_COMMAND,
            201 => msgtype::MSG_DETACH,
            202 => msgtype::MSG_DETACHKILL,
            203 => msgtype::MSG_EXIT,
            204 => msgtype::MSG_EXITED,
            205 => msgtype::MSG_EXITING,
            206 => msgtype::MSG_LOCK,
            207 => msgtype::MSG_READY,
            208 => msgtype::MSG_RESIZE,
            209 => msgtype::MSG_SHELL,
            210 => msgtype::MSG_SHUTDOWN,
            211 => msgtype::MSG_OLDSTDERR,
            212 => msgtype::MSG_OLDSTDIN,
            213 => msgtype::MSG_OLDSTDOUT,
            214 => msgtype::MSG_SUSPEND,
            215 => msgtype::MSG_UNLOCK,
            216 => msgtype::MSG_WAKEUP,
            217 => msgtype::MSG_EXEC,
            218 => msgtype::MSG_FLAGS,
            300 => msgtype::MSG_READ_OPEN,
            301 => msgtype::MSG_READ,
            302 => msgtype::MSG_READ_DONE,
            303 => msgtype::MSG_WRITE_OPEN,
            304 => msgtype::MSG_WRITE,
            305 => msgtype::MSG_WRITE_READY,
            306 => msgtype::MSG_WRITE_CLOSE,
            307 => msgtype::MSG_READ_CANCEL,
            _ => return Err(InvalidEnumValue),
        })
    }
}

/// Payload of [`msgtype::MSG_COMMAND`]: argv length of the command.
/// The argv bytes themselves follow the struct in the imsg payload.
#[repr(C)]
pub struct msg_command {
    pub argc: i32,
}

/// Payload of [`msgtype::MSG_READ_OPEN`]: open a read stream whose
/// bytes the server will forward to the client via MSG_READ messages.
/// `fd` carries the source descriptor via SCM_RIGHTS.
#[repr(C)]
pub struct msg_read_open {
    pub stream: i32,
    pub fd: i32,
}

/// Payload of [`msgtype::MSG_READ`]: stream id; the actual bytes follow
/// in the imsg payload after the struct.
#[repr(C)]
pub struct msg_read_data {
    pub stream: i32,
}

/// Payload of [`msgtype::MSG_READ_DONE`]: EOF (or terminal error) on a
/// read stream. `error` is 0 on clean EOF, or an errno otherwise.
#[repr(C)]
pub struct msg_read_done {
    pub stream: i32,
    pub error: i32,
}

/// Payload of [`msgtype::MSG_READ_CANCEL`]: abort an in-flight read
/// stream (e.g. client lost interest in the bytes being forwarded).
#[repr(C)]
pub struct msg_read_cancel {
    pub stream: i32,
}

/// Payload of [`msgtype::MSG_WRITE_OPEN`]: open a write stream that the
/// client will use to forward bytes to `fd`. `flags` carries `open(2)`
/// flags.
#[repr(C)]
pub struct msg_write_open {
    pub stream: i32,
    pub fd: i32,
    pub flags: i32,
}

/// Payload of [`msgtype::MSG_WRITE`]: stream id; the bytes to write
/// follow in the imsg payload after the struct.
#[repr(C)]
pub struct msg_write_data {
    pub stream: i32,
}

/// Payload of [`msgtype::MSG_WRITE_READY`]: the server is ready for
/// more bytes on a write stream (flow control ack). `error` is 0 or an
/// errno on failure.
#[repr(C)]
pub struct msg_write_ready {
    pub stream: i32,
    pub error: i32,
}

/// Payload of [`msgtype::MSG_WRITE_CLOSE`]: flush-and-close a write
/// stream.
#[repr(C)]
pub struct msg_write_close {
    pub stream: i32,
}
