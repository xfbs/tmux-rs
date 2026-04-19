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
//! Grid reader — cursor navigation over a terminal Grid.
//!
//! A `GridReader` provides stateful cursor movement over a `Grid`, used
//! primarily by copy mode for word/character jumping, selection, and search.
//! The reader tracks a `(cx, cy)` position and the Grid it operates on.
//!
//! ## Coordinate system
//! - `cx`: column (0-based, left to right)
//! - `cy`: row (0-based, absolute — includes scrollback history lines)
//! - `hsize`: number of history (scrollback) lines above the visible area
//! - `sy`: number of visible rows
//! - Total Grid height: `hsize + sy`
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

use tmux_types::{GridFlag, GridLineFlag, Utf8Data};
use crate::{WHITESPACE, codec, Grid, GridReader};

impl<'a> GridReader<'a> {
    /// Create a Grid reader at the given position over the given Grid.
    ///
    /// The reader borrows `gd` mutably for the lifetime `'a`, so it cannot
    /// outlive the Grid it navigates.
    pub fn new(gd: &'a mut Grid, cx: u32, cy: u32) -> Self {
        Self { gd, cx, cy }
    }

    /// Read the current cursor position as `(cx, cy)`.
    pub fn cursor(&self) -> (u32, u32) {
        (self.cx, self.cy)
    }

    /// Return the length of the current line (number of non-default cells from the left).
    pub fn line_length(&self) -> u32 {
        self.gd.line_length(self.cy)
    }

    /// Move cursor right by one character, skipping PADDING cells.
    /// If `wrap` is set and the cursor is at the end of the line, wraps to the
    /// start of the next line. If `all` is set, uses the full Grid width instead
    /// of the line content length as the right boundary.
    pub fn cursor_right(&mut self, wrap: u32, all: i32) {
        let px = if all != 0 {
            self.gd.sx
        } else {
            self.line_length()
        };

        if wrap != 0 && self.cx >= px && self.cy < self.gd.hsize + self.gd.sy - 1 {
            self.cursor_start_of_line(0);
            self.cursor_down();
        } else if self.cx < px {
            self.cx += 1;
            while self.cx < px {
                let gc = self.gd.get_cell(self.cx, self.cy);
                if !gc.flags.intersects(GridFlag::PADDING) {
                    break;
                }
                self.cx += 1;
            }
        }
    }

    /// Move cursor left by one character, skipping PADDING cells.
    /// If `wrap` is set, wraps from column 0 to the end of the previous line.
    /// Also wraps across wrapped lines (WRAPPED flag) regardless of the `wrap` parameter.
    pub fn cursor_left(&mut self, wrap: i32) {
        while self.cx > 0 {
            let gc = self.gd.get_cell(self.cx, self.cy);
            if !gc.flags.intersects(GridFlag::PADDING) {
                break;
            }
            self.cx -= 1;
        }
        if self.cx == 0
            && self.cy > 0
            && (wrap != 0
                || self
                    .gd
                    .line(self.cy - 1)
                    .flags
                    .intersects(GridLineFlag::WRAPPED))
        {
            self.cursor_up();
            self.cursor_end_of_line(0, 0);
        } else if self.cx > 0 {
            self.cx -= 1;
        }
    }

    /// Move cursor down one row. Adjusts cx leftward if it lands on a PADDING cell.
    pub fn cursor_down(&mut self) {
        if self.cy < self.gd.hsize + self.gd.sy - 1 {
            self.cy += 1;
        }
        while self.cx > 0 {
            let gc = self.gd.get_cell(self.cx, self.cy);
            if !gc.flags.intersects(GridFlag::PADDING) {
                break;
            }
            self.cx -= 1;
        }
    }

    /// Move cursor up one row. Adjusts cx leftward if it lands on a PADDING cell.
    pub fn cursor_up(&mut self) {
        if self.cy > 0 {
            self.cy -= 1;
        }
        while self.cx > 0 {
            let gc = self.gd.get_cell(self.cx, self.cy);
            if !gc.flags.intersects(GridFlag::PADDING) {
                break;
            }
            self.cx -= 1;
        }
    }

