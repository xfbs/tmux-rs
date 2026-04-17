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
//! Grid view — coordinate translation tests.
//!
//! The grid stores both scrollback history and the visible screen. History
//! occupies rows `0..hsize`, and the visible area occupies `hsize..hsize+sy`.
//! The view_* methods on [`grid`](crate::grid_::grid) translate "view
//! coordinates" (where y=0 is the top of the visible screen) into absolute
//! grid coordinates by adding `hsize` to y.
//!
//! All the former `grid_view_*` free functions have been migrated to `view_*`
//! methods on the `grid` type in [`grid_`](crate::grid_). This file now
//! contains only the unit tests for those methods.

#[cfg(test)]
mod tests {
    use crate::*;

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
            (*gd).view_get_cell(px, py, &raw mut gc);
            gc.data.data[0]
        }
    }

    /// Helper: extract a string from the view at row py, columns 0..nx.
    unsafe fn view_row_string(gd: *mut grid, py: u32, nx: u32) -> String {
        unsafe {
            let ptr = (*gd).view_string_cells(0, py, nx);
            let s = CStr::from_ptr(ptr.cast()).to_str().unwrap().to_string();
            free_(ptr);
            s
        }
    }

    // ---------------------------------------------------------------
    // view y-offset
    // ---------------------------------------------------------------

    #[test]
    fn view_y_adds_hsize() {
        unsafe {
            let mut gd = grid_create(10, 5, 100);
            let gd_ptr = &raw mut *gd;

            // With hsize=0, view y=0 maps to grid y=0.
            assert_eq!((*gd_ptr).hsize + 0, 0);
            assert_eq!((*gd_ptr).hsize + 3, 3);

            // Simulate history by scrolling a line into history.
            (*gd_ptr).scroll_history(8);
            assert_eq!(gd.hsize, 1);
            assert_eq!((*gd_ptr).hsize + 0, 1);
            assert_eq!((*gd_ptr).hsize + 3, 4);

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // view_set_cell / view_get_cell
    // ---------------------------------------------------------------

    #[test]
    fn set_and_get_cell_no_history() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;
            let gc = make_char_cell(b'A');

            (*gd_ptr).view_set_cell(3, 2, &gc);
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
            (*gd_ptr).view_set_cell(0, 0, &gc_x);
            assert_eq!(read_view_char(gd_ptr, 0, 0), b'X');

            // Scroll into history — view row 0 is now a new empty line.
            (*gd_ptr).scroll_history(8);

            // 'X' is now in history (grid row 0), not visible view row 0.
            // View row 0 is now grid row 1 (the new line).
            assert_ne!(read_view_char(gd_ptr, 0, 0), b'X');

            // Write 'Y' to the new view row 0.
            let gc_y = make_char_cell(b'Y');
            (*gd_ptr).view_set_cell(0, 0, &gc_y);
            assert_eq!(read_view_char(gd_ptr, 0, 0), b'Y');

            // The old 'X' should still be in grid row 0 (history).
            let mut gc_read: grid_cell = std::mem::zeroed();
            (*gd_ptr).get_cell(0, 0, &raw mut gc_read);
            assert_eq!(gc_read.data.data[0], b'X');

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // view_clear
    // ---------------------------------------------------------------

    #[test]
    fn clear_region() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Fill row 1 with 'B's.
            let gc = make_char_cell(b'B');
            for x in 0..10 {
                (*gd_ptr).view_set_cell(x, 1, &gc);
            }
            assert_eq!(read_view_char(gd_ptr, 0, 1), b'B');

            // Clear columns 2..6 on row 1.
            (*gd_ptr).view_clear(2, 1, 4, 1, 8);

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
    // view_string_cells
    // ---------------------------------------------------------------

    #[test]
    fn string_cells_reads_visible() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write "Hello" to view row 0.
            for (i, ch) in b"Hello".iter().enumerate() {
                let gc = make_char_cell(*ch);
                (*gd_ptr).view_set_cell(i as u32, 0, &gc);
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
                (*gd_ptr).view_set_cell(i as u32, 0, &gc);
            }

            // Scroll it into history.
            (*gd_ptr).scroll_history(8);

            // Write "Line1" to new view row 0.
            for (i, ch) in b"Line1".iter().enumerate() {
                let gc = make_char_cell(*ch);
                (*gd_ptr).view_set_cell(i as u32, 0, &gc);
            }

            // View should show Line1, not Line0.
            let s = view_row_string(gd_ptr, 0, 5);
            assert_eq!(s, "Line1");

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // view_delete_cells / view_insert_cells
    // ---------------------------------------------------------------

    #[test]
    fn delete_cells_shifts_left() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write "ABCDE" to row 0.
            for (i, ch) in b"ABCDE".iter().enumerate() {
                let gc = make_char_cell(*ch);
                (*gd_ptr).view_set_cell(i as u32, 0, &gc);
            }

            // Delete 2 cells starting at column 1 (removes 'B' and 'C').
            (*gd_ptr).view_delete_cells(1, 0, 2, 8);

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
                (*gd_ptr).view_set_cell(i as u32, 0, &gc);
            }

            // Insert 2 cells at column 1.
            (*gd_ptr).view_insert_cells(1, 0, 2, 8);

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
    // view_scroll_region_down
    // ---------------------------------------------------------------

    #[test]
    fn scroll_region_down() {
        unsafe {
            let mut gd = grid_create(10, 5, 0);
            let gd_ptr = &raw mut *gd;

            // Write a letter to each row.
            for row in 0..5u32 {
                let gc = make_char_cell(b'A' + row as u8);
                (*gd_ptr).view_set_cell(0, row, &gc);
            }

            // Scroll rows 1..3 down (inserts blank at top of region).
            (*gd_ptr).view_scroll_region_down(1, 3, 8);

            assert_eq!(read_view_char(gd_ptr, 0, 0), b'A'); // unchanged
            assert_eq!(read_view_char(gd_ptr, 0, 1), b' '); // new blank
            assert_eq!(read_view_char(gd_ptr, 0, 2), b'B'); // was row 1
            assert_eq!(read_view_char(gd_ptr, 0, 3), b'C'); // was row 2
            assert_eq!(read_view_char(gd_ptr, 0, 4), b'E'); // unchanged

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // view_clear_history
    // ---------------------------------------------------------------

    #[test]
    fn clear_history_moves_content() {
        unsafe {
            let mut gd = grid_create(10, 5, 100);
            let gd_ptr = &raw mut *gd;

            // Write content and verify it exists.
            let gc = make_char_cell(b'Z');
            (*gd_ptr).view_set_cell(0, 0, &gc);
            assert_eq!(gd.hsize, 0);

            // clear_history scrolls all visible lines into history.
            (*gd_ptr).view_clear_history(8);

            // hsize should have increased (content moved to history).
            assert!(gd.hsize > 0);

            drop(gd);
        }
    }
}
