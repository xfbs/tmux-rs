// Copyright (c) 2010 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF MIND, USE, DATA OR PROFITS, WHETHER
// IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING
// OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//! ACS (Alternative Character Set) lookup tables and UTF-8 border characters.
//!
//! Maps VT100 ACS drawing characters to their UTF-8 equivalents and back.
//! Used for rendering box-drawing borders in terminals. Three border styles
//! are supported: **double** (═║╔), **heavy** (━┃┏), and **rounded** (─│╭).
//!
//! Forward lookup (`tty_acs_get`): ACS key byte → UTF-8 string via binary
//! search on `TTY_ACS_TABLE`. Falls back to the terminal's own ACS set if
//! the client doesn't support UTF-8.
//!
//! Reverse lookup (`tty_acs_reverse_get`): UTF-8 byte sequence → ACS key,
//! split into 2-byte (`TTY_ACS_REVERSE2`) and 3-byte (`TTY_ACS_REVERSE3`)
//! tables for efficient binary search.

use crate::*;

/// Forward ACS table entry: maps an ACS key byte to a NUL-terminated UTF-8 sequence.
pub struct tty_acs_entry {
    pub key: u8,
    pub string: &'static [u8; 4],
}
impl tty_acs_entry {
    #[expect(clippy::trivially_copy_pass_by_ref, reason = "false positive")]
    pub const fn new(key: u8, string: &'static [u8; 4]) -> Self {
        Self { key, string }
    }
}

static TTY_ACS_TABLE: [tty_acs_entry; 36] = [
    tty_acs_entry::new(b'+', &[0o342, 0o206, 0o222, 0o000]), // arrow pointing right
    tty_acs_entry::new(b',', &[0o342, 0o206, 0o220, 0o000]), // arrow pointing left
    tty_acs_entry::new(b'-', &[0o342, 0o206, 0o221, 0o000]), // arrow pointing up
    tty_acs_entry::new(b'.', &[0o342, 0o206, 0o223, 0o000]), // arrow pointing down
    tty_acs_entry::new(b'0', &[0o342, 0o226, 0o256, 0o000]), // solid square block
    tty_acs_entry::new(b'`', &[0o342, 0o227, 0o206, 0o000]), // diamond
    tty_acs_entry::new(b'a', &[0o342, 0o226, 0o222, 0o000]), // checker board (stipple)
    tty_acs_entry::new(b'b', &[0o342, 0o220, 0o211, 0o000]),
    tty_acs_entry::new(b'c', &[0o342, 0o220, 0o214, 0o000]),
    tty_acs_entry::new(b'd', &[0o342, 0o220, 0o215, 0o000]),
    tty_acs_entry::new(b'e', &[0o342, 0o220, 0o212, 0o000]),
    tty_acs_entry::new(b'f', &[0o302, 0o260, 0o000, 0o000]), // degree symbol
    tty_acs_entry::new(b'g', &[0o302, 0o261, 0o000, 0o000]), // plus/minus
    tty_acs_entry::new(b'h', &[0o342, 0o220, 0o244, 0o000]),
    tty_acs_entry::new(b'i', &[0o342, 0o220, 0o213, 0o000]),
    tty_acs_entry::new(b'j', &[0o342, 0o224, 0o230, 0o000]), // lower right corner
    tty_acs_entry::new(b'k', &[0o342, 0o224, 0o220, 0o000]), // upper right corner
    tty_acs_entry::new(b'l', &[0o342, 0o224, 0o214, 0o000]), // upper left corner
    tty_acs_entry::new(b'm', &[0o342, 0o224, 0o224, 0o000]), // lower left corner
    tty_acs_entry::new(b'n', &[0o342, 0o224, 0o274, 0o000]), // large plus or crossover
    tty_acs_entry::new(b'o', &[0o342, 0o216, 0o272, 0o000]), // scan line 1
    tty_acs_entry::new(b'p', &[0o342, 0o216, 0o273, 0o000]), // scan line 3
    tty_acs_entry::new(b'q', &[0o342, 0o224, 0o200, 0o000]), // horizontal line
    tty_acs_entry::new(b'r', &[0o342, 0o216, 0o274, 0o000]), // scan line 7
    tty_acs_entry::new(b's', &[0o342, 0o216, 0o275, 0o000]), // scan line 9
    tty_acs_entry::new(b't', &[0o342, 0o224, 0o234, 0o000]), // tee pointing right
    tty_acs_entry::new(b'u', &[0o342, 0o224, 0o244, 0o000]), // tee pointing left
    tty_acs_entry::new(b'v', &[0o342, 0o224, 0o264, 0o000]), // tee pointing up
    tty_acs_entry::new(b'w', &[0o342, 0o224, 0o254, 0o000]), // tee pointing down
    tty_acs_entry::new(b'x', &[0o342, 0o224, 0o202, 0o000]), // vertical line
    tty_acs_entry::new(b'y', &[0o342, 0o211, 0o244, 0o000]), // less-than-or-equal-to
    tty_acs_entry::new(b'z', &[0o342, 0o211, 0o245, 0o000]), // greater-than-or-equal-to
    tty_acs_entry::new(b'{', &[0o317, 0o200, 0o000, 0o000]), // greek pi
    tty_acs_entry::new(b'|', &[0o342, 0o211, 0o240, 0o000]), // not-equal
    tty_acs_entry::new(b'}', &[0o302, 0o243, 0o000, 0o000]), // UK pound sign
    tty_acs_entry::new(b'~', &[0o302, 0o267, 0o000, 0o000]), // bullet
];

