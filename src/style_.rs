// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2014 Tiago Cunha <tcunha@users.sourceforge.net>
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
//! Style parsing and formatting for tmux status line and option values.
//!
//! A style is a combination of:
//! - Terminal attributes (bold, dim, italics, etc.) from [`GridAttr`]
//! - Foreground/background/underscore colors (`fg=`, `bg=`, `us=`)
//! - Fill color (`fill=`)
//! - Alignment (`align=left|centre|right|absolute-centre`)
//! - List mode (`list=on|focus|left-marker|right-marker`)
//! - Range type (`range=left|right|pane|window|session|user`)
//! - Default push/pop (`push-default`, `pop-default`)
//! - Ignore flag (`ignore`, `noignore`)
//!
//! Styles are parsed from comma/space/newline-delimited strings by [`style_parse`]
//! and formatted back to strings by [`style_tostring`]. The `"default"` keyword
//! resets colors and attributes to the base Grid cell.
//!
//! Color value 8 means "default" (terminal default color).

use crate::libc::{snprintf, strchr, strcspn, strncasecmp, strspn};
use crate::*;
use crate::options_::*;

// #define STYLE_ATTR_MASK (~0)

pub static mut STYLE_DEFAULT: style = style {
    gc: GridCell::new(
        Utf8Data::new([b' '], 0, 1, 1),
        GridAttr::empty(),
        GridFlag::empty(),
        8,
        8,
        0,
        0,
    ),
    ignore: 0,

    fill: 8,
    align: style_align::STYLE_ALIGN_DEFAULT,
    list: style_list::STYLE_LIST_OFF,

    range_type: style_range_type::STYLE_RANGE_NONE,
    range_argument: 0,
    range_string: [0; 16], // ""

    default_type: style_default_type::STYLE_DEFAULT_BASE,
};

/// Fuzz-friendly wrapper: parses a NUL-terminated byte slice as a style string.
/// Returns 0 on success, -1 on error. Encapsulates private types so fuzz targets
/// don't need to import `style` or `GridCell`.
#[cfg(fuzzing)]
pub fn fuzz_style_parse(input: &[u8]) -> i32 {
    unsafe {
        let mut sy = *(&raw const STYLE_DEFAULT);
        style_parse(&raw mut sy, &raw const grid_::GRID_DEFAULT_CELL, input.as_ptr())
    }
}

pub unsafe fn style_set_range_string(sy: *mut style, s: *const u8) {
    unsafe {
        strlcpy(&raw mut (*sy).range_string as _, s, 16); // TODO use better sizeof
    }
}

