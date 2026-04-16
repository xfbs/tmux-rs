// Copyright (c) 2020 Anindya Mukherjee <anindya49@hotmail.com>
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
//! Grid reader — cursor navigation over a terminal grid.
//!
//! A `grid_reader` provides stateful cursor movement over a `grid`, used
//! primarily by copy mode for word/character jumping, selection, and search.
//! The reader tracks a `(cx, cy)` position and the grid it operates on.
//!
//! ## Coordinate system
//! - `cx`: column (0-based, left to right)
//! - `cy`: row (0-based, absolute — includes scrollback history lines)
//! - `hsize`: number of history (scrollback) lines above the visible area
//! - `sy`: number of visible rows
//! - Total grid height: `hsize + sy`
//!
//! ## Word boundaries
//! Word movement functions classify characters into three categories:
//! 1. **Whitespace** — space characters (matched by `WHITESPACE`)
//! 2. **Separators** — configurable set (e.g. `-_@.`), from `word-separators` option
//! 3. **Word characters** — everything else (letters, digits, etc.)
//!
//! Word boundaries occur between adjacent characters of different categories.
//!
//! ## Wide characters and padding
//! Wide (CJK) characters occupy two cells. The second cell is a PADDING cell.
//! Cursor movement functions skip over PADDING cells to avoid landing in the
//! middle of a wide character.

use crate::*;

/// Initialize a grid reader at the given position.
pub unsafe fn grid_reader_start(gr: *mut grid_reader, gd: *mut grid, cx: u32, cy: u32) {
    unsafe {
        (*gr).gd = gd;
        (*gr).cx = cx;
        (*gr).cy = cy;
    }
}

/// Read the current cursor position.
pub unsafe fn grid_reader_get_cursor(gr: *mut grid_reader, cx: *mut u32, cy: *mut u32) {
    unsafe {
        *cx = (*gr).cx;
        *cy = (*gr).cy;
    }
}

/// Return the length of the current line (number of non-default cells from the left).
pub unsafe fn grid_reader_line_length(gr: *mut grid_reader) -> u32 {
    unsafe { grid_line_length((*gr).gd, (*gr).cy) }
}

/// Move cursor right by one character, skipping PADDING cells.
/// If `wrap` is set and the cursor is at the end of the line, wraps to the
/// start of the next line. If `all` is set, uses the full grid width instead
/// of the line content length as the right boundary.
pub unsafe fn grid_reader_cursor_right(gr: *mut grid_reader, wrap: u32, all: i32) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        let px = if all != 0 {
            (*(*gr).gd).sx
        } else {
            grid_reader_line_length(gr)
        };

        if wrap != 0 && (*gr).cx >= px && (*gr).cy < (*(*gr).gd).hsize + (*(*gr).gd).sy - 1 {
            grid_reader_cursor_start_of_line(gr, 0);
            grid_reader_cursor_down(gr);
        } else if (*gr).cx < px {
            (*gr).cx += 1;
            while (*gr).cx < px {
                grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc.as_mut_ptr());
                if !(*gc.as_ptr()).flags.intersects(grid_flag::PADDING) {
                    break;
                }
                (*gr).cx += 1;
            }
        }
    }
}

/// Move cursor left by one character, skipping PADDING cells.
/// If `wrap` is set, wraps from column 0 to the end of the previous line.
/// Also wraps across wrapped lines (WRAPPED flag) regardless of the `wrap` parameter.
pub unsafe fn grid_reader_cursor_left(gr: *mut grid_reader, wrap: i32) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();

        while (*gr).cx > 0 {
            grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc.as_mut_ptr());
            if !(*gc.as_ptr()).flags.intersects(grid_flag::PADDING) {
                break;
            }
            (*gr).cx -= 1;
        }
        if (*gr).cx == 0
            && (*gr).cy > 0
            && (wrap != 0
                || (*grid_get_line((*gr).gd, (*gr).cy - 1))
                    .flags
                    .intersects(grid_line_flag::WRAPPED))
        {
            grid_reader_cursor_up(gr);
            grid_reader_cursor_end_of_line(gr, 0, 0);
        } else if (*gr).cx > 0 {
            (*gr).cx -= 1;
        }
    }
}

