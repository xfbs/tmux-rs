//! Bitflag types describing per-cell, per-line, and per-string styling options.
//!
//! These are the "pure data" portion of the grid model — moved first into
//! `tmux-types` because they have no dependencies on other grid helpers.
//! The types, their constants, and the `GRID_ATTR_ALL_UNDERSCORE` aggregate
//! all preserve their original names and values so the rest of the codebase
//! (which re-imports them via `pub use tmux_types::*;`) is unaffected.

bitflags::bitflags! {
    /// Per-cell text attributes: bright/dim/italic/etc. SGR state. Stored
    /// packed in `grid_cell.attr` and propagated through the renderer.
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    pub struct grid_attr : u16 {
        const GRID_ATTR_BRIGHT = 0x1;
        const GRID_ATTR_DIM = 0x2;
        const GRID_ATTR_UNDERSCORE = 0x4;
        const GRID_ATTR_BLINK = 0x8;
        const GRID_ATTR_REVERSE = 0x10;
        const GRID_ATTR_HIDDEN = 0x20;
        const GRID_ATTR_ITALICS = 0x40;
        const GRID_ATTR_CHARSET = 0x80; // alternative character set
        const GRID_ATTR_STRIKETHROUGH = 0x100;
        const GRID_ATTR_UNDERSCORE_2 = 0x200;
        const GRID_ATTR_UNDERSCORE_3 = 0x400;
        const GRID_ATTR_UNDERSCORE_4 = 0x800;
        const GRID_ATTR_UNDERSCORE_5 = 0x1000;
        const GRID_ATTR_OVERLINE = 0x2000;
    }
}

/// Combined mask for every underscore variant — used to zero out all
/// underscore bits atomically when style resets (e.g. when `NOUNDERSCORE`
/// is selected).
pub const GRID_ATTR_ALL_UNDERSCORE: grid_attr = grid_attr::GRID_ATTR_UNDERSCORE
    .union(grid_attr::GRID_ATTR_UNDERSCORE_2)
    .union(grid_attr::GRID_ATTR_UNDERSCORE_3)
    .union(grid_attr::GRID_ATTR_UNDERSCORE_4)
    .union(grid_attr::GRID_ATTR_UNDERSCORE_5);

bitflags::bitflags! {
    /// Per-cell flags describing storage form and rendering state
    /// (padding cell, extended-style side-table entry, cleared, etc.).
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct grid_flag : u8 {
        const FG256 = 0x1;
        const BG256 = 0x2;
        const PADDING = 0x4;
        const EXTENDED = 0x8;
        const SELECTED = 0x10;
        const NOPALETTE = 0x20;
        const CLEARED = 0x40;
    }
}

bitflags::bitflags! {
    /// Per-line flags: wrapping, liveness, and prompt/output markers used
    /// for copy-mode search and command-history scrubbing.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct grid_line_flag: i32 {
        const WRAPPED      = 1 << 0; // 0x1
        const EXTENDED     = 1 << 1; // 0x2
        const DEAD         = 1 << 2; // 0x4
        const START_PROMPT = 1 << 3; // 0x8
        const START_OUTPUT = 1 << 4; // 0x10
    }
}

bitflags::bitflags! {
    /// Options for `grid::string_cells` — whether to emit escape sequences,
    /// escape them for printing, trim trailing spaces, or include padding cells.
    #[repr(transparent)]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct grid_string_flags: i32 {
        const GRID_STRING_WITH_SEQUENCES = 0x1;
        const GRID_STRING_ESCAPE_SEQUENCES = 0x2;
        const GRID_STRING_TRIM_SPACES = 0x4;
        const GRID_STRING_USED_ONLY = 0x8;
        const GRID_STRING_EMPTY_CELLS = 0x10;
    }
}