/// Reverse ACS table entry: maps a UTF-8 byte sequence back to its ACS key byte.
pub struct tty_acs_reverse_entry {
    pub string: &'static [u8; 4],
    pub key: u8,
}
impl tty_acs_reverse_entry {
    #[expect(clippy::trivially_copy_pass_by_ref, reason = "false positive")]
    const fn new(string: &'static [u8; 4], key: u8) -> Self {
        Self { string, key }
    }
}

static TTY_ACS_REVERSE2: [tty_acs_reverse_entry; 1] = [tty_acs_reverse_entry::new(
    &[0o302, 0o267, 0o000, 0o000],
    b'~',
)];

static TTY_ACS_REVERSE3: [tty_acs_reverse_entry; 32] = [
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o200, 0o000], b'q'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o201, 0o000], b'q'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o202, 0o000], b'x'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o203, 0o000], b'x'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o214, 0o000], b'l'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o217, 0o000], b'k'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o220, 0o000], b'k'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o223, 0o000], b'l'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o224, 0o000], b'm'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o227, 0o000], b'm'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o230, 0o000], b'j'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o233, 0o000], b'j'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o234, 0o000], b't'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o243, 0o000], b't'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o244, 0o000], b'u'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o253, 0o000], b'u'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o263, 0o000], b'w'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o264, 0o000], b'v'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o273, 0o000], b'v'),
    tty_acs_reverse_entry::new(&[0o342, 0o224, 0o274, 0o000], b'n'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o213, 0o000], b'n'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o220, 0o000], b'q'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o221, 0o000], b'x'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o224, 0o000], b'l'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o227, 0o000], b'k'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o232, 0o000], b'm'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o235, 0o000], b'j'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o240, 0o000], b't'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o243, 0o000], b'u'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o246, 0o000], b'w'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o251, 0o000], b'v'),
    tty_acs_reverse_entry::new(&[0o342, 0o225, 0o254, 0o000], b'n'),
];