/// Move cursor down one row. Adjusts cx leftward if it lands on a PADDING cell.
pub unsafe fn grid_reader_cursor_down(gr: *mut grid_reader) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        if (*gr).cy < (*(*gr).gd).hsize + (*(*gr).gd).sy - 1 {
            (*gr).cy += 1;
        }
        while (*gr).cx > 0 {
            grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc);
            if !(*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            (*gr).cx -= 1;
        }
    }
}

/// Move cursor up one row. Adjusts cx leftward if it lands on a PADDING cell.
pub unsafe fn grid_reader_cursor_up(gr: *mut grid_reader) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        if (*gr).cy > 0 {
            (*gr).cy -= 1;
        }
        while (*gr).cx > 0 {
            grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc);
            if !(*gc).flags.intersects(grid_flag::PADDING) {
                break;
            }
            (*gr).cx -= 1;
        }
    }
}

/// Move cursor to column 0 of the current line. If `wrap` is set and the
/// previous line has the WRAPPED flag, follows wrapped lines upward to find
/// the true start of the logical line.
pub unsafe fn grid_reader_cursor_start_of_line(gr: *mut grid_reader, wrap: i32) {
    unsafe {
        if wrap != 0 {
            while (*gr).cy > 0
                && (*grid_get_line((*gr).gd, (*gr).cy - 1))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                (*gr).cy -= 1;
            }
        }
        (*gr).cx = 0;
    }
}

/// Move cursor to the end of the current line. If `wrap` is set, follows
/// wrapped lines downward to find the true end of the logical line.
/// If `all` is set, moves to the full grid width rather than content length.
pub unsafe fn grid_reader_cursor_end_of_line(gr: *mut grid_reader, wrap: i32, all: i32) {
    unsafe {
        if wrap != 0 {
            let yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;
            while (*gr).cy < yy
                && (*grid_get_line((*gr).gd, (*gr).cy))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                (*gr).cy += 1;
            }
        }
        if all != 0 {
            (*gr).cx = (*(*gr).gd).sx;
        } else {
            (*gr).cx = grid_reader_line_length(gr);
        }
    }
}

/// Handle line wrapping during forward iteration. If the cursor has moved past
/// the end of the current line (`cx > xx`), advances to the next line (following
/// WRAPPED flags). Updates `xx` and `yy` to reflect the new line boundaries.
/// Returns 0 if the cursor cannot advance further (reached the last line).
pub unsafe fn grid_reader_handle_wrap(gr: *mut grid_reader, xx: *mut u32, yy: *mut u32) -> i32 {
    unsafe {
        while (*gr).cx > *xx {
            if (*gr).cy == *yy {
                return 0;
            }
            grid_reader_cursor_start_of_line(gr, 0);
            grid_reader_cursor_down(gr);

            if (*grid_get_line((*gr).gd, (*gr).cy))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                *xx = (*(*gr).gd).sx - 1;
            } else {
                *xx = grid_reader_line_length(gr);
            }
        }
    }
    1
}

/// Check whether the character at the current cursor position is in the given
/// character set. Returns false for PADDING cells. Used to classify characters
/// as whitespace, separators, or word characters during word movement.
pub unsafe fn grid_reader_in_set(gr: *mut grid_reader, set: *const u8) -> bool {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        grid_get_cell((*gr).gd, (*gr).cx, (*gr).cy, gc);
        if (*gc).flags.intersects(grid_flag::PADDING) {
            return false;
        }
        utf8_cstrhas(set, &raw mut (*gc).data)
    }
}

