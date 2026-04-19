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

mod colour;
mod grid_flags;
mod utf8;

pub use colour::*;
pub use grid_flags::*;
pub use utf8::*;