/// UTF-8 double borders.
static TTY_ACS_DOUBLE_BORDERS_LIST: [Utf8Data; 13] = [
    Utf8Data::new([0o000, 0o000, 0o000, 0o000], 0, 0, 0),
    Utf8Data::new([0o342, 0o225, 0o221, 0o000], 0, 3, 1), // U+2551
    Utf8Data::new([0o342, 0o225, 0o220, 0o000], 0, 3, 1), // U+2550
    Utf8Data::new([0o342, 0o225, 0o224, 0o000], 0, 3, 1), // U+2554
    Utf8Data::new([0o342, 0o225, 0o227, 0o000], 0, 3, 1), // U+2557
    Utf8Data::new([0o342, 0o225, 0o232, 0o000], 0, 3, 1), // U+255A
    Utf8Data::new([0o342, 0o225, 0o235, 0o000], 0, 3, 1), // U+255D
    Utf8Data::new([0o342, 0o225, 0o246, 0o000], 0, 3, 1), // U+2566
    Utf8Data::new([0o342, 0o225, 0o251, 0o000], 0, 3, 1), // U+2569
    Utf8Data::new([0o342, 0o225, 0o240, 0o000], 0, 3, 1), // U+2560
    Utf8Data::new([0o342, 0o225, 0o243, 0o000], 0, 3, 1), // U+2563
    Utf8Data::new([0o342, 0o225, 0o254, 0o000], 0, 3, 1), // U+256C
    Utf8Data::new([0o302, 0o267, 0o000, 0o000], 0, 2, 1), // U+00B7
];

/// UTF-8 heavy borders.
static TTY_ACS_HEAVY_BORDERS_LIST: [Utf8Data; 13] = [
    Utf8Data::new([0o000, 0o000, 0o000, 0o000], 0, 0, 0),
    Utf8Data::new([0o342, 0o224, 0o203, 0o000], 0, 3, 1), // U+2503
    Utf8Data::new([0o342, 0o224, 0o201, 0o000], 0, 3, 1), // U+2501
    Utf8Data::new([0o342, 0o224, 0o217, 0o000], 0, 3, 1), // U+250F
    Utf8Data::new([0o342, 0o224, 0o223, 0o000], 0, 3, 1), // U+2513
    Utf8Data::new([0o342, 0o224, 0o227, 0o000], 0, 3, 1), // U+2517
    Utf8Data::new([0o342, 0o224, 0o233, 0o000], 0, 3, 1), // U+251B
    Utf8Data::new([0o342, 0o224, 0o263, 0o000], 0, 3, 1), // U+2533
    Utf8Data::new([0o342, 0o224, 0o273, 0o000], 0, 3, 1), // U+253B
    Utf8Data::new([0o342, 0o224, 0o243, 0o000], 0, 3, 1), // U+2523
    Utf8Data::new([0o342, 0o224, 0o253, 0o000], 0, 3, 1), // U+252B
    Utf8Data::new([0o342, 0o225, 0o213, 0o000], 0, 3, 1), // U+254B
    Utf8Data::new([0o302, 0o267, 0o000, 0o000], 0, 2, 1), // U+00B7
];

/// UTF-8 rounded borders.
static TTY_ACS_ROUNDED_BORDERS_LIST: [Utf8Data; 13] = [
    Utf8Data::new([0o000, 0o000, 0o000, 0o000], 0, 0, 0),
    Utf8Data::new([0o342, 0o224, 0o202, 0o000], 0, 3, 1), // U+2502
    Utf8Data::new([0o342, 0o224, 0o200, 0o000], 0, 3, 1), // U+2500
    Utf8Data::new([0o342, 0o225, 0o255, 0o000], 0, 3, 1), // U+256D
    Utf8Data::new([0o342, 0o225, 0o256, 0o000], 0, 3, 1), // U+256E
    Utf8Data::new([0o342, 0o225, 0o260, 0o000], 0, 3, 1), // U+2570
    Utf8Data::new([0o342, 0o225, 0o257, 0o000], 0, 3, 1), // U+256F
    Utf8Data::new([0o342, 0o224, 0o263, 0o000], 0, 3, 1), // U+2533
    Utf8Data::new([0o342, 0o224, 0o273, 0o000], 0, 3, 1), // U+253B
    Utf8Data::new([0o342, 0o224, 0o234, 0o000], 0, 3, 1), // U+2524
    Utf8Data::new([0o342, 0o224, 0o244, 0o000], 0, 3, 1), // U+251C
    Utf8Data::new([0o342, 0o225, 0o213, 0o000], 0, 3, 1), // U+254B
    Utf8Data::new([0o302, 0o267, 0o000, 0o000], 0, 2, 1), // U+00B7
];

