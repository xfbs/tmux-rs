//! Shared terminal data vocabulary for tmux-rs.
//!
//! Landing zone for truly cross-cutting definitions — types that every
//! subsystem (grid, screen, tty, input, format, status) needs to agree
//! on but that don't belong to any one of them.
//!
//! Today: colour encoding helpers only. May grow as further subsystems
//! get extracted; will shrink or dissolve if nothing generic remains.

mod colour;

pub use colour::*;
