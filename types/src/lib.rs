//! Shared terminal data vocabulary for tmux-rs.
//!
//! This crate holds genuinely cross-cutting types — the ones every
//! subsystem (grid, screen, tty, input, format, status) needs to agree
//! on. Grid-specific types (`GridCell`, `GridLine`, `GridFlag`, etc.)
//! live in the `tmux-grid` crate; terminal-wide primitives
//! (`Utf8Data`, colour encoding) live here.
//!
//! Contents:
//! - `Utf8Data` / `Utf8Char` / `UTF8_SIZE` — UTF-8 cell data used by any
//!   renderer that writes characters.
//! - Colour constants (`COLOUR_FLAG_256`, `COLOUR_FLAG_RGB`) and
//!   helpers (`colour_split_rgb`, `colour_join_rgb`, `COLOUR_DEFAULT`).

mod colour;
mod utf8;

pub use colour::*;
pub use utf8::*;