/// Get cell border character for double-line style (═║╔╗╚╝).
pub fn tty_acs_double_borders(cell_type: cell_type) -> &'static Utf8Data {
    &TTY_ACS_DOUBLE_BORDERS_LIST[cell_type as usize]
}

/// Get cell border character for heavy/thick style (━┃┏┓┗┛).
pub fn tty_acs_heavy_borders(cell_type: cell_type) -> &'static Utf8Data {
    &TTY_ACS_HEAVY_BORDERS_LIST[cell_type as usize]
}

/// Get cell border character for rounded style.
pub fn tty_acs_rounded_borders(cell_type: cell_type) -> &'static Utf8Data {
    &TTY_ACS_ROUNDED_BORDERS_LIST[cell_type as usize]
}

/// Compare an ACS key byte against a table entry (for binary search).
pub fn tty_acs_cmp(test: u8, entry: &tty_acs_entry) -> std::cmp::Ordering {
    test.cmp(&entry.key)
}

/// Compare a UTF-8 byte sequence against a reverse table entry (for binary search).
pub unsafe fn tty_acs_reverse_cmp(
    key: *const u8,
    entry: *const tty_acs_reverse_entry,
) -> std::cmp::Ordering {
    unsafe { i32_to_ordering(libc::strcmp(key, (*entry).string.as_ptr().cast())) }
}

/// Should this terminal use ACS instead of UTF-8 line drawing?
pub unsafe fn tty_acs_needed(tty: *const tty) -> bool {
    unsafe {
        if tty.is_null() {
            return false;
        }

        if tty_term_has((*tty).term, tty_code_code::TTYC_U8)
            && tty_term_number((*tty).term, tty_code_code::TTYC_U8) == 0
        {
            return true;
        }

        if (*(*tty).client).flags.intersects(client_flag::UTF8) {
            return false;
        }
        true
    }
}

/// Retrieve ACS to output as UTF-8.
pub unsafe fn tty_acs_get(tty: *mut tty, ch: u8) -> *const u8 {
    unsafe {
        // Use the ACS set instead of UTF-8 if needed.
        if tty_acs_needed(tty) {
            if (*(*tty).term).acs[ch as usize][0] == b'\0' {
                return null();
            }
            return &raw const (*(*tty).term).acs[ch as usize][0];
        }

        let Ok(entry) = TTY_ACS_TABLE.binary_search_by(|e| tty_acs_cmp(ch, e).reverse()) else {
            return null_mut();
        };

        TTY_ACS_TABLE[entry].string.as_ptr().cast()
    }
}

