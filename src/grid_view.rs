// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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
//! Grid view — coordinate translation layer for the visible screen area.
//!
//! The grid stores both scrollback history and the visible screen. History
//! occupies rows `0..hsize`, and the visible area occupies `hsize..hsize+sy`.
//! This module translates "view coordinates" (where y=0 is the top of the
//! visible screen) into absolute grid coordinates by adding `hsize` to y.
//!
//! All public functions in this module are thin wrappers that apply the
//! coordinate offset and delegate to the corresponding `grid_*` functions
//! in [`grid_`](crate::grid_).

use crate::*;

/// Translate view x-coordinate to grid x-coordinate (currently identity).
fn grid_view_x(_gd: *mut grid, x: u32) -> u32 {
    x
}

/// Translate view y-coordinate to grid y-coordinate by adding `hsize`.
unsafe fn grid_view_y(gd: *mut grid, y: u32) -> u32 {
    unsafe { (*gd).hsize + (y) }
}

/// Get a cell from the visible area at view coordinates (px, py).
pub unsafe fn grid_view_get_cell(gd: *mut grid, px: u32, py: u32, gc: *mut grid_cell) {
    unsafe {
        (*gd).get_cell(grid_view_x(gd, px), grid_view_y(gd, py), gc);
    }
}

/// Set a cell in the visible area at view coordinates (px, py).
pub unsafe fn grid_view_set_cell(gd: *mut grid, px: u32, py: u32, gc: *const grid_cell) {
    unsafe {
        (*gd).set_cell(grid_view_x(gd, px), grid_view_y(gd, py), gc);
    }
}

pub unsafe fn grid_view_set_padding(gd: *mut grid, px: u32, py: u32) {
    unsafe {
        (*gd).set_padding(grid_view_x(gd, px), grid_view_y(gd, py));
    }
}

pub unsafe fn grid_view_set_cells(
    gd: *mut grid,
    px: u32,
    py: u32,
    gc: *const grid_cell,
    s: *const u8,
    slen: usize,
) {
    unsafe {
        (*gd).set_cells(grid_view_x(gd, px), grid_view_y(gd, py), gc, s, slen);
    }
}

/// Move all visible content into history and clear the screen.
/// Only moves lines up to the last non-empty line.
pub unsafe fn grid_view_clear_history(gd: *mut grid, bg: u32) {
    unsafe {
        let mut last = 0u32;

        for yy in 0..(*gd).sy {
            let gl = (*gd).get_line(grid_view_y(gd, yy));
            if (*gl).cellused != 0 {
                last = yy + 1;
            }
        }
        if last == 0 {
            grid_view_clear(gd, 0, 0, (*gd).sx, (*gd).sy, bg);
            return;
        }

        for _ in 0..(*gd).sy {
            (*gd).collect_history();
            (*gd).scroll_history(bg);
        }
        if last < (*gd).sy {
            grid_view_clear(gd, 0, 0, (*gd).sx, (*gd).sy - last, bg);
        }
        (*gd).hscrolled = 0;
    }
}

/// Clear a rectangular region in view coordinates.
pub unsafe fn grid_view_clear(gd: *mut grid, mut px: u32, mut py: u32, nx: u32, ny: u32, bg: u32) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        (*gd).clear(px, py, nx, ny, bg);
    }
}

pub unsafe fn grid_view_scroll_region_up(gd: *mut grid, mut rupper: u32, mut rlower: u32, bg: u32) {
    unsafe {
        if (*gd).flags & GRID_HISTORY != 0 {
            (*gd).collect_history();
            if rupper == 0 && rlower == (*gd).sy - 1 {
                (*gd).scroll_history(bg);
            } else {
                rupper = grid_view_y(gd, rupper);
                rlower = grid_view_y(gd, rlower);
                (*gd).scroll_history_region(rupper, rlower, bg);
            }
        } else {
            rupper = grid_view_y(gd, rupper);
            rlower = grid_view_y(gd, rlower);
            (*gd).move_lines(rupper, rupper + 1, rlower - rupper, bg);
        }
    }
}

