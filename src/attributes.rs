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
//! Converts between [`grid_attr`] bitflags and their string representation.
//! Supports 14 attributes (bright/bold, dim, underscore variants, blink, reverse,
//! hidden, italics, strikethrough, overline, acs/charset).
//!
//! String format uses comma-separated names (e.g. `"bold,italics,underline"`).
//! Delimiters can be commas, spaces, or pipes. Parsing is case-insensitive.
//! `"none"` and `"default"` both map to empty attributes.

use std::borrow::Cow;

use crate::grid_attr;

/// Converts a [`grid_attr`] bitflag set into a comma-separated string.
/// Returns `"none"` for empty attributes. Trailing comma is included in output
/// for non-empty sets (matches C tmux behavior).
#[rustfmt::skip]
pub fn attributes_tostring(attr: grid_attr) -> Cow<'static, str> {
    if attr.is_empty() {
        return Cow::Borrowed("none");
    }

    Cow::Owned(format!(
        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
        if attr.intersects(grid_attr::GRID_ATTR_CHARSET) { "acs," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_BRIGHT) { "bright," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_DIM ) { "dim," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE) { "underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_BLINK) { "blink," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_REVERSE ) { "reverse," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_HIDDEN) { "hidden," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_ITALICS ) { "italics," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_STRIKETHROUGH) { "strikethrough," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_2) { "double-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_3) { "curly-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_4) { "dotted-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_UNDERSCORE_5) { "dashed-underscore," } else { "" },
        if attr.intersects(grid_attr::GRID_ATTR_OVERLINE) { "overline," } else { "" },
    ))
}

