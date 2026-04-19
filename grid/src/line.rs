//! Per-line storage: a `GridLine` owns the cells for one screen row
//! (or one history row, depending on its `y` position in the parent Grid).
//!
//! Layout: `celldata` holds the packed-form cells in order, `extddata`
//! is a side-table for cells whose style needed the extended form, and
//! `flags` + `time` carry per-line metadata used by reflow, search, and
//! copy-mode.

use libc::time_t;

use crate::{GridCellEntry, GridExtdEntry, GridLineFlag};

/// One row of cells. Lines are stored contiguously in
/// `Grid.linedata: Vec<GridLine>`; logical "scrollback" is just the
/// prefix `0..hsize` and the visible screen is `hsize..hsize+sy`.
pub struct GridLine {
    pub celldata: Vec<GridCellEntry>,
    /// Number of cells actually written (0..=celldata.len()). Trailing
    /// default cells are implicit. Used to trim output in
    /// `string_cells` when `GRID_STRING_EMPTY_CELLS` is not set.
    pub cellused: u32,

    pub extddata: Vec<GridExtdEntry>,

    pub flags: GridLineFlag,
    /// Wall-clock timestamp when this line was scrolled into history.
    /// `0` for lines that are still in the visible area.
    pub time: time_t,
}

impl GridLine {
    /// Create a new empty Grid line — used both for initial Grid
    /// construction and for refilling the visible area after a scroll.
    pub fn new() -> Self {
        Self {
            celldata: Vec::new(),
            cellused: 0,
            extddata: Vec::new(),
            flags: GridLineFlag::empty(),
            time: 0,
        }
    }

    /// Create a dead Grid line (used by reflow to mark consumed lines).
    /// The line's Vec fields are empty and the `DEAD` flag is set so
    /// reflow can skip over it cheaply.
    pub fn new_dead() -> Self {
        Self {
            celldata: Vec::new(),
            cellused: 0,
            extddata: Vec::new(),
            flags: GridLineFlag::DEAD,
            time: 0,
        }
    }
}

impl Default for GridLine {
    fn default() -> Self {
        Self::new()
    }
}