/// Reverse UTF-8 into ACS.
/// Looks up a UTF-8 byte sequence and returns the corresponding ACS key byte,
/// or -1 if not found. The `_tty` parameter is currently unused.
pub unsafe fn tty_acs_reverse_get(_tty: *const tty, s: *const u8, slen: usize) -> i32 {
    unsafe {
        let table = if slen == 2 {
            TTY_ACS_REVERSE2.as_slice()
        } else if slen == 3 {
            TTY_ACS_REVERSE3.as_slice()
        } else {
            return -1;
        };
        let Ok(entry) = table.binary_search_by(|e| tty_acs_reverse_cmp(s, e).reverse()) else {
            return -1;
        };
        table[entry].key as _
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Helper: convert a NUL-terminated byte array to a UTF-8 str.
    fn acs_to_str(bytes: &[u8; 4]) -> &str {
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(4);
        std::str::from_utf8(&bytes[..len]).unwrap()
    }

    // ---------------------------------------------------------------
    // TTY_ACS_TABLE — table integrity
    // ---------------------------------------------------------------

    #[test]
    fn acs_table_sorted_by_key() {
        // Binary search requires the table to be sorted by key.
        for pair in TTY_ACS_TABLE.windows(2) {
            assert!(
                pair[0].key < pair[1].key,
                "TTY_ACS_TABLE not sorted: '{}' >= '{}'",
                pair[0].key as char,
                pair[1].key as char,
            );
        }
    }

    #[test]
    fn acs_table_entries_are_valid_utf8() {
        for entry in &TTY_ACS_TABLE {
            let s = acs_to_str(entry.string);
            assert!(
                !s.is_empty(),
                "empty UTF-8 for ACS key '{}'",
                entry.key as char
            );
        }
    }

    #[test]
    fn acs_table_entries_nul_terminated() {
        for entry in &TTY_ACS_TABLE {
            // Last byte should be NUL (the tables are [u8; 4] with NUL terminator).
            assert!(
                entry.string.contains(&0),
                "ACS entry '{}' missing NUL terminator",
                entry.key as char
            );
        }
    }

    // ---------------------------------------------------------------
    // tty_acs_cmp
    // ---------------------------------------------------------------

    #[test]
    fn cmp_equal() {
        let entry = &TTY_ACS_TABLE[0]; // key = '+'
        assert_eq!(tty_acs_cmp(b'+', entry), std::cmp::Ordering::Equal);
    }

    #[test]
    fn cmp_less() {
        let entry = &TTY_ACS_TABLE[0]; // key = '+'
        assert_eq!(tty_acs_cmp(b'!', entry), std::cmp::Ordering::Less);
    }

    #[test]
    fn cmp_greater() {
        let entry = &TTY_ACS_TABLE[0]; // key = '+'
        assert_eq!(tty_acs_cmp(b'z', entry), std::cmp::Ordering::Greater);
    }

    // ---------------------------------------------------------------
    // Forward lookup via binary search on TTY_ACS_TABLE
    // ---------------------------------------------------------------

    #[test]
    fn forward_lookup_known_keys() {
        // Spot-check some well-known ACS characters.
        let cases: &[(u8, &str)] = &[
            (b'q', "─"),  // horizontal line U+2500
            (b'x', "│"),  // vertical line U+2502
            (b'l', "┌"),  // upper left corner U+250C
            (b'k', "┐"),  // upper right corner U+2510
            (b'm', "└"),  // lower left corner U+2514
            (b'j', "┘"),  // lower right corner U+2518
            (b'n', "┼"),  // crossover U+253C
            (b'~', "·"),  // bullet U+00B7
        ];
        for &(key, expected) in cases {
            let result = TTY_ACS_TABLE
                .binary_search_by(|e| tty_acs_cmp(key, e).reverse())
                .map(|i| acs_to_str(TTY_ACS_TABLE[i].string));
            assert_eq!(
                result,
                Ok(expected),
                "forward lookup failed for ACS key '{}'",
                key as char
            );
        }
    }

    #[test]
    fn forward_lookup_not_found() {
        // 'A' is not an ACS key.
        let result = TTY_ACS_TABLE.binary_search_by(|e| tty_acs_cmp(b'A', e).reverse());
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Reverse lookup: tty_acs_reverse_get
    // ---------------------------------------------------------------

    #[test]
    fn reverse_lookup_3byte() {
        unsafe {
            // horizontal line "─" = [0o342, 0o224, 0o200] → 'q'
            let s: [u8; 4] = [0o342, 0o224, 0o200, 0];
            let result = tty_acs_reverse_get(null(), s.as_ptr(), 3);
            assert_eq!(result, b'q' as i32);
        }
    }

    #[test]
    fn reverse_lookup_2byte() {
        unsafe {
            // bullet "·" = [0o302, 0o267] → '~'
            let s: [u8; 3] = [0o302, 0o267, 0];
            let result = tty_acs_reverse_get(null(), s.as_ptr(), 2);
            assert_eq!(result, b'~' as i32);
        }
    }

    #[test]
    fn reverse_lookup_not_found() {
        unsafe {
            let s: [u8; 4] = [0xFF, 0xFF, 0xFF, 0];
            assert_eq!(tty_acs_reverse_get(null(), s.as_ptr(), 3), -1);
        }
    }

    #[test]
    fn reverse_lookup_bad_length() {
        unsafe {
            let s: [u8; 2] = [0o342, 0];
            assert_eq!(tty_acs_reverse_get(null(), s.as_ptr(), 1), -1);
            assert_eq!(tty_acs_reverse_get(null(), s.as_ptr(), 4), -1);
        }
    }

    // ---------------------------------------------------------------
    // Forward ↔ Reverse round-trip
    // ---------------------------------------------------------------

    #[test]
    fn round_trip_forward_then_reverse() {
        unsafe {
            // For each entry in the forward table, look it up in reverse.
            for entry in &TTY_ACS_TABLE {
                let len = entry.string.iter().position(|&b| b == 0).unwrap_or(4);
                let result = tty_acs_reverse_get(null(), entry.string.as_ptr(), len);
                // Not all forward entries have reverse mappings (the reverse tables
                // are subsets). If found, it should map back to the same key.
                if result != -1 {
                    assert_eq!(
                        result as u8, entry.key,
                        "round-trip mismatch for ACS key '{}'",
                        entry.key as char
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // Border style lookups
    // ---------------------------------------------------------------

    #[test]
    fn double_borders_all_cell_types() {
        // Verify all cell types return valid entries (no panics from out-of-bounds).
        for i in 0..13u8 {
            let ct: cell_type = unsafe { std::mem::transmute(i as u32) };
            let ud = tty_acs_double_borders(ct);
            // CELL_INSIDE has size 0 (empty), all others should have content.
            if i == 0 {
                assert_eq!(ud.size, 0);
            } else {
                assert!(ud.size > 0, "double border empty for cell_type {i}");
            }
        }
    }

    #[test]
    fn heavy_borders_all_cell_types() {
        for i in 0..13u8 {
            let ct: cell_type = unsafe { std::mem::transmute(i as u32) };
            let ud = tty_acs_heavy_borders(ct);
            if i == 0 {
                assert_eq!(ud.size, 0);
            } else {
                assert!(ud.size > 0, "heavy border empty for cell_type {i}");
            }
        }
    }

    #[test]
    fn rounded_borders_all_cell_types() {
        for i in 0..13u8 {
            let ct: cell_type = unsafe { std::mem::transmute(i as u32) };
            let ud = tty_acs_rounded_borders(ct);
            if i == 0 {
                assert_eq!(ud.size, 0);
            } else {
                assert!(ud.size > 0, "rounded border empty for cell_type {i}");
            }
        }
    }

    #[test]
    fn double_borders_known_chars() {
        let ud = tty_acs_double_borders(cell_type::CELL_TOPLEFT);
        let s = std::str::from_utf8(&ud.data[..ud.size as usize]).unwrap();
        assert_eq!(s, "╔"); // U+2554
    }

    #[test]
    fn heavy_borders_known_chars() {
        let ud = tty_acs_heavy_borders(cell_type::CELL_TOPLEFT);
        let s = std::str::from_utf8(&ud.data[..ud.size as usize]).unwrap();
        assert_eq!(s, "┏"); // U+250F
    }

    #[test]
    fn rounded_borders_known_chars() {
        let ud = tty_acs_rounded_borders(cell_type::CELL_TOPLEFT);
        let s = std::str::from_utf8(&ud.data[..ud.size as usize]).unwrap();
        assert_eq!(s, "╭"); // U+256D
    }

    // ---------------------------------------------------------------
    // Reverse tables — sorted (binary search precondition)
    // ---------------------------------------------------------------

    #[test]
    fn reverse2_table_sorted() {
        for pair in TTY_ACS_REVERSE2.windows(2) {
            assert!(
                pair[0].string < pair[1].string,
                "TTY_ACS_REVERSE2 not sorted"
            );
        }
    }

    #[test]
    fn reverse3_table_sorted() {
        for pair in TTY_ACS_REVERSE3.windows(2) {
            assert!(
                pair[0].string < pair[1].string,
                "TTY_ACS_REVERSE3 not sorted"
            );
        }
    }
}