    /// Move cursor to column 0 of the current line. If `wrap` is set and the
    /// previous line has the WRAPPED flag, follows wrapped lines upward to find
    /// the true start of the logical line.
    pub fn cursor_start_of_line(&mut self, wrap: i32) {
        if wrap != 0 {
            while self.cy > 0
                && self
                    .gd
                    .line(self.cy - 1)
                    .flags
                    .intersects(GridLineFlag::WRAPPED)
            {
                self.cy -= 1;
            }
        }
        self.cx = 0;
    }

    /// Move cursor to the end of the current line. If `wrap` is set, follows
    /// wrapped lines downward to find the true end of the logical line.
    /// If `all` is set, moves to the full Grid width rather than content length.
    pub fn cursor_end_of_line(&mut self, wrap: i32, all: i32) {
        if wrap != 0 {
            let yy = self.gd.hsize + self.gd.sy - 1;
            while self.cy < yy
                && self
                    .gd
                    .line(self.cy)
                    .flags
                    .intersects(GridLineFlag::WRAPPED)
            {
                self.cy += 1;
            }
        }
        if all != 0 {
            self.cx = self.gd.sx;
        } else {
            self.cx = self.line_length();
        }
    }

    /// Handle line wrapping during forward iteration. If the cursor has moved past
    /// the end of the current line (`cx > *xx`), advances to the next line (following
    /// WRAPPED flags). Updates `*xx` and `*yy` to reflect the new line boundaries.
    /// Returns 0 if the cursor cannot advance further (reached the last line).
    pub fn handle_wrap(&mut self, xx: &mut u32, yy: &mut u32) -> i32 {
        while self.cx > *xx {
            if self.cy == *yy {
                return 0;
            }
            self.cursor_start_of_line(0);
            self.cursor_down();

            if self
                .gd
                .line(self.cy)
                .flags
                .intersects(GridLineFlag::WRAPPED)
            {
                *xx = self.gd.sx - 1;
            } else {
                *xx = self.line_length();
            }
        }
        1
    }

    /// Check whether the character at the current cursor position is in the given
    /// character set. Returns false for PADDING cells. Used to classify characters
    /// as whitespace, separators, or word characters during word movement.
    pub unsafe fn in_set(&self, set: *const u8) -> bool {
        let gc = self.gd.get_cell(self.cx, self.cy);
        if gc.flags.intersects(GridFlag::PADDING) {
            return false;
        }
        unsafe { codec().cstr_has(set, &raw const gc.data) }
    }

    /// Move cursor forward to the start of the next word (vi `w` behavior).
    ///
    /// Skips over the current token (word or separator run), then skips whitespace,
    /// landing on the first character of the next word. Handles line wrapping.
    pub unsafe fn cursor_next_word(&mut self, separators: *const u8) {
        // Do not break up wrapped words.
        let mut xx = if self
            .gd
            .line(self.cy)
            .flags
            .intersects(GridLineFlag::WRAPPED)
        {
            self.gd.sx - 1
        } else {
            self.line_length()
        };
        let mut yy = self.gd.hsize + self.gd.sy - 1;

        if self.handle_wrap(&mut xx, &mut yy) == 0 {
            return;
        }
        unsafe {
            if !self.in_set(WHITESPACE) {
                if self.in_set(separators) {
                    loop {
                        self.cx += 1;

                        if !(self.handle_wrap(&mut xx, &mut yy) != 0
                            && self.in_set(separators)
                            && !self.in_set(WHITESPACE))
                        {
                            break;
                        }
                    }
                } else {
                    loop {
                        self.cx += 1;
                        // Skip word characters: stop at separator or whitespace.
                        if !(self.handle_wrap(&mut xx, &mut yy) != 0
                            && !self.in_set(separators)
                            && !self.in_set(WHITESPACE))
                        {
                            break;
                        }
                    }
                }
            }
            while self.handle_wrap(&mut xx, &mut yy) != 0 && self.in_set(WHITESPACE) {
                self.cx += 1;
            }
        }
    }