/// Parse a style string into a [`style`] struct.
/// The `base` Grid cell provides default values for the `"default"` keyword.
/// Returns 0 on success, -1 on parse error (style is restored to its prior state).
/// Delimiters are spaces, commas, and newlines.
pub unsafe fn style_parse(sy: *mut style, base: *const GridCell, mut in_: *const u8) -> i32 {
    unsafe {
        let delimiters = c!(" ,\n");

        type tmp_type = [u8; 256];
        let mut tmp_bak: tmp_type = [0; 256];
        let tmp = tmp_bak.as_mut_ptr();

        let mut found: *mut u8;
        let mut end: usize;

        if *in_ == b'\0' {
            return 0;
        }

        let mut saved = MaybeUninit::<style>::uninit();
        style_copy(saved.as_mut_ptr(), sy);
        let saved = saved.assume_init();

        'error: {
            log_debug!("{}: {}", "style_parse", _s(in_));
            loop {
                while *in_ != b'\0' && !strchr(delimiters, *in_ as _).is_null() {
                    in_ = in_.add(1);
                }
                if *in_ == b'\0' {
                    break;
                }

                end = strcspn(in_, delimiters);
                if end > size_of::<tmp_type>() - 1 {
                    break 'error;
                }
                memcpy_(tmp, in_, end);
                *tmp.add(end) = b'\0' as _;

                log_debug!("{}: {}", "style_parse", _s(tmp));
                if strcaseeq_(tmp, "default") {
                    (*sy).gc.fg = (*base).fg;
                    (*sy).gc.bg = (*base).bg;
                    (*sy).gc.us = (*base).us;
                    (*sy).gc.attr = (*base).attr;
                    (*sy).gc.flags = (*base).flags;
                } else if strcaseeq_(tmp, "ignore") {
                    (*sy).ignore = 1;
                } else if strcaseeq_(tmp, "noignore") {
                    (*sy).ignore = 0;
                } else if strcaseeq_(tmp, "push-default") {
                    (*sy).default_type = style_default_type::STYLE_DEFAULT_PUSH;
                } else if strcaseeq_(tmp, "pop-default") {
                    (*sy).default_type = style_default_type::STYLE_DEFAULT_POP;
                } else if strcaseeq_(tmp, "nolist") {
                    (*sy).list = style_list::STYLE_LIST_OFF;
                } else if strncasecmp(tmp, c!("list="), 5) == 0 {
                    if strcaseeq_(tmp.add(5), "on") {
                        (*sy).list = style_list::STYLE_LIST_ON;
                    } else if strcaseeq_(tmp.add(5), "focus") {
                        (*sy).list = style_list::STYLE_LIST_FOCUS;
                    } else if strcaseeq_(tmp.add(5), "left-marker") {
                        (*sy).list = style_list::STYLE_LIST_LEFT_MARKER;
                    } else if strcaseeq_(tmp.add(5), "right-marker") {
                        (*sy).list = style_list::STYLE_LIST_RIGHT_MARKER;
                    } else {
                        break 'error;
                    }
                } else if strcaseeq_(tmp, "norange") {
                    (*sy).range_type = STYLE_DEFAULT.range_type;
                    (*sy).range_argument = STYLE_DEFAULT.range_type as u32;
                    strlcpy(
                        &raw mut (*sy).range_string as *mut u8,
                        &raw const STYLE_DEFAULT.range_string as *const u8,
                        16,
                    );
                } else if end > 6 && strncasecmp(tmp, c!("range="), 6) == 0 {
                    found = strchr(tmp.add(6), b'|' as i32);
                    if !found.is_null() {
                        *found = b'\0' as _;
                        found = found.add(1);
                        if *found == b'\0' {
                            break 'error;
                        }
                    }
                    if strcaseeq_(tmp.add(6), "left") {
                        if !found.is_null() {
                            break 'error;
                        }
                        (*sy).range_type = style_range_type::STYLE_RANGE_LEFT;
                        (*sy).range_argument = 0;
                        style_set_range_string(sy, c!(""));
                    } else if strcaseeq_(tmp.add(6), "right") {
                        if !found.is_null() {
                            break 'error;
                        }
                        (*sy).range_type = style_range_type::STYLE_RANGE_RIGHT;
                        (*sy).range_argument = 0;
                        style_set_range_string(sy, c!(""));
                    } else if strcaseeq_(tmp.add(6), "pane") {
                        if found.is_null() {
                            break 'error;
                        }
                        if *found != b'%' || *found.add(1) == b'\0' {
                            break 'error;
                        }
                        let Ok(n) = strtonum(found.add(1), 0, u32::MAX) else {
                            break 'error;
                        };
                        (*sy).range_type = style_range_type::STYLE_RANGE_PANE;
                        (*sy).range_argument = n;
                        style_set_range_string(sy, c!(""));
                    } else if strcaseeq_(tmp.add(6), "window") {
                        if found.is_null() {
                            break 'error;
                        }
                        let Ok(n) = strtonum(found, 0, u32::MAX) else {
                            break 'error;
                        };
                        (*sy).range_type = style_range_type::STYLE_RANGE_WINDOW;
                        (*sy).range_argument = n;
                        style_set_range_string(sy, c!(""));
                    } else if strcaseeq_(tmp.add(6), "session") {
                        if found.is_null() {
                            break 'error;
                        }
                        if *found != b'$' || *found.add(1) == b'\0' {
                            break 'error;
                        }
                        let Ok(n) = strtonum(found.add(1), 0, u32::MAX) else {
                            break 'error;
                        };
                        (*sy).range_type = style_range_type::STYLE_RANGE_SESSION;
                        (*sy).range_argument = n;
                        style_set_range_string(sy, c!(""));
                    } else if strcaseeq_(tmp.add(6), "user") {
                        if found.is_null() {
                            break 'error;
                        }
                        (*sy).range_type = style_range_type::STYLE_RANGE_USER;
                        (*sy).range_argument = 0;
                        style_set_range_string(sy, found);
                    }
                } else if strcaseeq_(tmp, "noalign") {
                    (*sy).align = STYLE_DEFAULT.align;
                } else if end > 6 && strncasecmp(tmp, c!("align="), 6) == 0 {
                    if strcaseeq_(tmp.add(6), "left") {
                        (*sy).align = style_align::STYLE_ALIGN_LEFT;
                    } else if strcaseeq_(tmp.add(6), "centre") {
                        (*sy).align = style_align::STYLE_ALIGN_CENTRE;
                    } else if strcaseeq_(tmp.add(6), "right") {
                        (*sy).align = style_align::STYLE_ALIGN_RIGHT;
                    } else if strcaseeq_(tmp.add(6), "absolute-centre") {
                        (*sy).align = style_align::STYLE_ALIGN_ABSOLUTE_CENTRE;
                    } else {
                        break 'error;
                    }
                } else if end > 5 && strncasecmp(tmp, c!("fill="), 5) == 0 {
                    let Some(s) = cstr_to_str_(tmp.add(5)) else { break 'error };
                    let value = colour_fromstring(s);
                    if value == -1 {
                        break 'error;
                    }
                    (*sy).fill = value;
                } else if end > 3 && strncasecmp(tmp.add(1), c!("g="), 2) == 0 {
                    let Some(s) = cstr_to_str_(tmp.add(3)) else { break 'error };
                    let value = colour_fromstring(s);
                    if value == -1 {
                        break 'error;
                    }
                    if *in_ == b'f' || *in_ == b'F' {
                        if value != 8 {
                            (*sy).gc.fg = value;
                        } else {
                            (*sy).gc.fg = (*base).fg;
                        }
                    } else if *in_ == b'b' || *in_ == b'B' {
                        if value != 8 {
                            (*sy).gc.bg = value;
                        } else {
                            (*sy).gc.bg = (*base).bg;
                        }
                    } else {
                        break 'error;
                    }
                } else if end > 3 && strncasecmp(tmp, c!("us="), 3) == 0 {
                    let Some(s) = cstr_to_str_(tmp.add(3)) else { break 'error };
                    let value = colour_fromstring(s);
                    if value == -1 {
                        break 'error;
                    }
                    if value != 8 {
                        (*sy).gc.us = value;
                    } else {
                        (*sy).gc.us = (*base).us;
                    }
                } else if strcaseeq_(tmp, "none") {
                    (*sy).gc.attr = GridAttr::empty();
                } else if end > 2 && strncasecmp(tmp, c!("no"), 2) == 0 {
                    let Some(s) = cstr_to_str_(tmp.add(2)) else { break 'error };
                    let Ok(value) = attributes_fromstring(s) else {
                        break 'error;
                    };
                    (*sy).gc.attr &= !value;
                } else {
                    let Some(s) = cstr_to_str_(tmp) else { break 'error };
                    let Ok(value) = attributes_fromstring(s) else {
                        break 'error;
                    };
                    (*sy).gc.attr |= value;
                }

                in_ = in_.add(end + strspn(in_.add(end), delimiters));
                if *in_ == b'\0' {
                    break;
                }
            }

            return 0;
        }

        // error:
        style_copy(sy, &raw const saved);
        -1
    }
}

/// Format a [`style`] struct into a comma-separated string.
/// Returns `"default"` if no style properties are set.
/// Uses a static buffer — not thread-safe (matches C tmux behavior).
pub unsafe fn style_tostring(sy: *const style) -> *const u8 {
    type s_type = [i8; 256];
    static mut S_BUF: MaybeUninit<s_type> = MaybeUninit::<s_type>::uninit();

    unsafe {
        let gc = &raw const (*sy).gc;
        let mut off: i32 = 0;
        let mut comma = c!("");
        let mut tmp = c!("");
        type b_type = [i8; 21];
        let mut b: b_type = [0; 21];

        let s = &raw mut S_BUF as *mut u8;
        *s = b'\0';

        if (*sy).list != style_list::STYLE_LIST_OFF {
            if (*sy).list == style_list::STYLE_LIST_ON {
                tmp = c!("on");
            } else if (*sy).list == style_list::STYLE_LIST_FOCUS {
                tmp = c!("focus");
            } else if (*sy).list == style_list::STYLE_LIST_LEFT_MARKER {
                tmp = c!("left-marker");
            } else if (*sy).list == style_list::STYLE_LIST_RIGHT_MARKER {
                tmp = c!("right-marker");
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}list={}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*sy).range_type != style_range_type::STYLE_RANGE_NONE {
            if (*sy).range_type == style_range_type::STYLE_RANGE_LEFT {
                tmp = c!("left");
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_RIGHT {
                tmp = c!("right");
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_PANE {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"pane|%%%u".as_ptr(),
                    (*sy).range_argument,
                );
                tmp = &raw const b as _;
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_WINDOW {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"window|%u".as_ptr(),
                    (*sy).range_argument,
                );
                tmp = &raw const b as _;
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_SESSION {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"session|$%u".as_ptr(),
                    (*sy).range_argument,
                );
                tmp = &raw const b as _;
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_USER {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"user|%s".as_ptr(),
                    (*sy).range_string,
                );
                tmp = &raw const b as _;
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}range={}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*sy).align != style_align::STYLE_ALIGN_DEFAULT {
            if (*sy).align == style_align::STYLE_ALIGN_LEFT {
                tmp = c!("left");
            } else if (*sy).align == style_align::STYLE_ALIGN_CENTRE {
                tmp = c!("centre");
            } else if (*sy).align == style_align::STYLE_ALIGN_RIGHT {
                tmp = c!("right");
            } else if (*sy).align == style_align::STYLE_ALIGN_ABSOLUTE_CENTRE {
                tmp = c!("absolute-centre");
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}align={}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*sy).default_type != style_default_type::STYLE_DEFAULT_BASE {
            if (*sy).default_type == style_default_type::STYLE_DEFAULT_PUSH {
                tmp = c!("push-default");
            } else if (*sy).default_type == style_default_type::STYLE_DEFAULT_POP {
                tmp = c!("pop-default");
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}{}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*sy).fill != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}fill={}",
                _s(comma),
                colour_tostring((*sy).fill),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*gc).fg != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}fg={}",
                _s(comma),
                colour_tostring((*gc).fg),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*gc).bg != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}bg={}",
                _s(comma),
                colour_tostring((*gc).bg),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        if (*gc).us != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}us={}",
                _s(comma),
                colour_tostring((*gc).us),
            )
            .unwrap() as i32;
            comma = c!(",");
        }
        #[expect(unused_assignments)]
        if !(*gc).attr.is_empty() {
            _ = xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}{}",
                _s(comma),
                attributes_tostring((*gc).attr),
            );
            comma = c!(",");
        }

        if *s == b'\0' {
            return c!("default");
        }
        s
    }
}

