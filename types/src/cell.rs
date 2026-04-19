//! Per-cell data types: `GridCell` and its packed/extended companions.
//!
//! Each screen position is represented by a `GridCell` during computation,
//! but stored on disk-like [`GridLine`](super::GridLine) storage as a
//! compact `GridCellEntry`. When a cell carries styling that doesn't fit
//! the packed form (RGB fg/bg, extended attributes, hyperlinks), it spills
//! into a `GridExtdEntry` side-table indexed by
//! [`GridCellEntryUnion::offset`].
//!
//! The impl blocks for `Grid` itself (accessors, mutators, scroll, reflow)
//! remain in `tmux-rs::grid_.rs` — that's where they have access to the
//! process-global timestamp and the other tmux-rs helpers. The pure data
//! definitions live here so future consumers (`tmux-grid`) can depend on
//! the types without pulling the whole tmux-rs crate.

use crate::{GridFlag, Utf8Char, Utf8Data};

/// Primary in-memory representation of a styled terminal cell.
///
/// `Copy` because it's tiny (~40 bytes) and gets passed around freely
/// by value during rendering. For the on-Grid packed form see
/// [`GridCellEntry`]; for the RGB/extended spill-over see
/// [`GridExtdEntry`].
#[derive(Copy, Clone)]
pub struct GridCell {
    pub data: Utf8Data,
    pub attr: crate::GridAttr,
    pub flags: GridFlag,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

impl GridCell {
    /// Const constructor — used by the `GRID_DEFAULT_CELL` / `PADDING_CELL`
    /// / `CLEARED_CELL` statics. All fields are positional to mirror the
    /// original C struct literal form.
    pub const fn new(
        data: Utf8Data,
        attr: crate::GridAttr,
        flags: GridFlag,
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
/// `GridCellEntry.union_.offset` when `GridFlag::EXTENDED` is set.
#[derive(Copy, Clone)]
pub struct GridExtdEntry {
    pub data: Utf8Char,
    pub attr: u16,
    pub flags: u8,
    pub fg: i32,
    pub bg: i32,
    pub us: i32,
    pub link: u32,
}

/// Packed form used when the cell's style fits in 8-bit-per-channel
/// colours and the base attribute set. Lives inside the tagged union
/// [`GridCellEntryUnion`].
#[derive(Copy, Clone)]
#[repr(C, align(4))]
pub struct GridCellEntryData {
    pub attr: u8,
    pub fg: u8,
    pub bg: u8,
    pub data: u8,
}

/// Tagged union discriminated by `GridCellEntry::flags`:
/// - `GridFlag::EXTENDED` set → `offset` is an index into the line's
///   `extddata: Vec<GridExtdEntry>`.
/// - otherwise → `data` holds the packed-form cell directly.
#[derive(Copy, Clone)]
pub union GridCellEntryUnion {
    pub offset: u32,
    pub data: GridCellEntryData,
}

/// One cell as stored on a `GridLine`. 5 bytes packed (`union_` is 4
/// bytes wide and aligned to 4). Turn into a full [`GridCell`] via
/// [`grid_get_cell`](../../tmux_rs/fn.grid_get_cell.html) or the
/// corresponding method on `Grid`.
#[derive(Copy, Clone)]
pub struct GridCellEntry {
    pub union_: GridCellEntryUnion,
    pub flags: GridFlag,
}