pub unsafe fn grid_view_scroll_region_down(
    gd: *mut grid,
    mut rupper: u32,
    mut rlower: u32,
    bg: u32,
) {
    unsafe {
        rupper = grid_view_y(gd, rupper);
        rlower = grid_view_y(gd, rlower);

        (*gd).move_lines(rupper + 1, rupper, rlower - rupper, bg);
    }
}

pub unsafe fn grid_view_insert_lines(gd: *mut grid, mut py: u32, ny: u32, bg: u32) {
    unsafe {
        py = grid_view_y(gd, py);

        let sy = grid_view_y(gd, (*gd).sy);

        (*gd).move_lines(py + ny, py, sy - py - ny, bg);
    }
}

/// Insert lines in region.
pub unsafe fn grid_view_insert_lines_region(
    gd: *mut grid,
    mut rlower: u32,
    mut py: u32,
    ny: u32,
    bg: u32,
) {
    unsafe {
        rlower = grid_view_y(gd, rlower);

        py = grid_view_y(gd, py);

        let ny2 = rlower + 1 - py - ny;
        (*gd).move_lines(rlower + 1 - ny2, py, ny2, bg);
        // TODO does this bug exist upstream?
        (*gd).clear(0, py + ny2, (*gd).sx, ny.saturating_sub(ny2), bg);
    }
}

/// Delete lines.
pub unsafe fn grid_view_delete_lines(gd: *mut grid, mut py: u32, ny: u32, bg: u32) {
    unsafe {
        py = grid_view_y(gd, py);

        let sy = grid_view_y(gd, (*gd).sy);

        (*gd).move_lines(py, py + ny, sy - py - ny, bg);
        (*gd).clear(0, sy.saturating_sub(ny), (*gd).sx, ny, bg);
    }
}

/// Delete lines inside scroll region.
pub unsafe fn grid_view_delete_lines_region(
    gd: *mut grid,
    mut rlower: u32,
    mut py: u32,
    ny: u32,
    bg: u32,
) {
    unsafe {
        rlower = grid_view_y(gd, rlower);

        py = grid_view_y(gd, py);

        let ny2 = rlower + 1 - py - ny;
        (*gd).move_lines(py, py + ny, ny2, bg);
        // TODO does this bug exist in the tmux source code too
        (*gd).clear(0, py + ny2, (*gd).sx, ny.saturating_sub(ny2), bg);
    }
}

/// Insert characters.
pub unsafe fn grid_view_insert_cells(gd: *mut grid, mut px: u32, mut py: u32, nx: u32, bg: u32) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        let sx = grid_view_x(gd, (*gd).sx);

        if px >= sx - 1 {
            (*gd).clear(px, py, 1, 1, bg);
        } else {
            (*gd).move_cells(px + nx, px, py, sx - px - nx, bg);
        }
    }
}

/// Delete characters.
pub unsafe fn grid_view_delete_cells(gd: *mut grid, mut px: u32, mut py: u32, nx: u32, bg: u32) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        let sx = grid_view_x(gd, (*gd).sx);

        (*gd).move_cells(px, px + nx, py, sx - px - nx, bg);
        (*gd).clear(sx - nx, py, nx, 1, bg);
    }
}