/// Merge a named style option into a Grid cell (additive — OR's attributes).
pub unsafe fn style_add(
    gc: *mut GridCell,
    oo: *mut options,
    name: *const u8,
    mut ft: *mut format_tree,
) {
    unsafe {
        let mut ft0: *mut format_tree = null_mut();

        if ft.is_null() {
            ft0 = format_create(null_mut(), null_mut(), 0, format_flags::FORMAT_NOJOBS);
            ft = ft0;
        }

        let mut sy = options_string_to_style(oo, cstr_to_str(name), ft);
        if sy.is_null() {
            sy = &raw mut STYLE_DEFAULT;
        }
        if (*sy).gc.fg != 8 {
            (*gc).fg = (*sy).gc.fg;
        }
        if (*sy).gc.bg != 8 {
            (*gc).bg = (*sy).gc.bg;
        }
        if (*sy).gc.us != 8 {
            (*gc).us = (*sy).gc.us;
        }
        (*gc).attr |= (*sy).gc.attr;

        if !ft0.is_null() {
            format_free(ft0);
        }
    }
}

/// Reset a Grid cell to defaults, then apply a named style option.
pub unsafe fn style_apply(
    gc: *mut GridCell,
    oo: *mut options,
    name: *const u8,
    ft: *mut format_tree,
) {
    unsafe {
        memcpy__(gc, &raw const GRID_DEFAULT_CELL);
        style_add(gc, oo, name, ft);
    }
}