/// Move cursor forward to the start of the next word (vi `w` behavior).
///
/// Skips over the current token (word or separator run), then skips whitespace,
/// landing on the first character of the next word. Handles line wrapping.
pub unsafe fn grid_reader_cursor_next_word(gr: *mut grid_reader, separators: *const u8) {
    unsafe {
        // Do not break up wrapped words.
        let mut xx = if (*grid_get_line((*gr).gd, (*gr).cy))
            .flags
            .intersects(grid_line_flag::WRAPPED)
        {
            (*(*gr).gd).sx - 1
        } else {
            grid_reader_line_length(gr)
        };
        let mut yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;

        if grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) == 0 {
            return;
        }
        if !grid_reader_in_set(gr, WHITESPACE) {
            if grid_reader_in_set(gr, separators) {
                loop {
                    (*gr).cx += 1;

                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && grid_reader_in_set(gr, separators)
                        && !grid_reader_in_set(gr, WHITESPACE))
                    {
                        break;
                    }
                }
            } else {
                loop {
                    (*gr).cx += 1;
                    // Skip word characters: stop at separator or whitespace.
                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && !grid_reader_in_set(gr, separators)
                        && !grid_reader_in_set(gr, WHITESPACE))
                    {
                        break;
                    }
                }
            }
        }
        while grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
            && grid_reader_in_set(gr, WHITESPACE)
        {
            (*gr).cx += 1;
        }
    }
}

/// Move cursor forward to the end of the current or next word (vi `e` behavior).
///
/// If on whitespace, skips to the next non-whitespace token and advances to its
/// end. If inside a word, advances to the end of that word. If on a separator,
/// advances to the end of the separator run.
pub unsafe fn grid_reader_cursor_next_word_end(gr: *mut grid_reader, separators: *const u8) {
    unsafe {
        // Do not break up wrapped words.
        let mut xx = if (*grid_get_line((*gr).gd, (*gr).cy))
            .flags
            .intersects(grid_line_flag::WRAPPED)
        {
            (*(*gr).gd).sx - 1
        } else {
            grid_reader_line_length(gr)
        };
        let mut yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;

        while grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0 {
            if grid_reader_in_set(gr, WHITESPACE) {
                (*gr).cx += 1;
            } else if grid_reader_in_set(gr, separators) {
                loop {
                    (*gr).cx += 1;

                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && grid_reader_in_set(gr, separators)
                        && !grid_reader_in_set(gr, WHITESPACE))
                    {
                        break;
                    }
                }
                return;
            } else {
                loop {
                    (*gr).cx += 1;

                    if !(grid_reader_handle_wrap(gr, &raw mut xx, &raw mut yy) != 0
                        && !(grid_reader_in_set(gr, WHITESPACE)
                            || grid_reader_in_set(gr, separators)))
                    {
                        break;
                    }
                }
                return;
            }
        }
    }
}

