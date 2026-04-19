//! Per-cell data types: `grid_cell` and its packed/extended companions.
//!
//! Each screen position is represented by a `grid_cell` during computation,
//! but stored on disk-like [`grid_line`](super::grid_line) storage as a
//! compact `grid_cell_entry`. When a cell carries styling that doesn't fit
//! the packed form (RGB fg/bg, extended attributes, hyperlinks), it spills
//! into a `grid_extd_entry` side-table indexed by
//! [`grid_cell_entry_union::offset`].
//!
//! The impl blocks for `grid` itself (accessors, mutators, scroll, reflow)
//! remain in `tmux-rs::grid_.rs` — that's where they have access to the
//! process-global timestamp and the other tmux-rs helpers. The pure data
//! definitions live here so future consumers (`tmux-grid`) can depend on
//! the types without pulling the whole tmux-rs crate.

use crate::{grid_flag, utf8_char, utf8_data};

/// Primary in-memory representation of a styled terminal cell.
///
/// `Copy` because it's tiny (~40 bytes) and gets passed around freely
/// by value during rendering. For the on-grid packed form see
/// [`grid_cell_entry`]; for the RGB/extended spill-over see
/// [`grid_extd_entry`].
#[derive(Copy, Clone)]
pub struct grid_cell {
    pub data: utf8_data,
    pub attr: crate::grid_attr,
    pub flags: grid_flag,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

impl grid_cell {
    /// Const constructor — used by the `GRID_DEFAULT_CELL` / `PADDING_CELL`
    /// / `CLEARED_CELL` statics. All fields are positional to mirror the
    /// original C struct literal form.
    pub const fn new(
        data: utf8_data,
        attr: crate::grid_attr,
        flags: grid_flag,
        fg: i32,
        bg: i32,
        us: i32,
        link: u32,
    ) -> Self {
        Self { data, attr, flags, fg, bg, us, link }
    }
}

/// Side-table entry for cells whose style overflows the packed form
/// (RGB colours, extended attributes, hyperlinks). Indexed by
/// `grid_cell_entry.union_.offset` when `grid_flag::EXTENDED` is set.
#[derive(Copy, Clone)]
pub struct grid_extd_entry {
    pub data: utf8_char,
    pub attr: u16,
    pub flags: u8,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

/// Packed form used when the cell's style fits in 8-bit-per-channel
/// colours and the base attribute set. Lives inside the tagged union
/// [`grid_cell_entry_union`].
#[derive(Copy, Clone)]
#[repr(C, align(4))]
pub struct grid_cell_entry_data {
    pub attr: u8,
    pub fg: u8,
    pub bg: u8,
    pub data: u8,
}

/// Tagged union discriminated by `grid_cell_entry::flags`:
/// - `grid_flag::EXTENDED` set → `offset` is an index into the line's
///   `extddata: Vec<grid_extd_entry>`.
/// - otherwise → `data` holds the packed-form cell directly.
#[derive(Copy, Clone)]
pub union grid_cell_entry_union {
    pub offset: u32,
    pub data: grid_cell_entry_data,
}

/// One cell as stored on a `grid_line`. 5 bytes packed (`union_` is 4
/// bytes wide and aligned to 4). Turn into a full [`grid_cell`] via
/// [`grid_get_cell`](../../tmux_rs/fn.grid_get_cell.html) or the
/// corresponding method on `grid`.
#[derive(Copy, Clone)]
pub struct grid_cell_entry {
    pub union_: grid_cell_entry_union,
    pub flags: grid_flag,
}