/// Convert cells in the visible area into a string.
pub unsafe fn grid_view_string_cells(gd: *mut grid, mut px: u32, mut py: u32, nx: u32) -> *mut u8 {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        grid_string_cells(
            gd,
            px,
            py,
            nx,
            null_mut(),
            grid_string_flags::empty(),
            null_mut(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a grid cell containing a single ASCII character.
    fn make_char_cell(ch: u8) -> grid_cell {
        let mut gc = GRID_DEFAULT_CELL;
        gc.data.data[0] = ch;
        gc.data.size = 1;
        gc.data.width = 1;
        gc
    }

    /// Helper: read back the character at view coordinates (px, py).
    unsafe fn read_view_char(gd: *mut grid, px: u32, py: u32) -> u8 {
        unsafe {
            let mut gc: grid_cell = std::mem::zeroed();
            grid_view_get_cell(gd, px, py, &raw mut gc);
            gc.data.data[0]
        }
    }

    /// Helper: extract a string from the view at row py, columns 0..nx.
    unsafe fn view_row_string(gd: *mut grid, py: u32, nx: u32) -> String {
        unsafe {
            let ptr = grid_view_string_cells(gd, 0, py, nx);
            let s = CStr::from_ptr(ptr.cast()).to_str().unwrap().to_string();
            free_(ptr);
            s
        }
    }

    // ---------------------------------------------------------------
    // grid_view_y offset
    // ---------------------------------------------------------------

    #[test]
    fn view_y_adds_hsize() {
        unsafe {
            let mut gd = grid_create(10, 5, 100);
            let gd_ptr = &raw mut *gd;

            // With hsize=0, view y=0 maps to grid y=0.
            assert_eq!(grid_view_y(gd_ptr, 0), 0);
            assert_eq!(grid_view_y(gd_ptr, 3), 3);

            // Simulate history by scrolling a line into history.
            (*gd_ptr).scroll_history(8);
            assert_eq!(gd.hsize, 1);
            assert_eq!(grid_view_y(gd_ptr, 0), 1);
            assert_eq!(grid_view_y(gd_ptr, 3), 4);

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // grid_view_set_cell / grid_view_get_cell
    // ---------------------------------------------------------------

    #[test]
    fn set_and_get_cell_no_history() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;
            let gc = make_char_cell(b'A');

            grid_view_set_cell(gd_ptr, 3, 2, &gc);
            assert_eq!(read_view_char(gd_ptr, 3, 2), b'A');

            drop(gd);
        }
    }

    #[test]
    fn set_and_get_cell_with_history() {
        unsafe {
            let mut gd = grid_create(10, 5, 100);
            let gd_ptr = &raw mut *gd;

            // Write 'X' to view row 0 before scrolling.
            let gc_x = make_char_cell(b'X');
            grid_view_set_cell(gd_ptr, 0, 0, &gc_x);
            assert_eq!(read_view_char(gd_ptr, 0, 0), b'X');

            // Scroll into history — view row 0 is now a new empty line.
            (*gd_ptr).scroll_history(8);

            // 'X' is now in history (grid row 0), not visible view row 0.
            // View row 0 is now grid row 1 (the new line).
            assert_ne!(read_view_char(gd_ptr, 0, 0), b'X');

            // Write 'Y' to the new view row 0.
            let gc_y = make_char_cell(b'Y');
            grid_view_set_cell(gd_ptr, 0, 0, &gc_y);
            assert_eq!(read_view_char(gd_ptr, 0, 0), b'Y');

            // The old 'X' should still be in grid row 0 (history).
            let mut gc_read: grid_cell = std::mem::zeroed();
            (*gd_ptr).get_cell(0, 0, &raw mut gc_read);
            assert_eq!(gc_read.data.data[0], b'X');

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // grid_view_clear
    // ---------------------------------------------------------------

    #[test]
    fn clear_region() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Fill row 1 with 'B's.
            let gc = make_char_cell(b'B');
            for x in 0..10 {
                grid_view_set_cell(gd_ptr, x, 1, &gc);
            }
            assert_eq!(read_view_char(gd_ptr, 0, 1), b'B');

            // Clear columns 2..6 on row 1.
            grid_view_clear(gd_ptr, 2, 1, 4, 1, 8);

            // Cleared cells should be default (space).
            assert_eq!(read_view_char(gd_ptr, 2, 1), b' ');
            assert_eq!(read_view_char(gd_ptr, 5, 1), b' ');
            // Cells outside the cleared range should still be 'B'.
            assert_eq!(read_view_char(gd_ptr, 0, 1), b'B');
            assert_eq!(read_view_char(gd_ptr, 6, 1), b'B');

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // grid_view_string_cells
    // ---------------------------------------------------------------

    #[test]
    fn string_cells_reads_visible() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write "Hello" to view row 0.
            for (i, ch) in b"Hello".iter().enumerate() {
                let gc = make_char_cell(*ch);
                grid_view_set_cell(gd_ptr, i as u32, 0, &gc);
            }

            let s = view_row_string(gd_ptr, 0, 5);
            assert_eq!(s, "Hello");

            drop(gd);
        }
    }

    #[test]
    fn string_cells_with_history_offset() {
        unsafe {
            let mut gd = grid_create(10, 5, 100);
            let gd_ptr = &raw mut *gd;

            // Write "Line0" to view row 0.
            for (i, ch) in b"Line0".iter().enumerate() {
                let gc = make_char_cell(*ch);
                grid_view_set_cell(gd_ptr, i as u32, 0, &gc);
            }

            // Scroll it into history.
            (*gd_ptr).scroll_history(8);

            // Write "Line1" to new view row 0.
            for (i, ch) in b"Line1".iter().enumerate() {
                let gc = make_char_cell(*ch);
                grid_view_set_cell(gd_ptr, i as u32, 0, &gc);
            }

            // View should show Line1, not Line0.
            let s = view_row_string(gd_ptr, 0, 5);
            assert_eq!(s, "Line1");

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // grid_view_delete_cells / grid_view_insert_cells
    // ---------------------------------------------------------------

    #[test]
    fn delete_cells_shifts_left() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write "ABCDE" to row 0.
            for (i, ch) in b"ABCDE".iter().enumerate() {
                let gc = make_char_cell(*ch);
                grid_view_set_cell(gd_ptr, i as u32, 0, &gc);
            }

            // Delete 2 cells starting at column 1 (removes 'B' and 'C').
            grid_view_delete_cells(gd_ptr, 1, 0, 2, 8);

            // 'D' and 'E' should have shifted left.
            assert_eq!(read_view_char(gd_ptr, 0, 0), b'A');
            assert_eq!(read_view_char(gd_ptr, 1, 0), b'D');
            assert_eq!(read_view_char(gd_ptr, 2, 0), b'E');

            drop(gd);
        }
    }

    #[test]
    fn insert_cells_shifts_right() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write "ABCDE" to row 0.
            for (i, ch) in b"ABCDE".iter().enumerate() {
                let gc = make_char_cell(*ch);
                grid_view_set_cell(gd_ptr, i as u32, 0, &gc);
            }

            // Insert 2 cells at column 1.
            grid_view_insert_cells(gd_ptr, 1, 0, 2, 8);

            // 'A' stays, then 2 blank cells, then 'B', 'C', ...
            assert_eq!(read_view_char(gd_ptr, 0, 0), b'A');
            assert_eq!(read_view_char(gd_ptr, 1, 0), b' ');
            assert_eq!(read_view_char(gd_ptr, 2, 0), b' ');
            assert_eq!(read_view_char(gd_ptr, 3, 0), b'B');
            assert_eq!(read_view_char(gd_ptr, 4, 0), b'C');

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // grid_view_scroll_region_down
    // ---------------------------------------------------------------

    #[test]
    fn scroll_region_down() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write a letter to each row.
            for row in 0..5u32 {
                let gc = make_char_cell(b'A' + row as u8);
                grid_view_set_cell(gd_ptr, 0, row, &gc);
            }

            // Scroll rows 1..3 down (inserts blank at top of region).
            grid_view_scroll_region_down(gd_ptr, 1, 3, 8);

            assert_eq!(read_view_char(gd_ptr, 0, 0), b'A'); // unchanged
            assert_eq!(read_view_char(gd_ptr, 0, 1), b' '); // new blank
            assert_eq!(read_view_char(gd_ptr, 0, 2), b'B'); // was row 1
            assert_eq!(read_view_char(gd_ptr, 0, 3), b'C'); // was row 2
            assert_eq!(read_view_char(gd_ptr, 0, 4), b'E'); // unchanged

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // grid_view_clear_history
    // ---------------------------------------------------------------

    #[test]
    fn clear_history_moves_content() {
        unsafe {
            let mut gd = grid_create(10, 5, 100);
            let gd_ptr = &raw mut *gd;

            // Write content and verify it exists.
            let gc = make_char_cell(b'Z');
            grid_view_set_cell(gd_ptr, 0, 0, &gc);
            assert_eq!(gd.hsize, 0);

            // clear_history scrolls all visible lines into history.
            grid_view_clear_history(gd_ptr, 8);

            // hsize should have increased (content moved to history).
            assert!(gd.hsize > 0);

            drop(gd);
        }
    }
}