/// Initialize a style from a Grid cell, resetting all other fields to defaults.
pub unsafe fn style_set(sy: *mut style, gc: *const GridCell) {
    unsafe {
        memcpy__(sy, &raw const STYLE_DEFAULT);
        memcpy__(&raw mut (*sy).gc, gc);
    }
}

/// Copy a style struct (shallow memcpy).
/// Copy a style struct (shallow memcpy).
pub unsafe fn style_copy(dst: *mut style, src: *const style) {
    unsafe {
        memcpy__(dst, src);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GRID_DEFAULT_CELL;

    /// Mutex to serialize tests that use style_tostring's static buffer.
    static STYLE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Parse a style string starting from STYLE_DEFAULT with GRID_DEFAULT_CELL as base.
    /// Returns the parsed style on success, or None on error.
    unsafe fn parse(input: &str) -> Option<style> {
        unsafe {
            let mut sy: style = *(&raw const STYLE_DEFAULT);
            let c = CString::new(input).unwrap();
            let rc = style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c.as_ptr().cast());
            if rc == 0 { Some(sy) } else { None }
        }
    }

    /// Format a style to string. Caller must hold STYLE_LOCK.
    unsafe fn tostring(sy: &style) -> String {
        unsafe {
            let ptr = style_tostring(sy as *const style);
            CStr::from_ptr(ptr.cast()).to_str().unwrap().to_string()
        }
    }

    // ---------------------------------------------------------------
    // style_parse — basic cases
    // ---------------------------------------------------------------

    #[test]
    fn parse_empty_string() {
        unsafe {
            // Empty string is valid — no changes to style.
            let sy = parse("").unwrap();
            // STYLE_DEFAULT has us=0 (black), so it's not fully "default".
            assert_eq!(sy.gc.fg, 8);
            assert_eq!(sy.gc.bg, 8);
        }
    }

    #[test]
    fn parse_default_keyword() {
        unsafe {
            let sy = parse("default").unwrap();
            assert_eq!(sy.gc.fg, 8);
            assert_eq!(sy.gc.bg, 8);
        }
    }

    #[test]
    fn parse_invalid_returns_none() {
        unsafe {
            assert!(parse("completely-invalid-gibberish").is_none());
        }
    }

    // ---------------------------------------------------------------
    // Colors
    // ---------------------------------------------------------------

    #[test]
    fn parse_fg_color() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let sy = parse("fg=red").unwrap();
            assert_eq!(sy.gc.fg, 1); // red = colour index 1
            assert_eq!(tostring(&sy), "fg=red");
        }
    }

    #[test]
    fn parse_bg_color() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let sy = parse("bg=blue").unwrap();
            assert_eq!(sy.gc.bg, 4); // blue = colour index 4
            assert!(tostring(&sy).contains("bg=blue"));
        }
    }

    #[test]
    fn parse_fg_and_bg() {
        unsafe {
            let sy = parse("fg=red,bg=blue").unwrap();
            assert_eq!(sy.gc.fg, 1);
            assert_eq!(sy.gc.bg, 4);
        }
    }

    #[test]
    fn parse_us_color() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let sy = parse("us=green").unwrap();
            assert_eq!(sy.gc.us, 2); // green = 2
            assert!(tostring(&sy).contains("us=green"));
        }
    }

    #[test]
    fn parse_fill_color() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let sy = parse("fill=yellow").unwrap();
            assert_eq!(sy.fill, 3); // yellow = 3
            assert!(tostring(&sy).contains("fill=yellow"));
        }
    }

    #[test]
    fn parse_invalid_color_is_error() {
        unsafe {
            assert!(parse("fg=notacolor").is_none());
        }
    }

    // ---------------------------------------------------------------
    // Attributes
    // ---------------------------------------------------------------

    #[test]
    fn parse_bold() {
        unsafe {
            let sy = parse("bold").unwrap();
            assert!(sy.gc.attr.intersects(GridAttr::GRID_ATTR_BRIGHT));
        }
    }

    #[test]
    fn parse_multiple_attrs() {
        unsafe {
            let sy = parse("bold,italics").unwrap();
            assert!(sy.gc.attr.intersects(GridAttr::GRID_ATTR_BRIGHT));
            assert!(sy.gc.attr.intersects(GridAttr::GRID_ATTR_ITALICS));
        }
    }

    #[test]
    fn parse_none_clears_attrs() {
        unsafe {
            // First set bold, then clear with none.
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("bold").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());
            assert!(!sy.gc.attr.is_empty());

            let c2 = CString::new("none").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert!(sy.gc.attr.is_empty());
        }
    }

    #[test]
    fn parse_no_prefix_removes_attr() {
        unsafe {
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("bold,italics").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());

            let c2 = CString::new("nobold").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert!(!sy.gc.attr.intersects(GridAttr::GRID_ATTR_BRIGHT));
            assert!(sy.gc.attr.intersects(GridAttr::GRID_ATTR_ITALICS));
        }
    }

    // ---------------------------------------------------------------
    // Alignment
    // ---------------------------------------------------------------

    #[test]
    fn parse_align_left() {
        unsafe {
            let sy = parse("align=left").unwrap();
            assert_eq!(sy.align, style_align::STYLE_ALIGN_LEFT);
        }
    }

    #[test]
    fn parse_align_centre() {
        unsafe {
            let sy = parse("align=centre").unwrap();
            assert_eq!(sy.align, style_align::STYLE_ALIGN_CENTRE);
        }
    }

    #[test]
    fn parse_align_right() {
        unsafe {
            let sy = parse("align=right").unwrap();
            assert_eq!(sy.align, style_align::STYLE_ALIGN_RIGHT);
        }
    }

    #[test]
    fn parse_align_absolute_centre() {
        unsafe {
            let sy = parse("align=absolute-centre").unwrap();
            assert_eq!(sy.align, style_align::STYLE_ALIGN_ABSOLUTE_CENTRE);
        }
    }

    #[test]
    fn parse_noalign() {
        unsafe {
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("align=left").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());
            let c2 = CString::new("noalign").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert_eq!(sy.align, style_align::STYLE_ALIGN_DEFAULT);
        }
    }

    #[test]
    fn parse_invalid_align_is_error() {
        unsafe {
            assert!(parse("align=invalid").is_none());
        }
    }

    // ---------------------------------------------------------------
    // List mode
    // ---------------------------------------------------------------

    #[test]
    fn parse_list_on() {
        unsafe {
            let sy = parse("list=on").unwrap();
            assert_eq!(sy.list, style_list::STYLE_LIST_ON);
        }
    }

    #[test]
    fn parse_list_focus() {
        unsafe {
            let sy = parse("list=focus").unwrap();
            assert_eq!(sy.list, style_list::STYLE_LIST_FOCUS);
        }
    }

    #[test]
    fn parse_nolist() {
        unsafe {
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("list=on").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());
            let c2 = CString::new("nolist").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert_eq!(sy.list, style_list::STYLE_LIST_OFF);
        }
    }

    // ---------------------------------------------------------------
    // Range
    // ---------------------------------------------------------------

    #[test]
    fn parse_range_left() {
        unsafe {
            let sy = parse("range=left").unwrap();
            assert_eq!(sy.range_type, style_range_type::STYLE_RANGE_LEFT);
        }
    }

    #[test]
    fn parse_range_right() {
        unsafe {
            let sy = parse("range=right").unwrap();
            assert_eq!(sy.range_type, style_range_type::STYLE_RANGE_RIGHT);
        }
    }

    #[test]
    fn parse_range_window() {
        unsafe {
            let sy = parse("range=window|42").unwrap();
            assert_eq!(sy.range_type, style_range_type::STYLE_RANGE_WINDOW);
            assert_eq!(sy.range_argument, 42);
        }
    }

    #[test]
    fn parse_range_pane() {
        unsafe {
            let sy = parse("range=pane|%7").unwrap();
            assert_eq!(sy.range_type, style_range_type::STYLE_RANGE_PANE);
            assert_eq!(sy.range_argument, 7);
        }
    }

    #[test]
    fn parse_norange() {
        unsafe {
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("range=left").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());
            let c2 = CString::new("norange").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert_eq!(sy.range_type, style_range_type::STYLE_RANGE_NONE);
        }
    }

    // ---------------------------------------------------------------
    // Default push/pop
    // ---------------------------------------------------------------

    #[test]
    fn parse_push_default() {
        unsafe {
            let sy = parse("push-default").unwrap();
            assert_eq!(sy.default_type, style_default_type::STYLE_DEFAULT_PUSH);
        }
    }

    #[test]
    fn parse_pop_default() {
        unsafe {
            let sy = parse("pop-default").unwrap();
            assert_eq!(sy.default_type, style_default_type::STYLE_DEFAULT_POP);
        }
    }

    // ---------------------------------------------------------------
    // Ignore
    // ---------------------------------------------------------------

    #[test]
    fn parse_ignore() {
        unsafe {
            let sy = parse("ignore").unwrap();
            assert_eq!(sy.ignore, 1);
        }
    }

    #[test]
    fn parse_noignore() {
        unsafe {
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("ignore").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());
            let c2 = CString::new("noignore").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert_eq!(sy.ignore, 0);
        }
    }

    // ---------------------------------------------------------------
    // Combined styles
    // ---------------------------------------------------------------

    #[test]
    fn parse_complex_style() {
        unsafe {
            let sy = parse("fg=red,bg=blue,bold,align=centre").unwrap();
            assert_eq!(sy.gc.fg, 1);
            assert_eq!(sy.gc.bg, 4);
            assert!(sy.gc.attr.intersects(GridAttr::GRID_ATTR_BRIGHT));
            assert_eq!(sy.align, style_align::STYLE_ALIGN_CENTRE);
        }
    }

    #[test]
    fn parse_space_delimited() {
        unsafe {
            let sy = parse("fg=red bg=blue").unwrap();
            assert_eq!(sy.gc.fg, 1);
            assert_eq!(sy.gc.bg, 4);
        }
    }

    // ---------------------------------------------------------------
    // style_tostring
    // ---------------------------------------------------------------

    #[test]
    fn tostring_default_has_us_black() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap();
            // STYLE_DEFAULT has us=0 (black), so tostring shows "us=black".
            let s = tostring(&*(&raw const STYLE_DEFAULT));
            assert_eq!(s, "us=black");
        }
    }

    #[test]
    fn tostring_round_trip_colors() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap();
            let sy = parse("fg=red").unwrap();
            let s = tostring(&sy);
            assert!(s.contains("fg=red"), "got: {s}");
        }
    }

    #[test]
    fn tostring_round_trip_align() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap();
            let sy = parse("align=centre").unwrap();
            let s = tostring(&sy);
            assert!(s.contains("align=centre"), "got: {s}");
        }
    }

    #[test]
    fn tostring_round_trip_list() {
        unsafe {
            let _lock = STYLE_LOCK.lock().unwrap();
            let sy = parse("list=on").unwrap();
            let s = tostring(&sy);
            assert!(s.contains("list=on"), "got: {s}");
        }
    }

    // ---------------------------------------------------------------
    // Error recovery — style restored on failure
    // ---------------------------------------------------------------

    #[test]
    fn parse_error_restores_style() {
        unsafe {
            let mut sy: style = STYLE_DEFAULT;
            let c1 = CString::new("fg=red").unwrap();
            style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c1.as_ptr().cast());
            let original_fg = sy.gc.fg;

            // This should fail and restore the style.
            let c2 = CString::new("fg=notacolor").unwrap();
            let rc = style_parse(&raw mut sy, &raw const GRID_DEFAULT_CELL, c2.as_ptr().cast());
            assert_eq!(rc, -1);
            assert_eq!(sy.gc.fg, original_fg);
        }
    }

    // ---------------------------------------------------------------
    // style_copy
    // ---------------------------------------------------------------

    #[test]
    fn copy_produces_identical_style() {
        unsafe {
            let src = parse("fg=red,bold,align=left").unwrap();
            let mut dst: style = STYLE_DEFAULT;
            style_copy(&raw mut dst, &raw const src);
            assert_eq!(dst.gc.fg, src.gc.fg);
            assert_eq!(dst.gc.attr, src.gc.attr);
            assert_eq!(dst.align, src.align);
        }
    }

    /// Regression: style_parse panicked on non-UTF-8 input because cstr_to_str
    /// was used instead of the fallible cstr_to_str_. Now returns -1.
    #[test]
    fn non_utf8_returns_error() {
        unsafe {
            // Single byte 0xd0 is an incomplete UTF-8 lead byte.
            let input = b"\xd0\0";
            let mut sy = STYLE_DEFAULT;
            let ret = style_parse(&raw mut sy, &raw const STYLE_DEFAULT.gc, input.as_ptr());
            assert_eq!(ret, -1);

            // "fg=" followed by non-UTF-8
            let input = b"fg=\xc3\x28\0";
            let ret = style_parse(&raw mut sy, &raw const STYLE_DEFAULT.gc, input.as_ptr());
            assert_eq!(ret, -1);
        }
    }
}