    /// Move cursor forward to the end of the current or next word (vi `e` behavior).
    ///
    /// If on whitespace, skips to the next non-whitespace token and advances to its
    /// end. If inside a word, advances to the end of that word. If on a separator,
    /// advances to the end of the separator run.
    pub unsafe fn cursor_next_word_end(&mut self, separators: *const u8) {
        // Do not break up wrapped words.
        let mut xx = if self
            .gd
            .line(self.cy)
            .flags
            .intersects(GridLineFlag::WRAPPED)
        {
            self.gd.sx - 1
        } else {
            self.line_length()
        };
        let mut yy = self.gd.hsize + self.gd.sy - 1;

        unsafe {
            while self.handle_wrap(&mut xx, &mut yy) != 0 {
                if self.in_set(WHITESPACE) {
                    self.cx += 1;
                } else if self.in_set(separators) {
                    loop {
                        self.cx += 1;

                        if !(self.handle_wrap(&mut xx, &mut yy) != 0
                            && self.in_set(separators)
                            && !self.in_set(WHITESPACE))
                        {
                            break;
                        }
                    }
                    return;
                } else {
                    loop {
                        self.cx += 1;

                        if !(self.handle_wrap(&mut xx, &mut yy) != 0
                            && !(self.in_set(WHITESPACE) || self.in_set(separators)))
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
    pub unsafe fn cursor_previous_word(
        &mut self,
        separators: *const u8,
        already: i32,
        stop_at_eol: bool,
    ) {
        unsafe {
            let mut oldx: i32;
            let word_is_letters;

            if already != 0 || self.in_set(WHITESPACE) {
                loop {
                    if self.cx > 0 {
                        self.cx -= 1;
                        if !self.in_set(WHITESPACE) {
                            word_is_letters = !self.in_set(separators);
                            break;
                        }
                    } else {
                        if self.cy == 0 {
                            return;
                        }
                        self.cursor_up();
                        self.cursor_end_of_line(0, 0);

                        if stop_at_eol && self.cx > 0 {
                            oldx = self.cx as i32;
                            self.cx -= 1;
                            let at_eol = self.in_set(WHITESPACE);
                            self.cx = oldx as u32;
                            if at_eol {
                                word_is_letters = false;
                                break;
                            }
                        }
                    }
                }
            } else {
                word_is_letters = !self.in_set(separators);
            }

            let mut oldx;
            let mut oldy;
            loop {
                oldx = self.cx;
                oldy = self.cy;
                if self.cx == 0 {
                    if self.cy == 0
                        || (!self
                            .gd
                            .line(self.cy - 1)
                            .flags
                            .intersects(GridLineFlag::WRAPPED))
                    {
                        break;
                    }
                    self.cursor_up();
                    self.cursor_end_of_line(0, 1);
                }
                if self.cx > 0 {
                    self.cx -= 1;
                }

                if self.in_set(WHITESPACE) || word_is_letters == self.in_set(separators) {
                    break;
                }
            }
            self.cx = oldx;
            self.cy = oldy;
        }
    }

    /// Jump forward to the next occurrence of character `jc` on the current logical
    /// line (vi `f` behavior). Returns `true` if found. Does not cross
    /// non-wrapped line boundaries.
    pub fn cursor_jump(&mut self, jc: &Utf8Data) -> bool {
        let mut px = self.cx;
        let yy = self.gd.hsize + self.gd.sy - 1;

        let target_size = jc.size as usize;
        let target = &jc.data[..target_size];

        let mut py = self.cy;
        while py <= yy {
            let xx = self.gd.line_length(py);
            while px < xx {
                let gc = self.gd.get_cell(px, py);
                if !gc.flags.intersects(GridFlag::PADDING)
                    && gc.data.size as usize == target_size
                    && &gc.data.data[..target_size] == target
                {
                    self.cx = px;
                    self.cy = py;
                    return true;
                }
                px += 1;
            }

            if py == yy
                || !self.gd.line(py).flags.intersects(GridLineFlag::WRAPPED)
            {
                return false;
            }
            px = 0;
            py += 1;
        }
        false
    }

    /// Jump backward to the previous occurrence of character `jc` on the current
    /// logical line (vi `F` behavior). Returns `true` if found. Does not
    /// cross non-wrapped line boundaries.
    pub fn cursor_jump_back(&mut self, jc: &Utf8Data) -> bool {
        let mut xx = self.cx + 1;

        let target_size = jc.size as usize;
        let target = &jc.data[..target_size];

        let mut py = self.cy + 1;
        let mut px;
        while py > 0 {
            px = xx;
            while px > 0 {
                let gc = self.gd.get_cell(px - 1, py - 1);
                if !gc.flags.intersects(GridFlag::PADDING)
                    && gc.data.size as usize == target_size
                    && &gc.data.data[..target_size] == target
                {
                    self.cx = px - 1;
                    self.cy = py - 1;
                    return true;
                }
                px -= 1;
            }

            if py == 1
                || !self.gd.line(py - 2).flags.intersects(GridLineFlag::WRAPPED)
            {
                return false;
            }
            xx = self.gd.line_length(py - 2);
            py -= 1;
        }
        false
    }

    /// Move the cursor to the first non-space character on the current logical line.
    pub fn cursor_back_to_indentation(&mut self) {
        let yy = self.gd.hsize + self.gd.sy - 1;
        let oldx = self.cx;
        let oldy = self.cy;
        self.cursor_start_of_line(1);

        for py in self.cy..=yy {
            let xx = self.gd.line_length(py);
            for px in 0..xx {
                let gc = self.gd.get_cell(px, py);
                if gc.data.size != 1 || gc.data.data[0] != b' ' {
                    self.cx = px;
                    self.cy = py;
                    return;
                }
            }
            if !self
                .gd
                .line(py)
                .flags
                .intersects(GridLineFlag::WRAPPED)
            {
                break;
            }
        }
        self.cx = oldx;
        self.cy = oldy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid_create;
    use crate::test_support::install_test_codec;
    use tmux_types::{GridAttr, GridCell};

    /// Tiny C-literal helper — `c!("foo")` yields a NUL-terminated
    /// `*const u8`. Replaces the tmux-rs `c!` macro inside Grid's
    /// self-contained test suite.
    macro_rules! c {
        ($s:literal) => {
            concat!($s, "\0").as_ptr()
        };
    }

    /// Helper: create a Grid and fill lines with ASCII text. Registers
    /// the test codec so `in_set` / `cursor_next_word` / etc. can run
    /// without the tmux-rs utf8 machinery.
    fn make_grid_with_text(lines: &[&str], width: u32) -> Box<Grid> {
        install_test_codec();
        let mut gd = grid_create(width, lines.len() as u32, 0);
        for (y, line) in lines.iter().enumerate() {
            for (x, &ch) in line.as_bytes().iter().enumerate() {
                let cell = GridCell::new(
                    Utf8Data::new([ch], 0, 1, 1),
                    GridAttr::empty(),
                    GridFlag::empty(),
                    8, 8, 8, 0,
                );
                gd.set_cell(x as u32, y as u32, &cell);
            }
        }
        gd
    }

    /// Helper: create a Utf8Data for a single ASCII character.
    fn make_utf8_char(ch: u8) -> Utf8Data {
        Utf8Data::new([ch], 0, 1, 1)
    }

    // ---------------------------------------------------------------
    // Basic cursor movement
    // ---------------------------------------------------------------

    #[test]
    fn start_sets_cursor_position() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let gr = GridReader::new(&mut *gd, 3, 0);
            assert_eq!(gr.cx, 3);
            assert_eq!(gr.cy, 0);
            drop(gd);
        }
    }

    #[test]
    fn cursor_right_advances() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);
            gr.cursor_right(0, 0);
            assert_eq!(gr.cx, 1);
            drop(gd);
        }
    }

    #[test]
    fn cursor_left_retreats() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr = GridReader::new(&mut *gd, 3, 0);
            gr.cursor_left(0);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn cursor_left_at_zero_stays() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);
            gr.cursor_left(0);
            assert_eq!(gr.cx, 0);
            assert_eq!(gr.cy, 0);
            drop(gd);
        }
    }

    #[test]
    fn cursor_down_advances_row() {
        unsafe {
            let mut gd = make_grid_with_text(&["line1", "line2"], 80);
            let mut gr = GridReader::new(&mut *gd, 2, 0);
            gr.cursor_down();
            assert_eq!(gr.cy, 1);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn cursor_up_retreats_row() {
        unsafe {
            let mut gd = make_grid_with_text(&["line1", "line2"], 80);
            let mut gr = GridReader::new(&mut *gd, 2, 1);
            gr.cursor_up();
            assert_eq!(gr.cy, 0);
            assert_eq!(gr.cx, 2);
            drop(gd);
        }
    }

    #[test]
    fn cursor_up_at_top_stays() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);
            gr.cursor_up();
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
            let mut gr = GridReader::new(&mut *gd, 5, 0);
            gr.cursor_start_of_line(0);
            assert_eq!(gr.cx, 0);
            drop(gd);
        }
    }

    #[test]
    fn end_of_line_moves_to_line_length() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);
            gr.cursor_end_of_line(0, 0);
            assert_eq!(gr.cx, 5);
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Jump forward / backward
    // ---------------------------------------------------------------

    #[test]
    fn jump_forward_finds_character() {
        let mut gd = make_grid_with_text(&["abcdefghij"], 80);
        let mut gr = GridReader::new(&mut *gd, 0, 0);
        let jc = make_utf8_char(b'e');
        assert!(gr.cursor_jump(&jc));
        assert_eq!(gr.cx, 4);
    }

    #[test]
    fn jump_forward_not_found() {
        let mut gd = make_grid_with_text(&["abcdefghij"], 80);
        let mut gr = GridReader::new(&mut *gd, 0, 0);
        let jc = make_utf8_char(b'z');
        assert!(!gr.cursor_jump(&jc));
        assert_eq!(gr.cx, 0);
    }

    #[test]
    fn jump_forward_from_middle() {
        let mut gd = make_grid_with_text(&["abcdefghij"], 80);
        let mut gr = GridReader::new(&mut *gd, 5, 0);
        let jc = make_utf8_char(b'i');
        assert!(gr.cursor_jump(&jc));
        assert_eq!(gr.cx, 8);
    }

    #[test]
    fn jump_backward_finds_character() {
        let mut gd = make_grid_with_text(&["abcdefghij"], 80);
        let mut gr = GridReader::new(&mut *gd, 9, 0);
        let jc = make_utf8_char(b'c');
        assert!(gr.cursor_jump_back(&jc));
        assert_eq!(gr.cx, 2);
    }

    #[test]
    fn jump_backward_not_found() {
        let mut gd = make_grid_with_text(&["abcdefghij"], 80);
        let mut gr = GridReader::new(&mut *gd, 9, 0);
        let jc = make_utf8_char(b'z');
        assert!(!gr.cursor_jump_back(&jc));
        assert_eq!(gr.cx, 9);
    }

    #[test]
    fn jump_backward_finds_all_occurrences() {
        // "abcaefaghi" has 'a' at 0, 3, 6
        let mut gd = make_grid_with_text(&["abcaefaghi"], 80);
        let jc = make_utf8_char(b'a');

        let mut gr = GridReader::new(&mut *gd, 9, 0);
        assert!(gr.cursor_jump_back(&jc));
        assert_eq!(gr.cx, 6);

        let mut gr = GridReader::new(&mut *gd, 5, 0);
        assert!(gr.cursor_jump_back(&jc));
        assert_eq!(gr.cx, 3);

        let mut gr = GridReader::new(&mut *gd, 2, 0);
        assert!(gr.cursor_jump_back(&jc));
        assert_eq!(gr.cx, 0);
    }

    // ---------------------------------------------------------------
    // Next word / previous word
    // ---------------------------------------------------------------

    #[test]
    fn next_word_moves_to_next_word_start() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);

            gr.cursor_next_word(c!(""));
            assert_eq!(gr.cx, 4, "should land on 'two'");

            gr.cursor_next_word(c!(""));
            assert_eq!(gr.cx, 8, "should land on 'three'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_from_whitespace() {
        unsafe {
            let mut gd = make_grid_with_text(&["  hello world"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);

            gr.cursor_next_word(c!(""));
            assert_eq!(gr.cx, 2, "should skip leading spaces to 'hello'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_with_separators() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello.world end"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);

            gr.cursor_next_word(c!("."));
            assert_eq!(gr.cx, 5, "should stop at separator '.'");

            gr.cursor_next_word(c!("."));
            assert_eq!(gr.cx, 6, "should move past '.' to 'world'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_end_moves_to_end_of_word() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);

            gr.cursor_next_word_end(c!(""));
            assert_eq!(gr.cx, 3, "should land on 'e' of 'one'");

            drop(gd);
        }
    }

    #[test]
    fn next_word_end_from_end_of_word() {
        unsafe {
            let mut gd = make_grid_with_text(&["one   two"], 80);
            // Start at 'e' of "one" (col 2). Cursor is inside the word,
            // so next-word-end advances past the remaining word chars.
            let mut gr = GridReader::new(&mut *gd, 2, 0);

            gr.cursor_next_word_end(c!(""));
            assert_eq!(gr.cx, 3, "should advance past end of current word");

            drop(gd);
        }
    }

    #[test]
    fn previous_word_moves_back() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);

            // Verify the Grid is set up correctly
            {
                let mut gr = GridReader::new(&mut *gd, 8, 0);
                assert!(gr.in_set(WHITESPACE) == false, "col 8 should be 't'");
            }
            {
                let mut gr = GridReader::new(&mut *gd, 3, 0);
                assert!(gr.in_set(WHITESPACE) == true, "col 3 should be space");
            }

            let mut gr = GridReader::new(&mut *gd, 8, 0);
            // Use already=1, matching how tmux's previous-word command calls this.
            gr.cursor_previous_word(c!(""), 1, false);
            assert_eq!((gr.cx, gr.cy), (4, 0), "should land on 'two'");

            gr.cursor_previous_word(c!(""), 1, false);
            assert_eq!(gr.cx, 0, "should land on 'one'");

            drop(gd);
        }
    }

    #[test]
    fn previous_word_from_middle_of_word() {
        unsafe {
            let mut gd = make_grid_with_text(&["one two three"], 80);
            let mut gr = GridReader::new(&mut *gd, 5, 0);

            gr.cursor_previous_word(c!(""), 1, false);
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
            let mut gr = GridReader::new(&mut *gd, 8, 0);

            gr.cursor_back_to_indentation();
            assert_eq!(gr.cx, 4, "should skip 4 leading spaces");
            drop(gd);
        }
    }

    #[test]
    fn back_to_indentation_no_indent() {
        unsafe {
            let mut gd = make_grid_with_text(&["hello"], 80);
            let mut gr = GridReader::new(&mut *gd, 3, 0);

            gr.cursor_back_to_indentation();
            assert_eq!(gr.cx, 0, "no indent means column 0");
            drop(gd);
        }
    }

    #[test]
    fn back_to_indentation_all_spaces_stays() {
        unsafe {
            let mut gd = make_grid_with_text(&["     "], 80);
            let mut gr = GridReader::new(&mut *gd, 3, 0);

            gr.cursor_back_to_indentation();
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
            let mut gr = GridReader::new(&mut *gd, 0, 0);
            assert!(!gr.in_set(WHITESPACE));

            gr.cx = 1;
            assert!(gr.in_set(WHITESPACE));

            gr.cx = 2;
            assert!(!gr.in_set(WHITESPACE));

            drop(gd);
        }
    }

    #[test]
    fn in_set_detects_separators() {
        unsafe {
            let mut gd = make_grid_with_text(&["a.b"], 80);
            let mut gr = GridReader::new(&mut *gd, 0, 0);
            assert!(!gr.in_set(c!(".")));

            gr.cx = 1;
            assert!(gr.in_set(c!(".")));

            drop(gd);
        }
    }
}
