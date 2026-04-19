//! Shared data types for tmux-rs.
//!
//! This crate holds types that are used across multiple tmux-rs subsystems
//! (grid, screen, tty, input, format). Each type is pure data with no
//! dependencies on tmux-rs internals, so downstream crates (currently
//! `tmux-rs`, eventually `tmux-grid` et al.) can depend on `tmux-types`
//! without pulling in the full tmux codebase.
//!
//! **Naming.** Types keep their historical snake_case names (`grid_cell`,
//! `grid_attr`, …) to minimize churn during extraction. A CamelCase
//! rename pass is tracked separately (see PLAN.md).

#![allow(non_camel_case_types)]
// `grid_cell_entry_union` holds an unnamed union with a trivially-Copy
// inner type; Rust's safe-projection lints don't quite understand that
// and warn even though the access is guarded by a discriminant flag.
#![allow(unsafe_op_in_unsafe_fn)]

mod cell;
mod colour;
mod grid_flags;
mod line;
mod utf8;

pub use cell::*;
pub use colour::*;
pub use grid_flags::*;
pub use line::*;
pub use utf8::*;

/// Grid-level flag: this grid retains scrollback history. Passed to
/// [`grid_create`](../tmux_rs/fn.grid_create.html) when the caller wants
/// scrollback; omitted for ephemeral screens (popups, menus, alternate
/// screen). Bit `0x1` in `grid.flags`.
pub const GRID_HISTORY: i32 = 0x1;