/// Parses a delimiter-separated attribute string into [`grid_attr`] bitflags.
/// Accepts commas, spaces, or pipes as delimiters. Case-insensitive.
/// `"none"` and `"default"` return empty attributes.
/// Returns `Err(())` if the string is empty, starts/ends with a delimiter,
/// or contains an unrecognized attribute name.
/// Note: `"bold"` is an alias for `"bright"`.
pub fn attributes_fromstring(str: &str) -> Result<grid_attr, ()> {
    struct table_entry {
        name: &'static str,
        attr: grid_attr,
    }

    #[rustfmt::skip]
    const TABLE: [table_entry; 15] = [
        table_entry { name: "acs", attr: grid_attr::GRID_ATTR_CHARSET, },
        table_entry { name: "bright", attr: grid_attr::GRID_ATTR_BRIGHT, },
        table_entry { name: "bold", attr: grid_attr::GRID_ATTR_BRIGHT, },
        table_entry { name: "dim", attr: grid_attr::GRID_ATTR_DIM, },
        table_entry { name: "underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE, },
        table_entry { name: "blink", attr: grid_attr::GRID_ATTR_BLINK, },
        table_entry { name: "reverse", attr: grid_attr::GRID_ATTR_REVERSE, },
        table_entry { name: "hidden", attr: grid_attr::GRID_ATTR_HIDDEN, },
        table_entry { name: "italics", attr: grid_attr::GRID_ATTR_ITALICS, },
        table_entry { name: "strikethrough", attr: grid_attr::GRID_ATTR_STRIKETHROUGH, },
        table_entry { name: "double-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_2, },
        table_entry { name: "curly-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_3, },
        table_entry { name: "dotted-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_4, },
        table_entry { name: "dashed-underscore", attr: grid_attr::GRID_ATTR_UNDERSCORE_5, },
        table_entry { name: "overline", attr: grid_attr::GRID_ATTR_OVERLINE, },
    ];

    let delimiters = &[' ', ',', '|'];

    if str.is_empty() || str.find(delimiters) == Some(0) {
        return Err(());
    }

    if matches!(str.chars().next_back().unwrap(), ' ' | ',' | '|') {
        return Err(());
    }

    if str.eq_ignore_ascii_case("default") || str.eq_ignore_ascii_case("none") {
        return Ok(grid_attr::empty());
    }

    let mut attr = grid_attr::empty();
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
        assert_eq!(attributes_tostring(grid_attr::empty()).as_ref(), "none");
    }

    #[test]
    fn tostring_single_attributes() {
        // Each single attribute should produce its name followed by a comma.
        let cases = [
            (grid_attr::GRID_ATTR_BRIGHT, "bright,"),
            (grid_attr::GRID_ATTR_DIM, "dim,"),
            (grid_attr::GRID_ATTR_UNDERSCORE, "underscore,"),
            (grid_attr::GRID_ATTR_BLINK, "blink,"),
            (grid_attr::GRID_ATTR_REVERSE, "reverse,"),
            (grid_attr::GRID_ATTR_HIDDEN, "hidden,"),
            (grid_attr::GRID_ATTR_ITALICS, "italics,"),
            (grid_attr::GRID_ATTR_CHARSET, "acs,"),
            (grid_attr::GRID_ATTR_STRIKETHROUGH, "strikethrough,"),
            (grid_attr::GRID_ATTR_UNDERSCORE_2, "double-underscore,"),
            (grid_attr::GRID_ATTR_UNDERSCORE_3, "curly-underscore,"),
            (grid_attr::GRID_ATTR_UNDERSCORE_4, "dotted-underscore,"),
            (grid_attr::GRID_ATTR_UNDERSCORE_5, "dashed-underscore,"),
            (grid_attr::GRID_ATTR_OVERLINE, "overline,"),
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
        let attr = grid_attr::GRID_ATTR_BRIGHT | grid_attr::GRID_ATTR_ITALICS;
        let s = attributes_tostring(attr);
        assert!(s.contains("bright,"));
        assert!(s.contains("italics,"));
    }

    // ---------------------------------------------------------------
    // attributes_fromstring — valid inputs
    // ---------------------------------------------------------------

    #[test]
    fn fromstring_none() {
        assert_eq!(attributes_fromstring("none"), Ok(grid_attr::empty()));
        assert_eq!(attributes_fromstring("None"), Ok(grid_attr::empty()));
        assert_eq!(attributes_fromstring("NONE"), Ok(grid_attr::empty()));
    }

    #[test]
    fn fromstring_default() {
        assert_eq!(attributes_fromstring("default"), Ok(grid_attr::empty()));
        assert_eq!(attributes_fromstring("Default"), Ok(grid_attr::empty()));
    }

    #[test]
    fn fromstring_single_attributes() {
        let cases = [
            ("bright", grid_attr::GRID_ATTR_BRIGHT),
            ("bold", grid_attr::GRID_ATTR_BRIGHT), // alias
            ("dim", grid_attr::GRID_ATTR_DIM),
            ("underscore", grid_attr::GRID_ATTR_UNDERSCORE),
            ("blink", grid_attr::GRID_ATTR_BLINK),
            ("reverse", grid_attr::GRID_ATTR_REVERSE),
            ("hidden", grid_attr::GRID_ATTR_HIDDEN),
            ("italics", grid_attr::GRID_ATTR_ITALICS),
            ("acs", grid_attr::GRID_ATTR_CHARSET),
            ("strikethrough", grid_attr::GRID_ATTR_STRIKETHROUGH),
            ("double-underscore", grid_attr::GRID_ATTR_UNDERSCORE_2),
            ("curly-underscore", grid_attr::GRID_ATTR_UNDERSCORE_3),
            ("dotted-underscore", grid_attr::GRID_ATTR_UNDERSCORE_4),
            ("dashed-underscore", grid_attr::GRID_ATTR_UNDERSCORE_5),
            ("overline", grid_attr::GRID_ATTR_OVERLINE),
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
            Ok(grid_attr::GRID_ATTR_BRIGHT)
        );
        assert_eq!(
            attributes_fromstring("Italics"),
            Ok(grid_attr::GRID_ATTR_ITALICS)
        );
    }

    #[test]
    fn fromstring_comma_delimiter() {
        let result = attributes_fromstring("bold,italics").unwrap();
        assert!(result.intersects(grid_attr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(grid_attr::GRID_ATTR_ITALICS));
    }

    #[test]
    fn fromstring_space_delimiter() {
        let result = attributes_fromstring("bold italics").unwrap();
        assert!(result.intersects(grid_attr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(grid_attr::GRID_ATTR_ITALICS));
    }

    #[test]
    fn fromstring_pipe_delimiter() {
        let result = attributes_fromstring("bold|italics").unwrap();
        assert!(result.intersects(grid_attr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(grid_attr::GRID_ATTR_ITALICS));
    }

    #[test]
    fn fromstring_three_attributes() {
        let result = attributes_fromstring("bold,dim,blink").unwrap();
        assert!(result.intersects(grid_attr::GRID_ATTR_BRIGHT));
        assert!(result.intersects(grid_attr::GRID_ATTR_DIM));
        assert!(result.intersects(grid_attr::GRID_ATTR_BLINK));
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