/// Move cursor backward to the start of the previous word (vi `b` behavior).
///
/// If `already` is set (the normal case from copy mode), first skips backward
/// past whitespace to find a word, then scans backward through that word to
/// find its start. The cursor lands on the first character of the word.
///
/// If `stop_at_eol` is true, stops at line boundaries rather than crossing them.
pub unsafe fn grid_reader_cursor_previous_word(
    gr: *mut grid_reader,
    separators: *const u8,
    already: i32,
    stop_at_eol: bool,
) {
    unsafe {
        let mut oldx: i32;
        let word_is_letters;

        if already != 0 || grid_reader_in_set(gr, WHITESPACE) {
            loop {
                if (*gr).cx > 0 {
                    (*gr).cx -= 1;
                    if !grid_reader_in_set(gr, WHITESPACE) {
                        word_is_letters = !grid_reader_in_set(gr, separators);
                        break;
                    }
                } else {
                    if (*gr).cy == 0 {
                        return;
                    }
                    grid_reader_cursor_up(gr);
                    grid_reader_cursor_end_of_line(gr, 0, 0);

                    if stop_at_eol && (*gr).cx > 0 {
                        oldx = (*gr).cx as i32;
                        (*gr).cx -= 1;
                        let at_eol = grid_reader_in_set(gr, WHITESPACE);
                        (*gr).cx = oldx as u32;
                        if at_eol {
                            word_is_letters = false;
                            break;
                        }
                    }
                }
            }
        } else {
            word_is_letters = !grid_reader_in_set(gr, separators);
        }

        let mut oldx;
        let mut oldy;
        loop {
            oldx = (*gr).cx;
            oldy = (*gr).cy;
            if (*gr).cx == 0 {
                if (*gr).cy == 0
                    || (!(*grid_get_line((*gr).gd, (*gr).cy - 1))
                        .flags
                        .intersects(grid_line_flag::WRAPPED))
                {
                    break;
                }
                grid_reader_cursor_up(gr);
                grid_reader_cursor_end_of_line(gr, 0, 1);
            }
            if (*gr).cx > 0 {
                (*gr).cx -= 1;
            }

            if grid_reader_in_set(gr, WHITESPACE)
                || word_is_letters == grid_reader_in_set(gr, separators)
            {
                break;
            }
        }
        (*gr).cx = oldx;
        (*gr).cy = oldy;
    }
}

/// Jump forward to the next occurrence of character `jc` on the current logical
/// line (vi `f` behavior). Returns 1 if found, 0 if not. Does not cross
/// non-wrapped line boundaries.
pub unsafe fn grid_reader_cursor_jump(gr: *mut grid_reader, jc: *const utf8_data) -> i32 {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        let mut px = (*gr).cx;
        let yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;

        let mut py = (*gr).cy;
        while py <= yy {
            let xx = grid_line_length((*gr).gd, py);
            while px < xx {
                grid_get_cell((*gr).gd, px, py, gc);
                if !(*gc).flags.intersects(grid_flag::PADDING)
                    && (*gc).data.size == (*jc).size
                    && memcmp(
                        (*gc).data.data.as_ptr().cast(),
                        (*jc).data.as_ptr().cast(),
                        (*gc).data.size as usize,
                    ) == 0
                {
                    (*gr).cx = px;
                    (*gr).cy = py;
                    return 1;
                }
                px += 1;
            }

            if py == yy
                || !(*grid_get_line((*gr).gd, py))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                return 0;
            }
            px = 0;
            py += 1;
        }
    }
    0
}

/// Jump backward to the previous occurrence of character `jc` on the current
/// logical line (vi `F` behavior). Returns 1 if found, 0 if not. Does not
/// cross non-wrapped line boundaries.
pub unsafe fn grid_reader_cursor_jump_back(gr: *mut grid_reader, jc: *mut utf8_data) -> i32 {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        let mut xx = (*gr).cx + 1;

        let mut py = (*gr).cy + 1;
        let mut px;
        while py > 0 {
            px = xx;
            while px > 0 {
                grid_get_cell((*gr).gd, px - 1, py - 1, gc);
                if !(*gc).flags.intersects(grid_flag::PADDING)
                    && (*gc).data.size == (*jc).size
                    && memcmp(
                        (*gc).data.data.as_ptr().cast(),
                        (*jc).data.as_ptr().cast(),
                        (*gc).data.size as usize,
                    ) == 0
                {
                    (*gr).cx = px - 1;
                    (*gr).cy = py - 1;
                    return 1;
                }
                px -= 1;
            }

            if py == 1
                || !(*grid_get_line((*gr).gd, py - 2))
                    .flags
                    .intersects(grid_line_flag::WRAPPED)
            {
                return 0;
            }
            xx = grid_line_length((*gr).gd, py - 2);
            py -= 1;
        }
    }
    0
}

