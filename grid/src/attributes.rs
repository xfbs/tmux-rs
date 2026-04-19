// Copyright (c) 2009 Joshua Elsasser <josh@elsasser.org>
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
//! Terminal text attribute parsing and formatting.
//!
//! Converts between [`GridAttr`] bitflags and their string representation.
//! Supports 14 attributes (bright/bold, dim, underscore variants, blink, reverse,
//! hidden, italics, strikethrough, overline, acs/charset).
//!
//! String format uses comma-separated names (e.g. `"bold,italics,underline"`).
//! Delimiters can be commas, spaces, or pipes. Parsing is case-insensitive.
//! `"none"` and `"default"` both map to empty attributes.

use std::borrow::Cow;

use crate::GridAttr;

/// Fuzz-friendly wrapper: parse attributes from a string.
/// Exercises the parsing logic without asserting round-trip stability.
#[cfg(fuzzing)]
pub fn fuzz_attributes(input: &str) {
    let _ = attributes_fromstring(input);
}

/// Converts a [`GridAttr`] bitflag set into a comma-separated string.
/// Returns `"none"` for empty attributes. Trailing comma is included in output
/// for non-empty sets (matches C tmux behavior).
#[rustfmt::skip]
pub fn attributes_tostring(attr: GridAttr) -> Cow<'static, str> {
    if attr.is_empty() {
        return Cow::Borrowed("none");
    }

    Cow::Owned(format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        if attr.intersects(GridAttr::GRID_ATTR_CHARSET) { "acs," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_BRIGHT) { "bright," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_DIM ) { "dim," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_UNDERSCORE) { "underscore," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_BLINK) { "blink," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_REVERSE ) { "reverse," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_HIDDEN) { "hidden," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_ITALICS ) { "italics," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_STRIKETHROUGH) { "strikethrough," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_UNDERSCORE_2) { "double-underscore," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_UNDERSCORE_3) { "curly-underscore," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_UNDERSCORE_4) { "dotted-underscore," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_UNDERSCORE_5) { "dashed-underscore," } else { "" },
        if attr.intersects(GridAttr::GRID_ATTR_OVERLINE) { "overline," } else { "" },
    ))
}

