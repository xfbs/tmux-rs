//! Per-line storage: a `grid_line` owns the cells for one screen row
//! (or one history row, depending on its `y` position in the parent grid).
//!
//! Layout: `celldata` holds the packed-form cells in order, `extddata`
//! is a side-table for cells whose style needed the extended form, and
//! `flags` + `time` carry per-line metadata used by reflow, search, and
//! copy-mode.

use libc::time_t;

use crate::{grid_cell_entry, grid_extd_entry, grid_line_flag};

/// One row of cells. Lines are stored contiguously in
/// `grid.linedata: Vec<grid_line>`; logical "scrollback" is just the
/// prefix `0..hsize` and the visible screen is `hsize..hsize+sy`.
pub struct grid_line {
    pub celldata: Vec<grid_cell_entry>,
    /// Number of cells actually written (0..=celldata.len()). Trailing
    /// default cells are implicit. Used to trim output in
    /// `string_cells` when `GRID_STRING_EMPTY_CELLS` is not set.
    pub cellused: u32,

    pub extddata: Vec<grid_extd_entry>,

    pub flags: grid_line_flag,
    /// Wall-clock timestamp when this line was scrolled into history.
    /// `0` for lines that are still in the visible area.
    pub time: time_t,
}

impl grid_line {
    /// Create a new empty grid line — used both for initial grid
    /// construction and for refilling the visible area after a scroll.
    pub fn new() -> Self {
        Self {
            celldata: Vec::new(),
            cellused: 0,
            extddata: Vec::new(),
            flags: grid_line_flag::empty(),
            time: 0,
        }
    }

    /// Create a dead grid line (used by reflow to mark consumed lines).
    /// The line's Vec fields are empty and the `DEAD` flag is set so
    /// reflow can skip over it cheaply.
    pub fn new_dead() -> Self {
        Self {
            celldata: Vec::new(),
            cellused: 0,
            extddata: Vec::new(),
            flags: grid_line_flag::DEAD,
            time: 0,
        }
    }
}

impl Default for grid_line {
    fn default() -> Self {
        Self::new()
    }
}
