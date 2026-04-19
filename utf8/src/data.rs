//! UTF-8 character data types used to describe a single displayable cell.
//!
//! Only the data types move here; richer parsing and width-lookup helpers
//! remain in `tmux-rs::utf8` for now because they depend on platform
//! libraries (wcwidth, optional `utf8proc`). A future extraction may split
//! the parsing side into its own crate.

use std::ffi::c_uint;

/// A single stored UTF-8 character, packed into one `u32`. The encoding
/// is opaque to this crate — `tmux-rs::utf8` owns the interning table and
/// the conversion helpers (`utf8_from_data`, `utf8_to_data`, `utf8_set`,
/// `utf8_build_one`). This crate just names the handle type so other
/// shared data types (`GridExtdEntry`) can embed it.
pub type Utf8Char = c_uint;

/// Maximum combined-character size. Must fit a base codepoint plus any
/// combining marks that tmux renders as a single cell. Increasing this
/// requires updating the interning scheme on the `tmux-rs` side.
pub const UTF8_SIZE: usize = 21;

/// Expanded UTF-8 character with width metadata. Used wherever a cell
/// needs to carry the *bytes* (not just a handle) — notably in
/// `GridCell.data` for the unpacked-cell representation.
///
/// - `data`: raw UTF-8 bytes; only the first `size` are meaningful.
/// - `have`: bytes filled so far during streaming decode.
/// - `size`: complete byte count once decoding finishes; `0` is a
///   sentinel meaning "no character" (empty cell).
/// - `width`: display columns (0, 1, or 2). `0xff` indicates invalid.
#[derive(Copy, Clone)]
pub struct Utf8Data {
    pub data: [u8; UTF8_SIZE],
    pub have: u8,
    pub size: u8,
    pub width: u8,
}

impl Utf8Data {
    /// Const constructor: pad `data` to `UTF8_SIZE` bytes with zeros so
    /// `Utf8Data` can appear in `static` initializers (e.g. GRID_DEFAULT_CELL).
    /// Panics at compile time if `N >= UTF8_SIZE`.
    pub const fn new<const N: usize>(data: [u8; N], have: u8, size: u8, width: u8) -> Self {
        if N >= UTF8_SIZE {
            panic!("invalid size");
        }

        let mut padded_data = [0u8; UTF8_SIZE];
        let mut i = 0usize;
        while i < N {
            padded_data[i] = data[i];
            i += 1;
        }

        Self { data: padded_data, have, size, width }
    }

    /// Return the valid-byte prefix of `data` — `data[..size]`.
    pub fn initialized_slice(&self) -> &[u8] {
        &self.data[..self.size as usize]
    }
}
