pub const PROTOCOL_VERSION: i32 = 8;

/// Message types.
#[repr(i32)]
pub enum msgtype {
    MSG_VERSION = 12,

    MSG_IDENTIFY_FLAGS = 100,
    MSG_IDENTIFY_TERM,
    MSG_IDENTIFY_TTYNAME,
    MSG_IDENTIFY_OLDCWD, /* unused */
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
    MSG_OLDSTDERR, /* unused */
    MSG_OLDSTDIN,  /* unused */
    MSG_OLDSTDOUT, /* unused */
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

#[repr(C)]
pub struct msg_command {
    pub args: i32,
}

#[repr(C)]
pub struct msg_read_open {
    pub stream: i32,
    pub fd: i32,
}

#[repr(C)]
pub struct msg_read_data {
    pub stream: i32,
}

#[repr(C)]
pub struct msg_read_done {
    pub stream: i32,
    pub error: i32,
}

#[repr(C)]
pub struct msg_read_cancel {
    pub stream: i32,
}

#[repr(C)]
pub struct msg_write_open {
    pub stream: i32,
    pub fd: i32,
    pub flags: i32,
}

#[repr(C)]
pub struct msg_write_data {
    pub stream: i32,
}

#[repr(C)]
pub struct msg_write_ready {
    pub stream: i32,
    pub error: i32,
}

#[repr(C)]
pub struct msg_write_close {
    pub stream: i32,
}
