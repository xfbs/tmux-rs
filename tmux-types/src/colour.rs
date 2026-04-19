//! Colour constants and tiny pure helpers shared across subsystems.
//!
//! Colours in tmux are encoded in a single `i32`: the low 24 bits hold the
//! value, and one of two high flag bits distinguishes a 256-colour palette
//! index (`COLOUR_FLAG_256`) from a full RGB triplet (`COLOUR_FLAG_RGB`).
//! The unflagged 0..=7 range holds basic ANSI colours, and 90..=97 the
//! bright-ANSI variants.
//!
//! Only the primitives that multiple subsystems need (splitting an RGB
//! value, testing "is this the palette-default colour?") live here. The
//! broader conversion functions (`colour_find_rgb`, `colour_tostring`,
//! parsing, options integration) remain in `tmux-rs::colour` for now —
//! they have external dependencies.

/// High-bit tag: this colour is an 8-bit 256-palette index (low byte).
pub const COLOUR_FLAG_256: i32 = 0x01000000;

/// High-bit tag: this colour is an RGB triplet packed in the low 24 bits
/// as `(r << 16) | (g << 8) | b`.
pub const COLOUR_FLAG_RGB: i32 = 0x02000000;

/// Return `true` if `c` represents the terminal's *default* foreground or
/// background colour (ANSI "default-fg" 9 / "default-bg" 9, or the tmux
/// internal sentinel 8 for "no colour change"). Used by renderers to
/// decide whether an SGR reset is needed.
#[expect(non_snake_case)]
#[inline]
pub fn COLOUR_DEFAULT(c: i32) -> bool {
    c == 8 || c == 9
}

/// Extract the (r, g, b) bytes from a packed RGB colour. Does **not**
/// check the `COLOUR_FLAG_RGB` tag — callers that don't know the colour
/// form must check first.
#[inline]
pub fn colour_split_rgb(c: i32) -> (u8, u8, u8) {
    (
        ((c >> 16) & 0xff) as u8,
        ((c >> 8) & 0xff) as u8,
        (c & 0xff) as u8,
    )
}

/// Pack an RGB triplet into the `i32` colour encoding and set the
/// `COLOUR_FLAG_RGB` tag. Inverse of [`colour_split_rgb`].
#[inline]
pub fn colour_join_rgb(r: u8, g: u8, b: u8) -> i32 {
    (((r as i32) << 16) | ((g as i32) << 8) | (b as i32)) | COLOUR_FLAG_RGB
}