/// Move the cursor to the first non-space character on the current logical line.
pub unsafe fn grid_reader_cursor_back_to_indentation(gr: *mut grid_reader) {
    unsafe {
        let mut gc = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();
        // u_int px, py, xx, yy, oldx, oldy;

        let yy = (*(*gr).gd).hsize + (*(*gr).gd).sy - 1;
        let oldx = (*gr).cx;
        let oldy = (*gr).cy;
        grid_reader_cursor_start_of_line(gr, 1);

        for py in (*gr).cy..=yy {
            let xx = grid_line_length((*gr).gd, py);
            for px in 0..xx {
                grid_get_cell((*gr).gd, px, py, gc);
                if (*gc).data.size != 1 || (*gc).data.data[0] != b' ' {
                    (*gr).cx = px;
                    (*gr).cy = py;
                    return;
                }
            }
            if !(*grid_get_line((*gr).gd, py))
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                break;
            }
        }
        (*gr).cx = oldx;
        (*gr).cy = oldy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::zeroed;

    /// Helper: create a grid and fill lines with ASCII text.
    unsafe fn make_grid_with_text(lines: &[&str], width: u32) -> Box<grid> {
        let mut gd = grid_create(width, lines.len() as u32, 0);
        for (y, line) in lines.iter().enumerate() {
            for (x, &ch) in line.as_bytes().iter().enumerate() {
                let cell = grid_cell::new(
                    utf8_data::new([ch], 0, 1, 1),
                    grid_attr::empty(),
                    grid_flag::empty(),
                    8, 8, 8, 0,
                );
                grid_set_cell(&raw mut *gd, x as u32, y as u32, &cell);
            }
        }
        gd
    }

    /// Helper: create a utf8_data for a single ASCII character.
    fn make_utf8_char(ch: u8) -> utf8_data {
        utf8_data::new([ch], 0, 1, 1)
    }

    // ---------------------------------------------------------------
    // Basic cursor movement
    // ---------------------------------------------------------------

    #[test]
    fn start_sets_cursor_position() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 3, 0);
            assert_eq!(gr.cx, 3);
            assert_eq!(gr.cy, 0);
            drop(gd);
        }
    }

    #[test]
    fn cursor_right_advances() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            grid_reader_cursor_right(&mut gr, 0, 0);
            assert_eq!(gr.cx, 1);
            drop(gd);
        }
    }

    #[test]
    fn cursor_left_retreats() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 3, 0);
            grid_reader_cursor_left(&mut gr, 0);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn cursor_left_at_zero_stays() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            grid_reader_cursor_left(&mut gr, 0);
            assert_eq!(gr.cx, 0);
            assert_eq!(gr.cy, 0);
            drop(gd);
        }
    }

    #[test]
    fn cursor_down_advances_row() {
        unsafe {
            let mut gd = make_grid_with_text(&["line1", "line2"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 2, 0);
            grid_reader_cursor_down(&mut gr);
            assert_eq!(gr.cy, 1);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn cursor_up_retreats_row() {
        unsafe {
            let mut gd = make_grid_with_text(&["line1", "line2"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 2, 1);
            grid_reader_cursor_up(&mut gr);
            assert_eq!(gr.cy, 0);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn cursor_up_at_top_stays() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            grid_reader_cursor_up(&mut gr);
            assert_eq!(gr.cy, 0);
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Start/end of line
    // ---------------------------------------------------------------

    #[test]
    fn start_of_line_moves_to_column_zero() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello world"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 5, 0);
            grid_reader_cursor_start_of_line(&mut gr, 0);
            assert_eq!(gr.cx, 0);
            drop(gd);
        }
    }

    #[test]
    fn end_of_line_moves_to_line_length() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            grid_reader_cursor_end_of_line(&mut gr, 0, 0);
            assert_eq!(gr.cx, 5);
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Jump forward / backward
    // ---------------------------------------------------------------

    #[test]
    fn jump_forward_finds_character() {
        unsafe {
            let mut gd = make_grid_with_text(&["abcdefghij"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            let jc = make_utf8_char(b'e');
            assert_eq!(grid_reader_cursor_jump(&mut gr, &jc), 1);
            assert_eq!(gr.cx, 4);
            drop(gd);
        }
    }

    #[test]
    fn jump_forward_not_found() {
        unsafe {
            let mut gd = make_grid_with_text(&["abcdefghij"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            let jc = make_utf8_char(b'z');
            assert_eq!(grid_reader_cursor_jump(&mut gr, &jc), 0);
            assert_eq!(gr.cx, 0);
            drop(gd);
        }
    }

    #[test]
    fn jump_forward_from_middle() {
        unsafe {
            let mut gd = make_grid_with_text(&["abcdefghij"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 5, 0);
            let jc = make_utf8_char(b'i');
            assert_eq!(grid_reader_cursor_jump(&mut gr, &jc), 1);
            assert_eq!(gr.cx, 8);
            drop(gd);
        }
    }

    #[test]
    fn jump_backward_finds_character() {
        unsafe {
            let mut gd = make_grid_with_text(&["abcdefghij"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 9, 0);
            let mut jc = make_utf8_char(b'c');
            assert_eq!(grid_reader_cursor_jump_back(&mut gr, &mut jc), 1);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn jump_backward_not_found() {
        unsafe {
            let mut gd = make_grid_with_text(&["abcdefghij"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 9, 0);
            let mut jc = make_utf8_char(b'z');
            assert_eq!(grid_reader_cursor_jump_back(&mut gr, &mut jc), 0);
            assert_eq!(gr.cx, 9);
            drop(gd);
        }
    }

    #[test]
    fn jump_backward_finds_all_occurrences() {
        unsafe {
            // "abcaefaghi" has 'a' at 0, 3, 6
            let mut gd = make_grid_with_text(&["abcaefaghi"], 80);
            let mut gr: grid_reader = zeroed();
            let mut jc = make_utf8_char(b'a');

            grid_reader_start(&mut gr, &raw mut *gd, 9, 0);
            assert_eq!(grid_reader_cursor_jump_back(&mut gr, &mut jc), 1);
            assert_eq!(gr.cx, 6);

            grid_reader_start(&mut gr, &raw mut *gd, 5, 0);
            assert_eq!(grid_reader_cursor_jump_back(&mut gr, &mut jc), 1);
            assert_eq!(gr.cx, 3);

            grid_reader_start(&mut gr, &raw mut *gd, 2, 0);
            assert_eq!(grid_reader_cursor_jump_back(&mut gr, &mut jc), 1);
            assert_eq!(gr.cx, 0);

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Next word / previous word
    // ---------------------------------------------------------------

    #[test]
    fn next_word_moves_to_next_word_start() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);

            grid_reader_cursor_next_word(&mut gr, c!(""));
            assert_eq!(gr.cx, 4, "should land on 'two'");

            grid_reader_cursor_next_word(&mut gr, c!(""));
            assert_eq!(gr.cx, 8, "should land on 'three'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_from_whitespace() {
        unsafe {
            let mut gd = make_grid_with_text(&["  hello world"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);

            grid_reader_cursor_next_word(&mut gr, c!(""));
            assert_eq!(gr.cx, 2, "should skip leading spaces to 'hello'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_with_separators() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello.world end"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);

            grid_reader_cursor_next_word(&mut gr, c!("."));
            assert_eq!(gr.cx, 5, "should stop at separator '.'");

            grid_reader_cursor_next_word(&mut gr, c!("."));
            assert_eq!(gr.cx, 6, "should move past '.' to 'world'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_end_moves_to_end_of_word() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);

            grid_reader_cursor_next_word_end(&mut gr, c!(""));
            assert_eq!(gr.cx, 3, "should land on 'e' of 'one'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_end_from_end_of_word() {
        unsafe {
            let mut gd = make_grid_with_text(&["one   two"], 80);
            let mut gr: grid_reader = zeroed();
            // Start at 'e' of "one" (col 2). Cursor is inside the word,
            // so next-word-end advances past the remaining word chars.
            grid_reader_start(&mut gr, &raw mut *gd, 2, 0);

            grid_reader_cursor_next_word_end(&mut gr, c!(""));
            assert_eq!(gr.cx, 3, "should advance past end of current word");

            drop(gd);
        }
    }

    #[test]
    fn previous_word_moves_back() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 8, 0);

            // Verify the grid is set up correctly
            assert!(grid_reader_in_set(&mut gr, WHITESPACE) == false, "col 8 should be 't'");
            grid_reader_start(&mut gr, &raw mut *gd, 3, 0);
            assert!(grid_reader_in_set(&mut gr, WHITESPACE) == true, "col 3 should be space");
            grid_reader_start(&mut gr, &raw mut *gd, 8, 0);

            // Use already=1, matching how tmux's previous-word command calls this.
            grid_reader_cursor_previous_word(&mut gr, c!(""), 1, false);
            assert_eq!((gr.cx, gr.cy), (4, 0), "should land on 'two'");

            grid_reader_cursor_previous_word(&mut gr, c!(""), 1, false);
            assert_eq!(gr.cx, 0, "should land on 'one'");

            drop(gd);
        }
    }

    #[test]
    fn previous_word_from_middle_of_word() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 5, 0);

            grid_reader_cursor_previous_word(&mut gr, c!(""), 1, false);
            assert_eq!(gr.cx, 4, "should land on start of 'two'");

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Back to indentation
    // ---------------------------------------------------------------

    #[test]
    fn back_to_indentation_skips_spaces() {
        unsafe {
            let mut gd = make_grid_with_text(&["    indented"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 8, 0);

            grid_reader_cursor_back_to_indentation(&mut gr);
            assert_eq!(gr.cx, 4, "should skip 4 leading spaces");
            drop(gd);
        }
    }

    #[test]
    fn back_to_indentation_no_indent() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 3, 0);

            grid_reader_cursor_back_to_indentation(&mut gr);
            assert_eq!(gr.cx, 0, "no indent means column 0");
            drop(gd);
        }
    }

    #[test]
    fn back_to_indentation_all_spaces_stays() {
        unsafe {
            let mut gd = make_grid_with_text(&["     "], 80);
            let mut gr: grid_reader = zeroed();
            grid_reader_start(&mut gr, &raw mut *gd, 3, 0);

            grid_reader_cursor_back_to_indentation(&mut gr);
            assert_eq!(gr.cx, 3, "all spaces — should stay at original position");
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // in_set
    // ---------------------------------------------------------------

    #[test]
    fn in_set_detects_whitespace() {
        unsafe {
            let mut gd = make_grid_with_text(&["a b"], 80);
            let mut gr: grid_reader = zeroed();

            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            assert!(!grid_reader_in_set(&mut gr, WHITESPACE));

            grid_reader_start(&mut gr, &raw mut *gd, 1, 0);
            assert!(grid_reader_in_set(&mut gr, WHITESPACE));

            grid_reader_start(&mut gr, &raw mut *gd, 2, 0);
            assert!(!grid_reader_in_set(&mut gr, WHITESPACE));

            drop(gd);
        }
    }

    #[test]
    fn in_set_detects_separators() {
        unsafe {
            let mut gd = make_grid_with_text(&["a.b"], 80);
            let mut gr: grid_reader = zeroed();

            grid_reader_start(&mut gr, &raw mut *gd, 0, 0);
            assert!(!grid_reader_in_set(&mut gr, c!(".")));

            grid_reader_start(&mut gr, &raw mut *gd, 1, 0);
            assert!(grid_reader_in_set(&mut gr, c!(".")));

            drop(gd);
        }
    }
}