/// Parses a delimiter-separated attribute string into [`GridAttr`] bitflags.
/// Accepts commas, spaces, or pipes as delimiters. Case-insensitive.
/// `"none"` and `"default"` return empty attributes.
/// Returns `Err(())` if the string is empty, starts/ends with a delimiter,
/// or contains an unrecognized attribute name.
/// Note: `"bold"` is an alias for `"bright"`.
pub fn attributes_fromstring(str: &str) -> Result<GridAttr, ()> {
    struct TableEntry {
        name: &'static str,
        attr: GridAttr,
    }

    #[rustfmt::skip]
    const TABLE: [TableEntry; 15] = [
        TableEntry { name: "acs", attr: GridAttr::GRID_ATTR_CHARSET, },
        TableEntry { name: "bright", attr: GridAttr::GRID_ATTR_BRIGHT, },
        TableEntry { name: "bold", attr: GridAttr::GRID_ATTR_BRIGHT, },
        TableEntry { name: "dim", attr: GridAttr::GRID_ATTR_DIM, },
        TableEntry { name: "underscore", attr: GridAttr::GRID_ATTR_UNDERSCORE, },
        TableEntry { name: "blink", attr: GridAttr::GRID_ATTR_BLINK, },
        TableEntry { name: "reverse", attr: GridAttr::GRID_ATTR_REVERSE, },
        TableEntry { name: "hidden", attr: GridAttr::GRID_ATTR_HIDDEN, },
        TableEntry { name: "italics", attr: GridAttr::GRID_ATTR_ITALICS, },
        TableEntry { name: "strikethrough", attr: GridAttr::GRID_ATTR_STRIKETHROUGH, },
        TableEntry { name: "double-underscore", attr: GridAttr::GRID_ATTR_UNDERSCORE_2, },
        TableEntry { name: "curly-underscore", attr: GridAttr::GRID_ATTR_UNDERSCORE_3, },
        TableEntry { name: "dotted-underscore", attr: GridAttr::GRID_ATTR_UNDERSCORE_4, },
        TableEntry { name: "dashed-underscore", attr: GridAttr::GRID_ATTR_UNDERSCORE_5, },
        TableEntry { name: "overline", attr: GridAttr::GRID_ATTR_OVERLINE, },
    ];

    let delimiters = &[' ', ',', '|'];

    if str.is_empty() || str.find(delimiters) == Some(0) {
        return Err(());
    }

    if matches!(str.chars().next_back().unwrap(), ' ' | ',' | '|') {
        return Err(());
    }

    if str.eq_ignore_ascii_case("default") || str.eq_ignore_ascii_case("none") {
        return Ok(GridAttr::empty());
    }

    let mut attr = GridAttr::empty();
    for str in str.split(delimiters) {
        let Some(i) = TABLE.iter().position(|t| str.eq_ignore_ascii_case(t.name)) else {
            return Err(());
        };
        attr |= TABLE[i].attr;
    }

    Ok(attr)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // attributes_tostring
    // ---------------------------------------------------------------

    #[test]
    fn tostring_empty_is_none() {
        assert_eq!(attributes_tostring(GridAttr::empty()).as_ref(), "none");
    }

    #[test]
    fn tostring_single_attributes() {
        // Each single attribute should produce its name followed by a comma.
        let cases = [
            (GridAttr::GRID_ATTR_BRIGHT, "bright,"),
            (GridAttr::GRID_ATTR_DIM, "dim,"),
            (GridAttr::GRID_ATTR_UNDERSCORE, "underscore,"),
            (GridAttr::GRID_ATTR_BLINK, "blink,"),
            (GridAttr::GRID_ATTR_REVERSE, "reverse,"),
            (GridAttr::GRID_ATTR_HIDDEN, "hidden,"),
            (GridAttr::GRID_ATTR_ITALICS, "italics,"),
            (GridAttr::GRID_ATTR_CHARSET, "acs,"),
            (GridAttr::GRID_ATTR_STRIKETHROUGH, "strikethrough,"),
            (GridAttr::GRID_ATTR_UNDERSCORE_2, "double-underscore,"),
            (GridAttr::GRID_ATTR_UNDERSCORE_3, "curly-underscore,"),
            (GridAttr::GRID_ATTR_UNDERSCORE_4, "dotted-underscore,"),
            (GridAttr::GRID_ATTR_UNDERSCORE_5, "dashed-underscore,"),
            (GridAttr::GRID_ATTR_OVERLINE, "overline,"),
        ];
        for (attr, expected) in &cases {
            assert_eq!(
                attributes_tostring(*attr).as_ref(),
                *expected,
                "failed for {expected}"
            );
        }
    }

    #[test]
    fn tostring_multiple_attributes() {
        let attr = GridAttr::GRID_ATTR_BRIGHT | GridAttr::GRID_ATTR_ITALICS;
        let s = attributes_tostring(attr);
        assert!(s.contains("bright,"));
        assert!(s.contains("italics,"));
    }

    // ---------------------------------------------------------------
    // attributes_fromstring — valid inputs
    // ---------------------------------------------------------------

    #[test]
    fn fromstring_none() {
        assert_eq!(attributes_fromstring("none"), Ok(GridAttr::empty()));
        assert_eq!(attributes_fromstring("None"), Ok(GridAttr::empty()));
        assert_eq!(attributes_fromstring("NONE"), Ok(GridAttr::empty()));
    }

    #[test]
    fn fromstring_default() {
        assert_eq!(attributes_fromstring("default"), Ok(GridAttr::empty()));
        assert_eq!(attributes_fromstring("Default"), Ok(GridAttr::empty()));
    }

    #[test]
    fn fromstring_single_attributes() {
        let cases = [
            ("bright", GridAttr::GRID_ATTR_BRIGHT),
            ("bold", GridAttr::GRID_ATTR_BRIGHT), // alias
            ("dim", GridAttr::GRID_ATTR_DIM),
            ("underscore", GridAttr::GRID_ATTR_UNDERSCORE),
            ("blink", GridAttr::GRID_ATTR_BLINK),
            ("reverse", GridAttr::GRID_ATTR_REVERSE),
            ("hidden", GridAttr::GRID_ATTR_HIDDEN),
            ("italics", GridAttr::GRID_ATTR_ITALICS),
            ("acs", GridAttr::GRID_ATTR_CHARSET),
            ("strikethrough", GridAttr::GRID_ATTR_STRIKETHROUGH),
            ("double-underscore", GridAttr::GRID_ATTR_UNDERSCORE_2),
            ("curly-underscore", GridAttr::GRID_ATTR_UNDERSCORE_3),
            ("dotted-underscore", GridAttr::GRID_ATTR_UNDERSCORE_4),
            ("dashed-underscore", GridAttr::GRID_ATTR_UNDERSCORE_5),
            ("overline", GridAttr::GRID_ATTR_OVERLINE),
        ];
        for (name, expected) in &cases {
            assert_eq!(
                attributes_fromstring(name),
                Ok(*expected),
                "failed for {name}"
            );
        }
    }

    #[test]
    fn fromstring_case_insensitive() {
        assert_eq!(
            attributes_fromstring("BOLD"),
            Ok(GridAttr::GRID_ATTR_BRIGHT)
        );
        assert_eq!(
            attributes_fromstring("Italics"),
            Ok(GridAttr::GRID_ATTR_ITALICS)
        );
    }

    #[test]
    fn fromstring_comma_delimiter() {
        let result = attributes_fromstring("bold,italics").unwrap();
        assert!(result.intersects(GridAttr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(GridAttr::GRID_ATTR_ITALICS));
    }

    #[test]
    fn fromstring_space_delimiter() {
        let result = attributes_fromstring("bold italics").unwrap();
        assert!(result.intersects(GridAttr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(GridAttr::GRID_ATTR_ITALICS));
    }

    #[test]
    fn fromstring_pipe_delimiter() {
        let result = attributes_fromstring("bold|italics").unwrap();
        assert!(result.intersects(GridAttr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(GridAttr::GRID_ATTR_ITALICS));
    }

    #[test]
    fn fromstring_three_attributes() {
        let result = attributes_fromstring("bold,dim,blink").unwrap();
        assert!(result.intersects(GridAttr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(GridAttr::GRID_ATTR_DIM));
        assert!(result.intersects(GridAttr::GRID_ATTR_BLINK));
    }

    // ---------------------------------------------------------------
    // attributes_fromstring — invalid inputs
    // ---------------------------------------------------------------

    #[test]
    fn fromstring_empty_is_error() {
        assert_eq!(attributes_fromstring(""), Err(()));
    }

    #[test]
    fn fromstring_leading_delimiter_is_error() {
        assert_eq!(attributes_fromstring(",bold"), Err(()));
        assert_eq!(attributes_fromstring(" bold"), Err(()));
        assert_eq!(attributes_fromstring("|bold"), Err(()));
    }

    #[test]
    fn fromstring_trailing_delimiter_is_error() {
        assert_eq!(attributes_fromstring("bold,"), Err(()));
        assert_eq!(attributes_fromstring("bold "), Err(()));
        assert_eq!(attributes_fromstring("bold|"), Err(()));
    }

    #[test]
    fn fromstring_unknown_attribute_is_error() {
        assert_eq!(attributes_fromstring("foobar"), Err(()));
        assert_eq!(attributes_fromstring("bold,foobar"), Err(()));
    }

    // ---------------------------------------------------------------
    // Round-trip: fromstring → tostring
    // ---------------------------------------------------------------

    #[test]
    fn round_trip_single_attributes() {
        // Each attribute should survive a round-trip (parse then format).
        // Note: "bold" → BRIGHT → "bright," (alias doesn't round-trip to same name).
        let names = [
            "bright", "dim", "underscore", "blink", "reverse", "hidden",
            "italics", "acs", "strikethrough", "double-underscore",
            "curly-underscore", "dotted-underscore", "dashed-underscore", "overline",
        ];
        for name in &names {
            let attr = attributes_fromstring(name).unwrap();
            let s = attributes_tostring(attr);
            // tostring appends a trailing comma for single attributes.
            let trimmed = s.trim_end_matches(',');
            assert_eq!(
                trimmed, *name,
                "round-trip failed for {name}"
            );
        }
    }
}
