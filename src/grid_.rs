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
use crate::*;

/// Default grid cell data.
pub static GRID_DEFAULT_CELL: grid_cell = grid_cell::new(
    utf8_data::new([b' '], 0, 1, 1),
    grid_attr::empty(),
    grid_flag::empty(),
    8,
    8,
    8,
    0,
);

/// Padding grid cell data. Padding cells are the only zero width cell that
/// appears in the grid - because of this, they are always extended cells.
pub static GRID_PADDING_CELL: grid_cell = grid_cell::new(
    utf8_data::new([b'!'], 0, 0, 0),
    grid_attr::empty(),
    grid_flag::PADDING,
    8,
    8,
    8,
    0,
);

/// Cleared grid cell data.
pub static GRID_CLEARED_CELL: grid_cell = grid_cell::new(
    utf8_data::new([b' '], 0, 1, 1),
    grid_attr::empty(),
    grid_flag::CLEARED,
    8,
    8,
    8,
    0,
);

pub static GRID_CLEARED_ENTRY: grid_cell_entry = grid_cell_entry {
    union_: grid_cell_entry_union {
        data: grid_cell_entry_data {
            attr: 0,
            fg: 8,
            bg: 8,
            data: b' ',
        },
    },
    flags: grid_flag::CLEARED,
};

/// Store cell in entry.
unsafe fn grid_store_cell(gce: *mut grid_cell_entry, gc: *const grid_cell, c: u8) {
    unsafe {
        (*gce).flags = (*gc).flags & !grid_flag::CLEARED;

        (*gce).union_.data.fg = ((*gc).fg & 0xff) as u8;
        if (*gc).fg & COLOUR_FLAG_256 != 0 {
            (*gce).flags |= grid_flag::FG256;
        }

        (*gce).union_.data.bg = ((*gc).bg & 0xff) as u8;
        if (*gc).bg & COLOUR_FLAG_256 != 0 {
            (*gce).flags |= grid_flag::BG256;
        }

        (*gce).union_.data.attr = (*gc).attr.bits() as u8;
        (*gce).union_.data.data = c;
    }
}

/// Check if a cell should be an extended cell.
unsafe fn grid_need_extended_cell(gce: *const grid_cell_entry, gc: *const grid_cell) -> bool {
    unsafe {
        if (*gce).flags.contains(grid_flag::EXTENDED) {
            return true;
        }
        if (*gc).attr.bits() > 0xff {
            return true;
        }
        if (*gc).data.size != 1 || (*gc).data.width != 1 {
            return true;
        }
        if ((*gc).fg & COLOUR_FLAG_RGB != 0) || ((*gc).bg & COLOUR_FLAG_RGB != 0) {
            return true;
        }
        if (*gc).us != 8 {
            // only supports 256 or RGB
            return true;
        }
        if (*gc).link != 0 {
            return true;
        }
        false
    }
}

/// Get an extended cell.
unsafe fn grid_get_extended_cell(
    gl: *mut grid_line,
    gce: *mut grid_cell_entry,
    flags: grid_flag,
) {
    unsafe {
        (*gl).extddata.push(zeroed());
        let at = (*gl).extddata.len() as u32;

        (*gce).union_.offset = at - 1;
        (*gce).flags = flags | grid_flag::EXTENDED;
    }
}

/// Set cell as extended.
unsafe fn grid_extended_cell(
    gl: *mut grid_line,
    gce: *mut grid_cell_entry,
    gc: *const grid_cell,
) -> *mut grid_extd_entry {
    unsafe {
        let flags = (*gc).flags & !grid_flag::CLEARED;

        if !(*gce).flags.contains(grid_flag::EXTENDED) {
            grid_get_extended_cell(gl, gce, flags);
        } else if (*gce).union_.offset as usize >= (*gl).extddata.len() {
            fatalx("offset too big");
        }
        (*gl).flags |= grid_line_flag::EXTENDED;

        let mut uc = MaybeUninit::<utf8_char>::uninit();
        let uc = uc.as_mut_ptr();
        utf8_from_data(&raw const (*gc).data, uc);

        let gee = &mut (*gl).extddata.as_mut_slice()[(*gce).union_.offset as usize];
        gee.data = *uc;
        gee.attr = (*gc).attr.bits();
        gee.flags = flags.bits();
        gee.fg = (*gc).fg;
        gee.bg = (*gc).bg;
        gee.us = (*gc).us;
        gee.link = (*gc).link;
        gee
    }
}

/// Free up unused extended cells.
fn grid_compact_line(gl: &mut grid_line) {
    if gl.extddata.is_empty() {
        return;
    }

    // Count extended cells
    let new_extdsize = gl
        .celldata
        .iter()
        .filter(|gce| gce.flags.contains(grid_flag::EXTENDED))
        .count();

    if new_extdsize == 0 {
        gl.extddata.clear();
        return;
    }

    // Build new extddata, remapping offsets
    let mut new_extddata = Vec::with_capacity(new_extdsize);
    for gce in &mut gl.celldata {
        if gce.flags.contains(grid_flag::EXTENDED) {
            // SAFETY: union field read is safe when EXTENDED flag is set (the
            // entry is the `offset` variant).
            new_extddata.push(gl.extddata[unsafe { gce.union_.offset } as usize]);
            gce.union_.offset = (new_extddata.len() - 1) as u32;
        }
    }

    gl.extddata = new_extddata;
}

/// Copy default into a cell.
fn grid_clear_cell(gd: &mut grid, px: c_uint, py: c_uint, bg: c_uint) {
    let gl = &mut gd.linedata[py as usize];
    gl.celldata[px as usize] = GRID_CLEARED_ENTRY;
    if bg != 8 {
        // SAFETY: grid_get_extended_cell / grid_extended_cell take raw pointers
        // into the same grid_line; we hand them a pointer derived from the &mut
        // we already hold. No aliasing for the duration of the call.
        unsafe {
            let gl_ptr: *mut grid_line = gl;
            let gce = gl.celldata.as_mut_ptr().add(px as usize);
            if (bg & COLOUR_FLAG_RGB as u32) != 0 {
                grid_get_extended_cell(gl_ptr, gce, (*gce).flags);
                let gee = grid_extended_cell(gl_ptr, gce, &raw const GRID_CLEARED_CELL);
                (*gee).bg = bg as i32;
            } else {
                if (bg & COLOUR_FLAG_256 as u32) != 0 {
                    (*gce).flags |= grid_flag::BG256;
                }
                (*gce).union_.data.bg = bg as c_uchar;
            }
        }
    }
}

/// Check grid y position.
fn grid_check_y(gd: &grid, from: *const u8, py: c_uint) -> c_int {
    if py >= gd.hsize + gd.sy {
        // SAFETY: `from` is a NUL-terminated ASCII tag passed in from a c!()
        // literal; log_debug is the only safe consumer.
        unsafe { log_debug!("{}: y out of range: {}", _s(from), py) };
        return -1;
    }
    0
}

/// Check if two styles are (visibly) the same.
pub unsafe fn grid_cells_look_equal(gc1: *const grid_cell, gc2: *const grid_cell) -> c_int {
    unsafe {
        if (*gc1).fg != (*gc2).fg || (*gc1).bg != (*gc2).bg {
            return 0;
        }
        if (*gc1).attr != (*gc2).attr || (*gc1).flags != (*gc2).flags {
            return 0;
        }
        if (*gc1).link != (*gc2).link {
            return 0;
        }
        1
    }
}

/// Compare grid cells. Return 1 if equal, 0 if not.
pub unsafe fn grid_cells_equal(gc1: *const grid_cell, gc2: *const grid_cell) -> bool {
    unsafe {
        if grid_cells_look_equal(gc1, gc2) == 0 {
            return false;
        }
        if (*gc1).data.width != (*gc2).data.width {
            return false;
        }
        if (*gc1).data.size != (*gc2).data.size {
            return false;
        }
        libc::memcmp(
            (*gc1).data.data.as_ptr().cast(),
            (*gc2).data.data.as_ptr().cast(),
            (*gc1).data.size as usize,
        ) == 0
    }
}

/// Free one line.
fn grid_free_line(gd: &mut grid, py: c_uint) {
    let gl = &mut gd.linedata[py as usize];
    gl.celldata = Vec::new();
    gl.cellused = 0;
    gl.extddata = Vec::new();
}

/// Free several lines.
fn grid_free_lines(gd: &mut grid, py: c_uint, ny: c_uint) {
    for yy in py..(py + ny) {
        grid_free_line(gd, yy);
    }
}

/// Create a new grid.
pub fn grid_create(sx: u32, sy: u32, hlimit: u32) -> Box<grid> {
    let mut linedata = Vec::with_capacity(sy as usize);
    linedata.resize_with(sy as usize, grid_line::new);
    Box::new(grid {
        sx,
        sy,
        flags: if hlimit != 0 { GRID_HISTORY } else { 0 },
        hscrolled: 0,
        hsize: 0,
        hlimit,
        linedata,
    })
}

// grid_destroy removed — Grid is now Box<grid>, Drop handles cleanup.

/// Compare grids.
pub unsafe fn grid_compare(ga: *mut grid, gb: *mut grid) -> c_int {
    unsafe {
        if (*ga).sx != (*gb).sx || (*ga).sy != (*gb).sy {
            return 1;
        }

        for yy in 0..(*ga).sy {
            let gla = &(&(*ga).linedata)[yy as usize];
            let glb = &(&(*gb).linedata)[yy as usize];

            if gla.celldata.len() != glb.celldata.len() {
                return 1;
            }

            for xx in 0..gla.celldata.len() as u32 {
                let gca = (*ga).get_cell(xx, yy);
                let gcb = (*gb).get_cell(xx, yy);

                if !grid_cells_equal(&gca, &gcb) {
                    return 1;
                }
            }
        }

        0
    }
}

/// Trim lines from the history.
fn grid_trim_history(gd: &mut grid, ny: c_uint) {
    grid_free_lines(gd, 0, ny);
    // Remove the first `ny` lines (already freed above) by draining them.
    // drain(0..ny) shifts the remaining lines to the front.
    gd.linedata.drain(0..ny as usize);
}

/// Expand line to fit to cell.
fn grid_expand_line(gd: &mut grid, py: c_uint, mut sx: c_uint, bg: c_uint) {
    let old_len = gd.linedata[py as usize].celldata.len() as u32;
    if sx <= old_len {
        return;
    }

    if sx < gd.sx / 4 {
        sx = gd.sx / 4;
    } else if sx < gd.sx / 2 {
        sx = gd.sx / 2;
    } else if gd.sx > sx {
        sx = gd.sx;
    }

    gd.linedata[py as usize].celldata.resize(sx as usize, GRID_CLEARED_ENTRY);

    for xx in old_len..sx {
        grid_clear_cell(gd, xx, py, bg);
    }
}

/// Initialize a line slot without dropping (for after `ptr::copy` where
/// the old data was bitwise-moved to another location).
fn grid_init_line(gd: &mut grid, py: c_uint, bg: c_uint) {
    // Write a fresh line. The old value was bitwise-moved away, so we must
    // NOT drop it. Use ptr::write to avoid the implicit drop from assignment.
    // SAFETY: the caller guarantees that the slot at `py` holds bitwise-moved
    // data that must not be dropped.
    unsafe {
        let gl = gd.linedata.as_mut_ptr().add(py as usize);
        std::ptr::write(gl, grid_line::new());
    }
    if !COLOUR_DEFAULT(bg as i32) {
        let sx = gd.sx;
        grid_expand_line(gd, py, sx, bg);
    }
}

/// Get cell from line.
unsafe fn grid_get_cell1(gl: &grid_line, px: c_uint, gc: *mut grid_cell) {
    unsafe {
        let gce = &gl.celldata[px as usize];

        if gce.flags.contains(grid_flag::EXTENDED) {
            if (gce.union_.offset as usize) >= gl.extddata.len() {
                std::ptr::copy(&GRID_DEFAULT_CELL, gc, 1);
            } else {
                let gee = &gl.extddata[gce.union_.offset as usize];
                (*gc).flags = grid_flag::from_bits(gee.flags).unwrap();
                (*gc).attr = grid_attr::from_bits(gee.attr).expect("invalid grid_attr");
                (*gc).fg = gee.fg;
                (*gc).bg = gee.bg;
                (*gc).us = gee.us;
                (*gc).link = gee.link;
                (*gc).data = utf8_to_data(gee.data);
            }
            return;
        }

        (*gc).flags = gce.flags & !(grid_flag::FG256 | grid_flag::BG256);
        (*gc).attr = grid_attr::from_bits(gce.union_.data.attr as u16).unwrap();
        (*gc).fg = gce.union_.data.fg as i32;
        if gce.flags.contains(grid_flag::FG256) {
            (*gc).fg |= COLOUR_FLAG_256;
        }
        (*gc).bg = gce.union_.data.bg as i32;
        if gce.flags.contains(grid_flag::BG256) {
            (*gc).bg |= COLOUR_FLAG_256;
        }
        (*gc).us = 8;
        utf8_set(&mut (*gc).data, gce.union_.data.data);
        (*gc).link = 0;
    }
}

impl grid {
    /// Get line data (mutable reference).
    pub fn get_line(&mut self, line: c_uint) -> &mut grid_line {
        &mut self.linedata[line as usize]
    }

    /// Adjust number of lines.
    pub fn adjust_lines(&mut self, lines: c_uint) {
        self.linedata.resize_with(lines as usize, grid_line::new);
    }

    /// Peek at grid line — returns null if py is out of range.
    pub fn peek_line(&mut self, py: c_uint) -> *mut grid_line {
        if grid_check_y(self, c!("grid_peek_line"), py) != 0 {
            return null_mut();
        }
        &raw mut self.linedata[py as usize]
    }

    /// Return the length of a line (position past last non-space cell).
    pub fn line_length(&mut self, py: u32) -> u32 {
        let mut px = self.get_line(py).celldata.len() as u32;
        if px > self.sx {
            px = self.sx;
        }
        while px > 0 {
            let gc = self.get_cell(px - 1, py);
            if (gc.flags.intersects(grid_flag::PADDING))
                || gc.data.size != 1
                || gc.data.data[0] != b' '
            {
                break;
            }
            px -= 1;
        }
        px
    }

    /// Get cell at position `(px, py)`. Returns `GRID_DEFAULT_CELL` if the
    /// position is out of range.
    pub fn get_cell(&self, px: c_uint, py: c_uint) -> grid_cell {
        if grid_check_y(self, c!("grid_get_cell"), py) != 0
            || px as usize >= self.linedata[py as usize].celldata.len()
        {
            GRID_DEFAULT_CELL
        } else {
            let mut gc: grid_cell = GRID_DEFAULT_CELL;
            unsafe {
                grid_get_cell1(&self.linedata[py as usize], px, &raw mut gc);
            }
            gc
        }
    }

    /// Set cell at position.
    pub fn set_cell(&mut self, px: c_uint, py: c_uint, gc: &grid_cell) {
        if grid_check_y(self, c!("grid_set_cell"), py) != 0 {
            return;
        }

        grid_expand_line(self, py, px + 1, 8);

        let gl = &mut self.linedata[py as usize];
        if px + 1 > gl.cellused {
            gl.cellused = px + 1;
        }

        let gc_ptr: *const grid_cell = gc;
        let gce = &mut gl.celldata[px as usize] as *mut grid_cell_entry;
        unsafe {
            if grid_need_extended_cell(gce, gc_ptr) {
                grid_extended_cell(gl, gce, gc_ptr);
            } else {
                grid_store_cell(gce, gc_ptr, gc.data.data[0]);
            }
        }
    }

    /// Set padding at position.
    pub fn set_padding(&mut self, px: c_uint, py: c_uint) {
        self.set_cell(px, py, &GRID_PADDING_CELL)
    }

    /// Set cells at position.
    pub unsafe fn set_cells(
        &mut self,
        px: u32,
        py: u32,
        gc: *const grid_cell,
        s: *const u8,
        slen: usize,
    ) {
        unsafe {
            if grid_check_y(self, c!("grid_set_cells"), py) != 0 {
                return;
            }

            grid_expand_line(self, py, px + slen as c_uint, 8);

            let gl = self.linedata.as_mut_ptr().add(py as usize);
            if px + slen as c_uint > (*gl).cellused {
                (*gl).cellused = px + slen as c_uint;
            }

            for i in 0..slen {
                let gce = (*gl).celldata.as_mut_ptr().add((px + i as c_uint) as usize);
                if grid_need_extended_cell(gce, gc) {
                    let gee = grid_extended_cell(gl, gce, gc);
                    (*gee).data = utf8_build_one(*s.add(i));
                } else {
                    grid_store_cell(gce, gc, *s.add(i));
                }
            }
        }
    }

    /// Collect lines from the history if at the limit. Free the top (oldest) 10% and shift up.
    pub fn collect_history(&mut self) {
        if self.hsize == 0 || self.hsize < self.hlimit {
            return;
        }

        let mut ny = self.hlimit / 10;
        if ny < 1 {
            ny = 1;
        }
        if ny > self.hsize {
            ny = self.hsize;
        }

        grid_trim_history(self, ny);

        self.hsize -= ny;
        if self.hscrolled > self.hsize {
            self.hscrolled = self.hsize;
        }
    }

    /// Remove lines from the bottom of the history.
    pub fn remove_history(&mut self, ny: c_uint) {
        if ny > self.hsize {
            return;
        }
        for yy in 0..ny {
            grid_free_line(self, self.hsize + self.sy - 1 - yy);
        }
        self.hsize -= ny;
    }

    /// Scroll the entire visible screen, moving one line into the history.
    pub fn scroll_history(&mut self, bg: c_uint) {
        let yy = self.hsize + self.sy;
        self.linedata.push(grid_line::new());

        self.empty_line(yy, bg);

        self.hscrolled += 1;
        let hsize = self.hsize as usize;
        let gl = &mut self.linedata[hsize];
        grid_compact_line(gl);
        // SAFETY: CURRENT_TIME is a process-global time_t updated once per
        // event-loop tick; a racy read of a POD is fine here.
        gl.time = unsafe { CURRENT_TIME };
        self.hsize += 1;
    }

    /// Clear the history.
    pub fn clear_history(&mut self) {
        grid_trim_history(self, self.hsize);

        self.hscrolled = 0;
        self.hsize = 0;

        self.linedata.resize_with(self.sy as usize, grid_line::new);
    }

    /// Scroll a region up, moving the top line into the history.
    pub fn scroll_history_region(&mut self, upper: c_uint, lower: c_uint, bg: c_uint) {
        // Indices are relative to the visible screen; adjust for hsize.
        let hsize = self.hsize as usize;
        let upper_abs = hsize + upper as usize;
        let lower_abs = hsize + lower as usize;

        // Remove the upper line from its position (this shifts everything above down).
        let upper_line = self.linedata.remove(upper_abs);

        // Insert it at the history position (hsize), pushing visible lines down.
        self.linedata.insert(hsize, upper_line);
        // SAFETY: see scroll_history for CURRENT_TIME rationale.
        self.linedata[hsize].time = unsafe { CURRENT_TIME };

        // The region shifted up by one. Insert a new empty line at the lower position.
        self.linedata.insert(lower_abs + 1, grid_line::new());
        if !COLOUR_DEFAULT(bg as i32) {
            let sx = self.sx;
            grid_expand_line(self, (lower_abs + 1) as u32, sx, bg);
        }

        // Move history offset down
        self.hscrolled += 1;
        self.hsize += 1;
    }

    /// Clear a rectangular area.
    pub fn clear(&mut self, px: c_uint, py: c_uint, nx: c_uint, ny: c_uint, bg: c_uint) {
        if nx == 0 || ny == 0 {
            return;
        }

        if px == 0 && nx == self.sx {
            self.clear_lines(py, ny, bg);
            return;
        }

        if grid_check_y(self, c!("grid_clear"), py) != 0 {
            return;
        }
        if grid_check_y(self, c!("grid_clear"), py + ny - 1) != 0 {
            return;
        }

        for yy in py..py + ny {
            let mut sx = self.sx;
            let celldata_len = self.linedata[yy as usize].celldata.len() as u32;
            if sx > celldata_len {
                sx = celldata_len;
            }
            let mut ox = nx;
            if COLOUR_DEFAULT(bg as i32) {
                if px > sx {
                    continue;
                }
                if px + nx > sx {
                    ox = sx - px;
                }
            }

            grid_expand_line(self, yy, px + ox, 8); // default bg first
            for xx in px..px + ox {
                grid_clear_cell(self, xx, yy, bg);
            }
        }
    }

    /// Clear a range of lines. Frees and truncates them.
    pub fn clear_lines(&mut self, py: c_uint, ny: c_uint, bg: c_uint) {
        if ny == 0 {
            return;
        }

        if grid_check_y(self, c!("grid_clear_lines"), py) != 0 {
            return;
        }
        if grid_check_y(self, c!("grid_clear_lines"), py + ny - 1) != 0 {
            return;
        }

        for yy in py..py + ny {
            grid_free_line(self, yy);
            self.empty_line(yy, bg);
        }
        if py != 0 {
            self.linedata[py as usize - 1].flags &= !grid_line_flag::WRAPPED;
        }
    }

    /// Move a group of lines.
    pub unsafe fn move_lines(&mut self, dy: c_uint, py: c_uint, ny: c_uint, bg: c_uint) {
        unsafe {
            if ny == 0 || py == dy {
                return;
            }

            if grid_check_y(self, c!("grid_move_lines"), py) != 0 {
                return;
            }
            if grid_check_y(self, c!("grid_move_lines"), py + ny - 1) != 0 {
                return;
            }
            if grid_check_y(self, c!("grid_move_lines"), dy) != 0 {
                return;
            }
            if grid_check_y(self, c!("grid_move_lines"), dy + ny - 1) != 0 {
                return;
            }

            // Free any lines which are being replaced
            for yy in dy..dy + ny {
                if yy >= py && yy < py + ny {
                    continue;
                }
                grid_free_line(self, yy);
            }
            if dy != 0 {
                self.linedata[dy as usize - 1].flags &= !grid_line_flag::WRAPPED;
            }

            // Move the lines (memmove semantics — handles overlap).
            // Can't use copy_within because grid_line is not Copy (contains Vec).
            let src = self.linedata.as_mut_ptr().add(py as usize);
            let dst = self.linedata.as_mut_ptr().add(dy as usize);
            std::ptr::copy(src, dst, ny as usize);

            // Wipe source lines that are outside the destination range.
            // These were bitwise-moved, so DON'T drop their Vec fields — the data
            // is now owned by the destination. Use grid_init_line (no drop).
            for yy in py..py + ny {
                if yy < dy || yy >= dy + ny {
                    grid_init_line(self, yy, bg);
                }
            }
            if py != 0 && (py < dy || py >= dy + ny) {
                self.linedata[py as usize - 1].flags &= !grid_line_flag::WRAPPED;
            }
        }
    }

    /// Move a group of cells within a line.
    pub fn move_cells(&mut self, dx: c_uint, px: c_uint, py: c_uint, nx: c_uint, bg: c_uint) {
        if nx == 0 || px == dx {
            return;
        }

        if grid_check_y(self, c!("grid_move_cells"), py) != 0 {
            return;
        }
        grid_expand_line(self, py, px + nx, 8);
        grid_expand_line(self, py, dx + nx, 8);

        let gl = &mut self.linedata[py as usize];
        gl.celldata.copy_within(px as usize..(px + nx) as usize, dx as usize);

        if dx + nx > gl.cellused {
            gl.cellused = dx + nx;
        }

        // Wipe any cells that have been moved
        for xx in px..px + nx {
            if xx >= dx && xx < dx + nx {
                continue;
            }
            grid_clear_cell(self, xx, py, bg);
        }
    }

    /// Empty a line and optionally fill with a background color.
    pub fn empty_line(&mut self, py: c_uint, bg: c_uint) {
        self.linedata[py as usize] = grid_line::new();
        if !COLOUR_DEFAULT(bg as i32) {
            let sx = self.sx;
            grid_expand_line(self, py, sx, bg);
        }
    }

    /// Convert cells into a string.
    pub unsafe fn string_cells(
        &mut self,
        px: c_uint,
        py: c_uint,
        nx: c_uint,
        lastgc: *mut *mut grid_cell,
        flags: grid_string_flags,
        s: *mut screen,
    ) -> *mut u8 {
        static mut LASTGC1: grid_cell = unsafe { zeroed() };
        unsafe {
            let mut gc: grid_cell;
            let mut data: *const u8;
            let mut code: [u8; 8192] = [0; 8192];
            let mut len: usize = 128;
            let mut off: usize = 0;
            let mut size: usize = 0;
            let mut codelen: usize;
            let mut has_link: bool = false;

            if !lastgc.is_null() && (*lastgc).is_null() {
                std::ptr::copy(&GRID_DEFAULT_CELL, &raw mut LASTGC1, 1);
                *lastgc = &raw mut LASTGC1;
            }

            let mut buf: *mut u8 = xmalloc(len).as_ptr() as *mut u8;

            let gl = self.peek_line(py);
            let end = if flags.intersects(grid_string_flags::GRID_STRING_EMPTY_CELLS) {
                (*gl).celldata.len() as u32
            } else {
                (*gl).cellused
            };

            for xx in px..px + nx {
                if gl.is_null() || xx >= end {
                    break;
                }
                gc = self.get_cell(xx, py);
                if gc.flags.intersects(grid_flag::PADDING) {
                    continue;
                }

                if flags.intersects(grid_string_flags::GRID_STRING_WITH_SEQUENCES) {
                    has_link = grid_string_cells_code(
                        *lastgc,
                        &gc,
                        code.as_mut_ptr(),
                        code.len(),
                        flags,
                        s
                    );
                    codelen = strlen(code.as_ptr());
                    std::ptr::copy(&gc, *lastgc, 1);
                } else {
                    codelen = 0;
                }

                data = &raw const gc.data.data as *const u8;
                size = gc.data.size as usize;
                if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES)
                    && size == 1
                    && *data == b'\\'
                {
                    data = c!("\\\\");
                    size = 2;
                }

                while len < off + size + codelen + 1 {
                    buf = xreallocarray(buf.cast(), 2, len).as_ptr() as *mut u8;
                    len *= 2;
                }

                if codelen != 0 {
                    std::ptr::copy(code.as_ptr(), buf.add(off), codelen);
                    off += codelen;
                }
                std::ptr::copy(data, buf.add(off), size);
                off += size;
            }

            if has_link {
                grid_string_cells_add_hyperlink(code.as_mut_ptr(), code.len(), c!(""), c!(""), flags);
                codelen = strlen(code.as_ptr());
                while len < off + size + codelen + 1 {
                    buf = xreallocarray(buf.cast(), 2, len).as_ptr() as *mut u8;
                    len *= 2;
                }
                std::ptr::copy(code.as_ptr(), buf.add(off), codelen);
                off += codelen;
            }

            if flags.intersects(grid_string_flags::GRID_STRING_TRIM_SPACES) {
                while off > 0 && *buf.add(off - 1) as u8 == b' ' {
                    off -= 1;
                }
            }
            *buf.add(off) = 0;

            buf
        }
    }

    /// Duplicate a set of lines from `src` into `self` (destination).
    /// Both source and destination should be big enough.
    pub unsafe fn duplicate_lines(
        &mut self,
        mut dy: c_uint,
        src: *mut grid,
        mut sy: c_uint,
        mut ny: c_uint,
    ) {
        unsafe {
            if dy + ny > self.hsize + self.sy {
                ny = self.hsize + self.sy - dy;
            }
            if sy + ny > (*src).hsize + (*src).sy {
                ny = (*src).hsize + (*src).sy - sy;
            }
            grid_free_lines(self, dy, ny);

            for _ in 0..ny {
                let srcl = &(&(*src).linedata)[sy as usize];
                let dstl = &mut self.linedata[dy as usize];

                dstl.celldata = srcl.celldata.clone();
                dstl.cellused = srcl.cellused;
                dstl.extddata = srcl.extddata.clone();
                dstl.flags = srcl.flags;
                dstl.time = srcl.time;

                sy += 1;
                dy += 1;
            }
        }
    }

    /// Reflow lines on grid to new width.
    pub unsafe fn reflow(&mut self, sx: u32) {
        unsafe {
            let gd = self as *mut grid;
            // Create destination grid - just used as container for line data
            let mut target = grid_create(self.sx, 0, 0);
            let target_ptr = &raw mut *target;

            // Loop over each source line
            for yy in 0..(self.hsize + self.sy) {
                let gl = self.linedata.as_mut_ptr().add(yy as usize);
                if (*gl).flags.intersects(grid_line_flag::DEAD) {
                    continue;
                }

                // Work out width of this line. at is point where available width is hit,
                // width is full line width
                let mut at = 0;
                let mut width = 0;
                let mut gc = zeroed();

                if !(*gl).flags.intersects(grid_line_flag::EXTENDED) {
                    width = (*gl).cellused;
                    if width > sx {
                        at = sx;
                    } else {
                        at = width;
                    }
                } else {
                    for i in 0..(*gl).cellused {
                        grid_get_cell1(&*gl, i, &mut gc);
                        if at == 0 && width + gc.data.width as u32 > sx {
                            at = i;
                        }
                        width += gc.data.width as u32;
                    }
                }

                // If line exactly right, move across unchanged
                if width == sx {
                    grid_reflow_move(target_ptr, gl);
                    continue;
                }

                // If line too big, needs to be split
                if width > sx {
                    grid_reflow_split(target_ptr, gd, sx, yy, at);
                    continue;
                }

                // If line was previously wrapped, join as much as possible of next line
                if (*gl).flags.intersects(grid_line_flag::WRAPPED) {
                    grid_reflow_join(target_ptr, gd, sx, yy, width, 0);
                } else {
                    grid_reflow_move(target_ptr, gl);
                }
            }

            // Replace old grid with new
            if target.sy < self.sy {
                grid_reflow_add(target_ptr, self.sy - target.sy);
            }
            self.hsize = target.sy - self.sy;
            if self.hscrolled > self.hsize {
                self.hscrolled = self.hsize;
            }
            // Swap linedata: old Vec drops automatically, take target's.
            self.linedata = std::mem::take(&mut target.linedata);
            target.sy = 0;
            // target is now an empty grid — Box drop handles cleanup.
            drop(target);
        }
    }

    /// Convert grid position `(px, py)` to wrapped-line position `(wx, wy)`.
    ///
    /// Collapses contiguous runs of lines that have `grid_line_flag::WRAPPED`
    /// on the previous line into a single logical line: `wy` counts the
    /// number of unwrapped ("visual") lines above `py`, and `wx` is the
    /// column within that logical line. If `px` is past the end of the
    /// line's used cells, `wx` is returned as `u32::MAX`.
    pub fn wrap_position(&self, px: u32, py: u32) -> (u32, u32) {
        let mut ax = 0;
        let mut ay = 0;

        for yy in 0..py as usize {
            if self.linedata[yy].flags.intersects(grid_line_flag::WRAPPED) {
                ax += self.linedata[yy].cellused;
            } else {
                ax = 0;
                ay += 1;
            }
        }

        let wx = if px >= self.linedata[py as usize].cellused {
            u32::MAX
        } else {
            ax + px
        };
        (wx, ay)
    }

    /// Convert wrapped-line position `(wx, wy)` back to grid position
    /// `(px, py)`.
    ///
    /// Inverse of [`wrap_position`](Self::wrap_position). `wx == u32::MAX`
    /// means "end of the logical line" and is resolved to the end of the
    /// last wrapped segment.
    pub fn unwrap_position(&self, mut wx: u32, wy: u32) -> (u32, u32) {
        let mut ay = 0;
        let mut yy: usize = 0;

        while yy < (self.hsize + self.sy - 1) as usize {
            if ay == wy {
                break;
            }
            if !self.linedata[yy].flags.intersects(grid_line_flag::WRAPPED) {
                ay += 1;
            }
            yy += 1;
        }

        // yy is now 0 on unwrapped line containing wx
        // Walk forwards until we find end or line now containing wx
        if wx == u32::MAX {
            while self.linedata[yy].flags.intersects(grid_line_flag::WRAPPED) {
                yy += 1;
            }
            wx = self.linedata[yy].cellused;
        } else {
            while self.linedata[yy].flags.intersects(grid_line_flag::WRAPPED) {
                if wx < self.linedata[yy].cellused {
                    break;
                }
                wx -= self.linedata[yy].cellused;
                yy += 1;
            }
        }
        (wx, yy as u32)
    }

    // ---------------------------------------------------------------
    // View-coordinate helpers.
    //
    // "View" coordinates treat y=0 as the top of the visible screen.
    // Converting to absolute grid coordinates means adding `hsize`
    // (the number of scrollback history lines). The x translation is
    // currently the identity, so we simply inline `self.hsize + py`.
    // ---------------------------------------------------------------

    /// Get a cell from the visible area at view coordinates (px, py).
    pub fn view_get_cell(&self, px: u32, py: u32) -> grid_cell {
        self.get_cell(px, self.hsize + py)
    }

    /// Set a cell in the visible area at view coordinates (px, py).
    pub fn view_set_cell(&mut self, px: u32, py: u32, gc: &grid_cell) {
        self.set_cell(px, self.hsize + py, gc);
    }

    /// Mark a cell in the visible area as padding (following a wide char).
    pub fn view_set_padding(&mut self, px: u32, py: u32) {
        self.set_padding(px, self.hsize + py);
    }

    /// Set a run of cells in the visible area starting at (px, py).
    pub unsafe fn view_set_cells(
        &mut self,
        px: u32,
        py: u32,
        gc: *const grid_cell,
        s: *const u8,
        slen: usize,
    ) {
        unsafe {
            self.set_cells(px, self.hsize + py, gc, s, slen);
        }
    }

    /// Move all visible content into history and clear the screen.
    /// Only moves lines up to the last non-empty line.
    pub fn view_clear_history(&mut self, bg: u32) {
        let mut last = 0u32;

        for yy in 0..self.sy {
            let gl = self.get_line(self.hsize + yy);
            if gl.cellused != 0 {
                last = yy + 1;
            }
        }
        if last == 0 {
            self.view_clear(0, 0, self.sx, self.sy, bg);
            return;
        }

        for _ in 0..self.sy {
            self.collect_history();
            self.scroll_history(bg);
        }
        if last < self.sy {
            self.view_clear(0, 0, self.sx, self.sy - last, bg);
        }
        self.hscrolled = 0;
    }

    /// Clear a rectangular region in view coordinates.
    pub fn view_clear(&mut self, px: u32, py: u32, nx: u32, ny: u32, bg: u32) {
        self.clear(px, self.hsize + py, nx, ny, bg);
    }

    /// Scroll a region upward: contents of `[rupper, rlower]` move up by one line.
    pub unsafe fn view_scroll_region_up(&mut self, rupper: u32, rlower: u32, bg: u32) {
        unsafe {
            if self.flags & GRID_HISTORY != 0 {
                self.collect_history();
                if rupper == 0 && rlower == self.sy - 1 {
                    self.scroll_history(bg);
                } else {
                    let rupper_abs = self.hsize + rupper;
                    let rlower_abs = self.hsize + rlower;
                    self.scroll_history_region(rupper_abs, rlower_abs, bg);
                }
            } else {
                let rupper_abs = self.hsize + rupper;
                let rlower_abs = self.hsize + rlower;
                self.move_lines(rupper_abs, rupper_abs + 1, rlower_abs - rupper_abs, bg);
            }
        }
    }

    /// Scroll a region downward: contents of `[rupper, rlower]` move down by one line.
    pub unsafe fn view_scroll_region_down(&mut self, rupper: u32, rlower: u32, bg: u32) {
        unsafe {
            let rupper_abs = self.hsize + rupper;
            let rlower_abs = self.hsize + rlower;
            self.move_lines(rupper_abs + 1, rupper_abs, rlower_abs - rupper_abs, bg);
        }
    }

    /// Insert `ny` blank lines at view row `py`.
    pub unsafe fn view_insert_lines(&mut self, py: u32, ny: u32, bg: u32) {
        unsafe {
            let py_abs = self.hsize + py;
            let sy = self.hsize + self.sy;
            self.move_lines(py_abs + ny, py_abs, sy - py_abs - ny, bg);
        }
    }

    /// Insert `ny` blank lines at view row `py` inside scroll region bounded by `rlower`.
    pub unsafe fn view_insert_lines_region(&mut self, rlower: u32, py: u32, ny: u32, bg: u32) {
        unsafe {
            let rlower_abs = self.hsize + rlower;
            let py_abs = self.hsize + py;

            let ny2 = rlower_abs + 1 - py_abs - ny;
            self.move_lines(rlower_abs + 1 - ny2, py_abs, ny2, bg);
            // TODO does this bug exist upstream?
            self.clear(0, py_abs + ny2, self.sx, ny.saturating_sub(ny2), bg);
        }
    }

    /// Delete `ny` lines at view row `py`.
    pub unsafe fn view_delete_lines(&mut self, py: u32, ny: u32, bg: u32) {
        unsafe {
            let py_abs = self.hsize + py;
            let sy = self.hsize + self.sy;

            self.move_lines(py_abs, py_abs + ny, sy - py_abs - ny, bg);
            self.clear(0, sy.saturating_sub(ny), self.sx, ny, bg);
        }
    }

    /// Delete `ny` lines at view row `py` inside scroll region bounded by `rlower`.
    pub unsafe fn view_delete_lines_region(&mut self, rlower: u32, py: u32, ny: u32, bg: u32) {
        unsafe {
            let rlower_abs = self.hsize + rlower;
            let py_abs = self.hsize + py;

            let ny2 = rlower_abs + 1 - py_abs - ny;
            self.move_lines(py_abs, py_abs + ny, ny2, bg);
            // TODO does this bug exist in the tmux source code too
            self.clear(0, py_abs + ny2, self.sx, ny.saturating_sub(ny2), bg);
        }
    }

    /// Insert `nx` blank cells at view position (px, py).
    pub fn view_insert_cells(&mut self, px: u32, py: u32, nx: u32, bg: u32) {
        let py_abs = self.hsize + py;
        let sx = self.sx;

        if px >= sx - 1 {
            self.clear(px, py_abs, 1, 1, bg);
        } else {
            self.move_cells(px + nx, px, py_abs, sx - px - nx, bg);
        }
    }

    /// Delete `nx` cells at view position (px, py).
    pub fn view_delete_cells(&mut self, px: u32, py: u32, nx: u32, bg: u32) {
        let py_abs = self.hsize + py;
        let sx = self.sx;

        self.move_cells(px, px + nx, py_abs, sx - px - nx, bg);
        self.clear(sx - nx, py_abs, nx, 1, bg);
    }

    /// Convert `nx` cells in the visible area starting at (px, py) into a string.
    pub unsafe fn view_string_cells(&mut self, px: u32, py: u32, nx: u32) -> *mut u8 {
        unsafe {
            let py_abs = self.hsize + py;
            self.string_cells(
                px,
                py_abs,
                nx,
                null_mut(),
                grid_string_flags::empty(),
                null_mut(),
            )
        }
    }
}


/// Get ANSI foreground sequence.
unsafe fn grid_string_cells_fg(gc: *const grid_cell, values: *mut c_int) -> usize {
    unsafe {
        let mut n: usize = 0;

        if (*gc).fg & COLOUR_FLAG_256 != 0 {
            *values.add(n) = 38;
            n += 1;
            *values.add(n) = 5;
            n += 1;
            *values.add(n) = ((*gc).fg & 0xff) as c_int;
            n += 1;
        } else if (*gc).fg & COLOUR_FLAG_RGB != 0 {
            *values.add(n) = 38;
            n += 1;
            *values.add(n) = 2;
            n += 1;
            let (r, g, b) = colour_split_rgb((*gc).fg);
            *values.add(n) = r as c_int;
            n += 1;
            *values.add(n) = g as c_int;
            n += 1;
            *values.add(n) = b as c_int;
            n += 1;
        } else {
            match (*gc).fg {
                0..=7 => {
                    *values.add(n) = (*gc).fg + 30;
                    n += 1;
                }
                8 => {
                    *values.add(n) = 39;
                    n += 1;
                }
                90..=97 => {
                    *values.add(n) = (*gc).fg;
                    n += 1;
                }
                _ => {}
            }
        }
        n
    }
}

/// Get ANSI background sequence.
unsafe fn grid_string_cells_bg(gc: *const grid_cell, values: *mut c_int) -> usize {
    unsafe {
        let mut n: usize = 0;

        if (*gc).bg & COLOUR_FLAG_256 != 0 {
            *values.add(n) = 48;
            n += 1;
            *values.add(n) = 5;
            n += 1;
            *values.add(n) = ((*gc).bg & 0xff) as c_int;
            n += 1;
        } else if (*gc).bg & COLOUR_FLAG_RGB != 0 {
            *values.add(n) = 48;
            n += 1;
            *values.add(n) = 2;
            n += 1;
            let (r, g, b) = colour_split_rgb((*gc).bg);
            *values.add(n) = r as c_int;
            n += 1;
            *values.add(n) = g as c_int;
            n += 1;
            *values.add(n) = b as c_int;
            n += 1;
        } else {
            match (*gc).bg {
                0..=7 => {
                    *values.add(n) = (*gc).bg + 40;
                    n += 1;
                }
                8 => {
                    *values.add(n) = 49;
                    n += 1;
                }
                90..=97 => {
                    *values.add(n) = (*gc).bg + 10;
                    n += 1;
                }
                _ => {}
            }
        }
        n
    }
}

/// Get underscore colour sequence.
unsafe fn grid_string_cells_us(gc: *const grid_cell, values: *mut c_int) -> usize {
    unsafe {
        let mut n: usize = 0;
        if (*gc).us & COLOUR_FLAG_256 != 0 {
            *values.add(n) = 58;
            n += 1;
            *values.add(n) = 5;
            n += 1;
            *values.add(n) = ((*gc).us & 0xff) as c_int;
            n += 1;
        } else if (*gc).us & COLOUR_FLAG_RGB != 0 {
            *values.add(n) = 58;
            n += 1;
            *values.add(n) = 2;
            n += 1;
            let (r, g, b) = colour_split_rgb((*gc).us);
            *values.add(n) = r as c_int;
            n += 1;
            *values.add(n) = g as c_int;
            n += 1;
            *values.add(n) = b as c_int;
            n += 1;
        }
        n
    }
}

/// Add on SGR code.
unsafe fn grid_string_cells_add_code(
    buf: *mut u8,
    len: usize,
    n: c_uint,
    s: *mut c_int,
    newc: *mut c_int,
    oldc: *mut c_int,
    nnewc: usize,
    noldc: usize,
    flags: grid_string_flags,
) {
    unsafe {
        let mut tmp: [u8; 64] = [0; 64];
        let reset = n != 0 && *s == 0;

        if nnewc == 0 {
            return; // no code to add
        }
        if !reset
            && nnewc == noldc
            && libc::memcmp(
                newc as *const c_void,
                oldc as *const c_void,
                nnewc * std::mem::size_of::<c_int>(),
            ) == 0
        {
            return; // no reset and colour unchanged
        }
        if reset && (*newc == 49 || *newc == 39) {
            return; // reset and colour default
        }

        if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
            strlcat(buf, c!("\\033["), len);
        } else {
            strlcat(buf, c!("\x1b["), len);
        }

        for i in 0..nnewc {
            if i + 1 < nnewc {
                _ = xsnprintf_!(tmp.as_mut_ptr(), tmp.len(), "{};", *newc.add(i));
            } else {
                _ = xsnprintf_!(tmp.as_mut_ptr(), tmp.len(), "{}", *newc.add(i));
            }
            strlcat(buf, tmp.as_ptr(), len);
        }
        strlcat(buf, c!("m"), len);
    }
}

unsafe fn grid_string_cells_add_hyperlink(
    buf: *mut u8,
    len: usize,
    id: *const u8,
    uri: *const u8,
    flags: grid_string_flags,
) -> bool {
    unsafe {
        if strlen(uri) + strlen(id) + 17 >= len {
            return false;
        }

        if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
            strlcat(buf, c!("\\033]8;"), len);
        } else {
            strlcat(buf, c!("\x1b]8;"), len);
        }

        if *id != 0 {
            let tmp = format_nul!("id={};", _s(id));
            strlcat(buf, tmp, len);
            free_(tmp);
        } else {
            strlcat(buf, c!(";"), len);
        }

        strlcat(buf, uri, len);

        if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
            strlcat(buf, c!("\\033\\\\"), len);
        } else {
            strlcat(buf, c!("\x1b\\"), len);
        }

        true
    }
}

/// Returns ANSI code to set particular attributes (colour, bold and so on) given a current state.
unsafe fn grid_string_cells_code(
    lastgc: *const grid_cell,
    gc: *const grid_cell,
    buf: *mut u8,
    len: usize,
    flags: grid_string_flags,
    sc: *mut screen,
) -> bool {
    unsafe {
        let mut oldc: [c_int; 64] = [0; 64];
        let mut newc: [c_int; 64] = [0; 64];
        let mut s: [c_int; 128] = [0; 128];
        let mut noldc: usize;
        let mut nnewc: usize;
        let mut n: u32 = 0;
        let attr = (*gc).attr;
        let mut lastattr = (*lastgc).attr;
        let mut tmp: [u8; 64] = [0; 64];
        let mut uri: *const u8 = null();
        let mut id: *const u8 = null();
        let mut has_link = false;

        static ATTRS: [(grid_attr, c_uint); 13] = [
            (grid_attr::GRID_ATTR_BRIGHT, 1),
            (grid_attr::GRID_ATTR_DIM, 2),
            (grid_attr::GRID_ATTR_ITALICS, 3),
            (grid_attr::GRID_ATTR_UNDERSCORE, 4),
            (grid_attr::GRID_ATTR_BLINK, 5),
            (grid_attr::GRID_ATTR_REVERSE, 7),
            (grid_attr::GRID_ATTR_HIDDEN, 8),
            (grid_attr::GRID_ATTR_STRIKETHROUGH, 9),
            (grid_attr::GRID_ATTR_UNDERSCORE_2, 42),
            (grid_attr::GRID_ATTR_UNDERSCORE_3, 43),
            (grid_attr::GRID_ATTR_UNDERSCORE_4, 44),
            (grid_attr::GRID_ATTR_UNDERSCORE_5, 45),
            (grid_attr::GRID_ATTR_OVERLINE, 53),
        ];

        // If any attribute is removed, begin with 0
        for &(mask, _) in &ATTRS {
            if !attr.intersects(mask) && lastattr.intersects(mask)
                || ((*lastgc).us != 8 && (*gc).us == 8)
            {
                s[n as usize] = 0;
                n += 1;
                lastattr &= grid_attr::GRID_ATTR_CHARSET;
                break;
            }
        }

        // For each attribute that is newly set, add its code
        for &(mask, code) in &ATTRS {
            if attr.intersects(mask) && !lastattr.intersects(mask) {
                s[n as usize] = code as c_int;
                n += 1;
            }
        }

        // Write the attributes
        *buf = 0;
        if n > 0 {
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
                strlcat(buf, c!("\\033["), len);
            } else {
                strlcat(buf, c!("\x1b["), len);
            }

            for i in 0..n {
                if s[i as usize] < 10 {
                    _ = xsnprintf_!(tmp.as_mut_ptr(), tmp.len(), "{}", s[i as usize],);
                } else {
                    _ = xsnprintf_!(
                        tmp.as_mut_ptr(),
                        tmp.len(),
                        "{}:{}",
                        s[i as usize] / 10,
                        s[i as usize] % 10,
                    );
                }
                strlcat(buf, tmp.as_ptr(), len);
                if i + 1 < n {
                    strlcat(buf, c!(";"), len);
                }
            }
            strlcat(buf, c!("m"), len);
        }

        // If the foreground colour changed, write its parameters
        nnewc = grid_string_cells_fg(gc, newc.as_mut_ptr());
        noldc = grid_string_cells_fg(lastgc, oldc.as_mut_ptr());
        grid_string_cells_add_code(
            buf,
            len,
            n,
            s.as_mut_ptr(),
            newc.as_mut_ptr(),
            oldc.as_mut_ptr(),
            nnewc,
            noldc,
            flags,
        );

        // If the background colour changed, append its parameters
        nnewc = grid_string_cells_bg(gc, newc.as_mut_ptr());
        noldc = grid_string_cells_bg(lastgc, oldc.as_mut_ptr());
        grid_string_cells_add_code(
            buf,
            len,
            n,
            s.as_mut_ptr(),
            newc.as_mut_ptr(),
            oldc.as_mut_ptr(),
            nnewc,
            noldc,
            flags,
        );

        // If the underscore colour changed, append its parameters
        nnewc = grid_string_cells_us(gc, newc.as_mut_ptr());
        noldc = grid_string_cells_us(lastgc, oldc.as_mut_ptr());
        grid_string_cells_add_code(
            buf,
            len,
            n,
            s.as_mut_ptr(),
            newc.as_mut_ptr(),
            oldc.as_mut_ptr(),
            nnewc,
            noldc,
            flags,
        );

        // Append shift in/shift out if needed
        if attr.intersects(grid_attr::GRID_ATTR_CHARSET)
            && !lastattr.intersects(grid_attr::GRID_ATTR_CHARSET)
        {
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
                strlcat(buf, c!("\\016"), len); // SO
            } else {
                strlcat(buf, c!("\x0e"), len); // SO
            }
        }
        if !attr.intersects(grid_attr::GRID_ATTR_CHARSET)
            && lastattr.intersects(grid_attr::GRID_ATTR_CHARSET)
        {
            if flags.intersects(grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES) {
                strlcat(buf, c!("\\017"), len); // SI
            } else {
                strlcat(buf, c!("\x0f"), len); // SI
            }
        }

        // Add hyperlink if changed
        if !sc.is_null() && (*sc).hyperlinks.is_some() && (*lastgc).link != (*gc).link {
            if hyperlinks_get(
                (*sc).hyperlinks.unwrap_or(null_mut()),
                (*gc).link,
                &raw mut uri,
                &raw mut id,
                null_mut(),
            ) {
                has_link = grid_string_cells_add_hyperlink(buf, len, id, uri, flags);
            } else if has_link {
                grid_string_cells_add_hyperlink(buf, len, c!(""), c!(""), flags);
                has_link = false;
            }
        }
        has_link
    }
}

/// Mark line as dead. Caller must ensure the line's Vec fields have already
/// been moved out or dropped — this overwrites without dropping.
unsafe fn grid_reflow_dead(gl: *mut grid_line) {
    unsafe {
        std::ptr::write(gl, grid_line::new_dead());
    }
}

/// Add lines, return the first new one.
unsafe fn grid_reflow_add(gd: *mut grid, n: c_uint) -> *mut grid_line {
    unsafe {
        let sy = (*gd).sy + n;

        let old_sy = (*gd).sy as usize;
        (*gd).linedata.resize_with(sy as usize, grid_line::new);
        (*gd).sy = sy;
        (*gd).linedata.as_mut_ptr().add(old_sy)
    }
}

/// Move a line across.
unsafe fn grid_reflow_move(gd: *mut grid, from: *mut grid_line) -> *mut grid_line {
    unsafe {
        let to = grid_reflow_add(gd, 1);
        // Move the line value out of `from`, write it to `to`.
        // `to` was just initialized by grid_reflow_add — drop it before overwriting.
        std::ptr::drop_in_place(to);
        std::ptr::write(to, std::ptr::read(from));
        // Write a dead line over the now-empty source (no drop — data was moved).
        grid_reflow_dead(from);
        to
    }
}

/// Join line below onto this one.
unsafe fn grid_reflow_join(
    target: *mut grid,
    gd: *mut grid,
    sx: c_uint,
    yy: c_uint,
    mut width: c_uint,
    already: c_int,
) {
    unsafe {
        let mut from: *mut grid_line = null_mut();
        let mut gc = zeroed();
        let mut lines = 0;
        let mut wrapped = true;
        let mut want = 0;

        // Add a new target line
        let (to, gl) = if already == 0 {
            let to = (*target).sy;
            let gl = grid_reflow_move(target, (*gd).linedata.as_mut_ptr().add(yy as usize));
            (to, gl)
        } else {
            let to = (*target).sy - 1;
            let gl = (*target).linedata.as_mut_ptr().add(to as usize);
            (to, gl)
        };
        let mut at = (*gl).cellused;

        // Loop until no more to consume or target line is full
        loop {
            // If this is now the last line, nothing more to do
            if yy + 1 + lines == (*gd).hsize + (*gd).sy {
                break;
            }
            let line = yy + 1 + lines;

            // If next line is empty, skip it
            if !(&(*gd).linedata)[line as usize]
                .flags
                .intersects(grid_line_flag::WRAPPED)
            {
                wrapped = false;
            }
            if (&(*gd).linedata)[line as usize].cellused == 0 {
                if !wrapped {
                    break;
                }
                lines += 1;
                continue;
            }

            // Is destination line now full? Copy first char separately
            grid_get_cell1(&(&(*gd).linedata)[line as usize], 0, &mut gc);
            if width + gc.data.width as u32 > sx {
                break;
            }
            width += gc.data.width as u32;
            (*target).set_cell(at, to, &gc);
            at += 1;

            // Join as much more as possible onto current line
            from = (*gd).linedata.as_mut_ptr().add(line as usize);
            want = 1;
            while want < (*from).cellused {
                grid_get_cell1(&*from, want, &mut gc);
                if width + gc.data.width as u32 > sx {
                    break;
                }
                width += gc.data.width as u32;

                (*target).set_cell(at, to, &gc);
                at += 1;
                want += 1;
            }
            lines += 1;

            // If line wasn't wrapped or we didn't consume entire line,
            // don't try to join further lines
            if !wrapped || want != (*from).cellused || width == sx {
                break;
            }
        }
        if lines == 0 {
            return;
        }

        // If we didn't consume entire final line, remove what we did consume.
        // If we consumed entire line and it wasn't wrapped, remove wrap flag.
        let left = (*from).cellused - want;
        if left != 0 {
            (*gd).move_cells(0, want, yy + lines, left, 8);
            (*from).celldata.truncate(left as usize);
            (*from).cellused = left;
            lines -= 1;
        } else if !wrapped {
            (*gl).flags &= !grid_line_flag::WRAPPED;
        }

        // Remove lines that were completely consumed
        for i in (yy + 1)..(yy + 1 + lines) {
            let dead = (*gd).linedata.as_mut_ptr().add(i as usize);
            // Drop the line's Vec fields, then mark as dead.
            std::ptr::drop_in_place(dead);
            grid_reflow_dead(dead);
        }

        // Adjust scroll position
        if (*gd).hscrolled > to + lines {
            (*gd).hscrolled -= lines;
        } else if (*gd).hscrolled > to {
            (*gd).hscrolled = to;
        }
    }
}

/// Split this line into several new ones
unsafe fn grid_reflow_split(target: *mut grid, gd: *mut grid, sx: u32, yy: u32, at: u32) {
    unsafe {
        let gl = (*gd).linedata.as_mut_ptr().add(yy as usize);
        let mut gc = zeroed();
        let used = (*gl).cellused;
        let flags = (*gl).flags;

        // How many lines do we need to insert? We know we need at least two.
        let lines = if !(*gl).flags.intersects(grid_line_flag::EXTENDED) {
            1 + ((*gl).cellused - 1) / sx
        } else {
            let mut lines = 2;
            let mut width = 0;
            for i in at..used {
                grid_get_cell1(&*gl, i, &mut gc);
                if width + gc.data.width as u32 > sx {
                    lines += 1;
                    width = 0;
                }
                width += gc.data.width as u32;
            }
            lines
        };

        // Insert new lines
        let mut line = (*target).sy + 1;
        let first = grid_reflow_add(target, lines);

        // Copy sections from original line
        let mut width = 0;
        let mut xx = 0;
        for i in at..used {
            grid_get_cell1(&*gl, i, &raw mut gc);
            if width + gc.data.width as u32 > sx {
                (&mut (*target).linedata)[line as usize].flags |= grid_line_flag::WRAPPED;

                line += 1;
                width = 0;
                xx = 0;
            }
            width += gc.data.width as u32;
            (*target).set_cell(xx, line, &gc);
            xx += 1;
        }
        if flags.intersects(grid_line_flag::WRAPPED) {
            (&mut (*target).linedata)[line as usize].flags |= grid_line_flag::WRAPPED;
        }

        // Move remainder of original line
        (*gl).celldata.truncate(at as usize);
        (*gl).cellused = at;
        (*gl).flags |= grid_line_flag::WRAPPED;
        // Move the line value to `first`, then mark source as dead.
        std::ptr::drop_in_place(first);
        std::ptr::write(first, std::ptr::read(gl));
        grid_reflow_dead(gl);

        // Adjust scroll position
        if yy <= (*gd).hscrolled {
            (*gd).hscrolled += lines - 1;
        }

        // If original line had wrapped flag and there is still space in last new line,
        // try to join with next lines
        if width < sx && flags.intersects(grid_line_flag::WRAPPED) {
            grid_reflow_join(target, gd, sx, yy, width, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use std::ptr::null_mut;

    /// Helper: create a grid_cell with a single ASCII character.
    fn make_cell(ch: u8, fg: i32, bg: i32) -> grid_cell {
        grid_cell::new(
            utf8_data::new([ch], 0, 1, 1),
            grid_attr::empty(),
            grid_flag::empty(),
            fg,
            bg,
            8,
            0,
        )
    }

    // ---------------------------------------------------------------
    // Constructors / destructors
    // ---------------------------------------------------------------

    #[test]
    fn grid_create_returns_valid_grid() {
        let gd = grid_create(80, 24, 1000);
        assert_eq!(gd.sx, 80);
        assert_eq!(gd.sy, 24);
        assert_eq!(gd.hlimit, 1000);
        assert_eq!(gd.hsize, 0);
        drop(gd);
    }

    #[test]
    fn grid_create_zero_size() {
        let gd = grid_create(0, 0, 0);
        assert_eq!(gd.sx, 0);
        assert_eq!(gd.sy, 0);
        drop(gd);
    }

    #[test]
    fn grid_destroy_does_not_crash() {
        let gd = grid_create(10, 5, 0);
        drop(gd);
        // If we reach here, destroy did not crash.
    }

    // ---------------------------------------------------------------
    // Cell operations
    // ---------------------------------------------------------------

    #[test]
    fn grid_get_cell_on_fresh_grid_returns_default() {
        let gd = grid_create(80, 24, 0);
        unsafe {
            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            // Fresh grid returns GRID_DEFAULT_CELL (space character).
            assert!(grid_cells_equal(&gc, &GRID_DEFAULT_CELL));
            drop(gd);
        }
    }

    #[test]
    fn grid_set_cell_then_get_cell_roundtrip() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell_in = make_cell(b'A', 8, 8);
            gd.set_cell(5, 3, &cell_in);

            let cell_out: grid_cell;
            cell_out = gd.get_cell(5, 3);

            assert!(grid_cells_equal(&cell_in, &cell_out));
            drop(gd);
        }
    }

    #[test]
    fn grid_set_cell_multiple_positions() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell_a = make_cell(b'X', 8, 8);
            let cell_b = make_cell(b'Y', 8, 8);

            gd.set_cell(0, 0, &cell_a);
            gd.set_cell(1, 0, &cell_b);

            let out_a: grid_cell;
            let out_b: grid_cell;
            out_a = gd.get_cell(0, 0);
            out_b = gd.get_cell(1, 0);

            assert!(grid_cells_equal(&cell_a, &out_a));
            assert!(grid_cells_equal(&cell_b, &out_b));
            drop(gd);
        }
    }

    #[test]
    fn grid_cells_equal_identical() {
        let a = make_cell(b'Z', 8, 8);
        let b = make_cell(b'Z', 8, 8);
        assert!(unsafe { grid_cells_equal(&a, &b) });
    }

    #[test]
    fn grid_cells_equal_different_char() {
        let a = make_cell(b'A', 8, 8);
        let b = make_cell(b'B', 8, 8);
        assert!(!unsafe { grid_cells_equal(&a, &b) });
    }

    #[test]
    fn grid_cells_equal_different_fg() {
        let a = make_cell(b'A', 1, 8);
        let b = make_cell(b'A', 2, 8);
        assert!(!unsafe { grid_cells_equal(&a, &b) });
    }

    #[test]
    fn grid_cells_look_equal_same() {
        let a = make_cell(b'A', 8, 8);
        let b = make_cell(b'A', 8, 8);
        assert_eq!(unsafe { grid_cells_look_equal(&a, &b) }, 1);
    }

    #[test]
    fn grid_cells_look_equal_different_fg() {
        let a = make_cell(b'A', 1, 8);
        let b = make_cell(b'A', 2, 8);
        assert_eq!(unsafe { grid_cells_look_equal(&a, &b) }, 0);
    }

    #[test]
    fn grid_cells_look_equal_different_char_same_style() {
        // look_equal only compares style, not data content
        let a = make_cell(b'A', 8, 8);
        let b = make_cell(b'B', 8, 8);
        assert_eq!(unsafe { grid_cells_look_equal(&a, &b) }, 1);
    }

    // ---------------------------------------------------------------
    // Line operations
    // ---------------------------------------------------------------

    #[test]
    fn grid_get_line_returns_valid_ref() {
        let mut gd = grid_create(80, 24, 0);
        // Just check we can read fields without panicking on bounds.
        let _cellused = gd.get_line(0).cellused;
        let _cellused_last = gd.get_line(23).cellused;
        drop(gd);
    }

    #[test]
    fn grid_line_length_on_empty_line() {
        let mut gd = grid_create(80, 24, 0);
        let len = gd.line_length(0);
        assert_eq!(len, 0);
        drop(gd);
    }

    #[test]
    fn grid_line_length_after_set_cell() {
        let mut gd = grid_create(80, 24, 0);
        let cell = make_cell(b'A', 8, 8);
        gd.set_cell(0, 0, &cell);
        gd.set_cell(4, 0, &cell);

        // Line length should be 5 (positions 0..=4, trailing spaces trimmed).
        let len = gd.line_length(0);
        assert_eq!(len, 5);
        drop(gd);
    }

    #[test]
    fn grid_empty_line_clears_cells() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell = make_cell(b'X', 8, 8);
            gd.set_cell(0, 0, &cell);
            gd.set_cell(5, 0, &cell);

            // Now empty line 0.
            gd.empty_line(0, 8);

            // After emptying, get_cell should return default.
            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &GRID_DEFAULT_CELL));

            let len = gd.line_length(0);
            assert_eq!(len, 0);
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Grid-wide operations
    // ---------------------------------------------------------------

    #[test]
    fn grid_clear_rectangular_region() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell = make_cell(b'#', 8, 8);
            // Fill row 0, columns 0..10
            for x in 0..10 {
                gd.set_cell(x, 0, &cell);
            }

            // Clear columns 2..6 on row 0 (px=2, py=0, nx=4, ny=1).
            gd.clear(2, 0, 4, 1, 8);

            // Cells outside cleared region should still be '#'.
            let mut gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &cell));
            gc = gd.get_cell(1, 0);
            assert!(grid_cells_equal(&gc, &cell));

            // Cells inside cleared region should be cleared/default.
            gc = gd.get_cell(2, 0);
            assert!(
                grid_cells_equal(&gc, &GRID_CLEARED_CELL)
                    || grid_cells_equal(&gc, &GRID_DEFAULT_CELL)
            );

            // Cell after cleared region still '#'.
            gc = gd.get_cell(6, 0);
            assert!(grid_cells_equal(&gc, &cell));

            drop(gd);
        }
    }

    #[test]
    fn grid_compare_equal_grids() {
        let mut g1 = grid_create(80, 24, 0);
        let mut g2 = grid_create(80, 24, 0);
        unsafe {
            assert_eq!(grid_compare(&raw mut *g1, &raw mut *g2), 0);
            drop(g1);
            drop(g2);
        }
    }

    #[test]
    fn grid_compare_different_dimensions() {
        let mut g1 = grid_create(80, 24, 0);
        let mut g2 = grid_create(40, 24, 0);
        unsafe {
            assert_ne!(grid_compare(&raw mut *g1, &raw mut *g2), 0);
            drop(g1);
            drop(g2);
        }
    }

    #[test]
    fn grid_compare_different_content() {
        let mut g1 = grid_create(80, 24, 0);
        let mut g2 = grid_create(80, 24, 0);
        unsafe {
            let cell = make_cell(b'A', 8, 8);
            g1.set_cell(0, 0, &cell);
            // g2 has no cell set at (0,0), so they differ.
            assert_ne!(grid_compare(&raw mut *g1, &raw mut *g2), 0);
            drop(g1);
            drop(g2);
        }
    }

    #[test]
    fn grid_compare_same_content() {
        let mut g1 = grid_create(80, 24, 0);
        let mut g2 = grid_create(80, 24, 0);
        unsafe {
            let cell = make_cell(b'Q', 8, 8);
            g1.set_cell(3, 2, &cell);
            g2.set_cell(3, 2, &cell);
            assert_eq!(grid_compare(&raw mut *g1, &raw mut *g2), 0);
            drop(g1);
            drop(g2);
        }
    }

    #[test]
    fn grid_move_lines_basic() {
        let mut gd = grid_create(80, 10, 0);
        unsafe {
            let cell = make_cell(b'M', 8, 8);
            gd.set_cell(0, 0, &cell);

            // Move line 0 to line 5.
            gd.move_lines(5, 0, 1, 8);

            // Line 0 should now be empty.
            let mut gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &GRID_DEFAULT_CELL));

            // Line 5 should have the cell.
            gc = gd.get_cell(0, 5);
            assert!(grid_cells_equal(&gc, &cell));

            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Position operations
    // ---------------------------------------------------------------

    #[test]
    fn grid_wrap_unwrap_position_roundtrip() {
        let mut gd = grid_create(80, 10, 0);
        // Place content so that cellused > px for the lines we test.
        let cell = make_cell(b'.', 8, 8);
        for y in 0..4 {
            for x in 0..10 {
                gd.set_cell(x, y, &cell);
            }
        }

        let (wx, wy) = gd.wrap_position(5, 3);
        let (px, py) = gd.unwrap_position(wx, wy);

        assert_eq!(px, 5);
        assert_eq!(py, 3);

        drop(gd);
    }

    #[test]
    fn grid_wrap_position_at_end_of_line() {
        let gd = grid_create(80, 10, 0);
        // cellused for line 0 is 0 on a fresh grid, so px=0 >= cellused=0.
        let (wx, wy) = gd.wrap_position(0, 0);
        assert_eq!(wx, u32::MAX);
        assert_eq!(wy, 0);
        drop(gd);
    }

    // ---------------------------------------------------------------
    // String conversion
    // ---------------------------------------------------------------

    #[test]
    fn grid_string_cells_empty_line() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let mut lastgc: *mut grid_cell = null_mut();
            let buf = gd.string_cells(
                0,
                0,
                80,
                &mut lastgc,
                grid_string_flags::empty(),
                null_mut(),
            );
            assert!(!buf.is_null());
            // Empty line should produce empty string.
            assert_eq!(*buf, 0);
            free_(buf);
            drop(gd);
        }
    }

    #[test]
    fn grid_string_cells_with_content() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell_h = make_cell(b'H', 8, 8);
            let cell_i = make_cell(b'i', 8, 8);
            gd.set_cell(0, 0, &cell_h);
            gd.set_cell(1, 0, &cell_i);

            let mut lastgc: *mut grid_cell = null_mut();
            let buf = gd.string_cells(
                0,
                0,
                80,
                &mut lastgc,
                grid_string_flags::empty(),
                null_mut(),
            );
            assert!(!buf.is_null());

            let s = std::ffi::CStr::from_ptr(buf as *const i8);
            assert_eq!(s.to_str().unwrap(), "Hi");

            free_(buf);
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Grid duplicate lines
    // ---------------------------------------------------------------

    #[test]
    fn grid_duplicate_lines_produces_equal_grids() {
        let mut src = grid_create(80, 5, 0);
        let mut dst = grid_create(80, 5, 0);
        unsafe {
            let cell = make_cell(b'D', 8, 8);
            src.set_cell(0, 0, &cell);
            src.set_cell(3, 2, &cell);

            dst.duplicate_lines(0, &raw mut *src, 0, 5);

            assert_eq!(grid_compare(&raw mut *src, &raw mut *dst), 0);

            drop(src);
            drop(dst);
        }
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn grid_clear_zero_size_does_not_crash() {
        let mut gd = grid_create(80, 24, 0);
        // nx=0 or ny=0 should be no-op.
        gd.clear(0, 0, 0, 1, 8);
        gd.clear(0, 0, 1, 0, 8);
        drop(gd);
    }

    #[test]
    fn grid_move_lines_noop_same_src_dst() {
        let mut gd = grid_create(80, 10, 0);
        unsafe {
            let cell = make_cell(b'N', 8, 8);
            gd.set_cell(0, 2, &cell);

            // Moving line 2 to line 2 should be a no-op.
            gd.move_lines(2, 2, 1, 8);

            let gc: grid_cell;
            gc = gd.get_cell(0, 2);
            assert!(grid_cells_equal(&gc, &cell));

            drop(gd);
        }
    }

    #[test]
    fn grid_default_cell_is_space() {
        assert_eq!(GRID_DEFAULT_CELL.data.data[0], b' ');
        assert_eq!(GRID_DEFAULT_CELL.data.size, 1);
        assert_eq!(GRID_DEFAULT_CELL.data.width, 1);
        assert_eq!(GRID_DEFAULT_CELL.fg, 8);
        assert_eq!(GRID_DEFAULT_CELL.bg, 8);
    }

    #[test]
    fn grid_padding_cell_has_padding_flag() {
        assert!(GRID_PADDING_CELL.flags.intersects(grid_flag::PADDING));
        assert_eq!(GRID_PADDING_CELL.data.width, 0);
    }

    #[test]
    fn grid_cleared_cell_has_cleared_flag() {
        assert!(GRID_CLEARED_CELL.flags.intersects(grid_flag::CLEARED));
        assert_eq!(GRID_CLEARED_CELL.data.data[0], b' ');
    }

    // ---------------------------------------------------------------
    // Extended cells (wide chars, RGB colors, attributes)
    // ---------------------------------------------------------------

    /// Helper: create a grid_cell with an RGB foreground color.
    /// RGB colors force the EXTENDED code path in grid storage.
    fn make_rgb_fg_cell(ch: u8, r: u8, g: u8, b: u8) -> grid_cell {
        grid_cell::new(
            utf8_data::new([ch], 0, 1, 1),
            grid_attr::empty(),
            grid_flag::empty(),
            colour_join_rgb(r, g, b),
            8,
            8,
            0,
        )
    }

    /// Helper: create a grid_cell with an RGB background color.
    fn make_rgb_bg_cell(ch: u8, r: u8, g: u8, b: u8) -> grid_cell {
        grid_cell::new(
            utf8_data::new([ch], 0, 1, 1),
            grid_attr::empty(),
            grid_flag::empty(),
            8,
            colour_join_rgb(r, g, b),
            8,
            0,
        )
    }

    /// Helper: create a wide (width=2) cell. The multi-byte UTF-8 data
    /// and width > 1 both force the EXTENDED path.
    fn make_wide_cell() -> grid_cell {
        // Use a 3-byte UTF-8 sequence for a CJK character (U+4E16 '世')
        grid_cell::new(
            utf8_data::new([0xE4, 0xB8, 0x96], 0, 3, 2),
            grid_attr::empty(),
            grid_flag::empty(),
            8,
            8,
            8,
            0,
        )
    }

    /// Helper: create a cell with underscore color (forces EXTENDED).
    fn make_us_cell(ch: u8, us: i32) -> grid_cell {
        grid_cell::new(
            utf8_data::new([ch], 0, 1, 1),
            grid_attr::empty(),
            grid_flag::empty(),
            8,
            8,
            us,
            0,
        )
    }

    #[test]
    fn grid_extended_cell_rgb_fg_roundtrip() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell_in = make_rgb_fg_cell(b'R', 255, 0, 128);
            gd.set_cell(0, 0, &cell_in);

            let cell_out: grid_cell;
            cell_out = gd.get_cell(0, 0);

            assert!(grid_cells_equal(&cell_in, &cell_out));
            // Verify the RGB value survived the extended storage round-trip.
            assert_eq!(cell_out.fg, colour_join_rgb(255, 0, 128));
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_cell_rgb_bg_roundtrip() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell_in = make_rgb_bg_cell(b'B', 0, 128, 255);
            gd.set_cell(3, 1, &cell_in);

            let cell_out: grid_cell;
            cell_out = gd.get_cell(3, 1);

            assert!(grid_cells_equal(&cell_in, &cell_out));
            assert_eq!(cell_out.bg, colour_join_rgb(0, 128, 255));
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_cell_wide_char_roundtrip() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let cell_in = make_wide_cell();
            gd.set_cell(0, 0, &cell_in);

            let cell_out: grid_cell;
            cell_out = gd.get_cell(0, 0);

            assert!(grid_cells_equal(&cell_in, &cell_out));
            assert_eq!(cell_out.data.width, 2);
            assert_eq!(cell_out.data.size, 3);
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_cell_underscore_color_roundtrip() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            // us != 8 forces EXTENDED.
            let cell_in = make_us_cell(b'U', COLOUR_FLAG_256 | 42);
            gd.set_cell(2, 0, &cell_in);

            let cell_out: grid_cell;
            cell_out = gd.get_cell(2, 0);

            assert!(grid_cells_equal(&cell_in, &cell_out));
            assert_eq!(cell_out.us, COLOUR_FLAG_256 | 42);
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_cell_with_attributes() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            // attr > 0xff forces EXTENDED path.
            let cell_in = grid_cell::new(
                utf8_data::new([b'A'], 0, 1, 1),
                grid_attr::GRID_ATTR_BRIGHT | grid_attr::GRID_ATTR_UNDERSCORE_2,
                grid_flag::empty(),
                8,
                8,
                8,
                0,
            );
            gd.set_cell(0, 0, &cell_in);

            let cell_out: grid_cell;
            cell_out = gd.get_cell(0, 0);

            assert!(grid_cells_equal(&cell_in, &cell_out));
            assert!(cell_out.attr.contains(grid_attr::GRID_ATTR_BRIGHT));
            assert!(cell_out.attr.contains(grid_attr::GRID_ATTR_UNDERSCORE_2));
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_mixed_inline_and_extended_cells() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            // Set alternating inline (simple ASCII) and extended (RGB) cells.
            let simple = make_cell(b'S', 8, 8);
            let extended = make_rgb_fg_cell(b'E', 100, 200, 50);

            for x in 0..10u32 {
                if x % 2 == 0 {
                    gd.set_cell(x, 0, &simple);
                } else {
                    gd.set_cell(x, 0, &extended);
                }
            }

            // Verify all round-trip correctly.
            for x in 0..10u32 {
                let gc: grid_cell;
                gc = gd.get_cell(x, 0);
                if x % 2 == 0 {
                    assert!(grid_cells_equal(&gc, &simple), "mismatch at x={x}");
                } else {
                    assert!(grid_cells_equal(&gc, &extended), "mismatch at x={x}");
                }
            }
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_overwrite_inline_with_extended() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            // Write a simple cell first, then overwrite with an extended cell.
            let simple = make_cell(b'A', 8, 8);
            let extended = make_rgb_fg_cell(b'B', 10, 20, 30);

            gd.set_cell(0, 0, &simple);
            gd.set_cell(0, 0, &extended);

            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &extended));
            drop(gd);
        }
    }

    #[test]
    fn grid_extended_overwrite_extended_with_inline() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            // Write an extended cell first, then overwrite with a simple cell.
            let extended = make_rgb_fg_cell(b'B', 10, 20, 30);
            let simple = make_cell(b'A', 8, 8);

            gd.set_cell(0, 0, &extended);
            gd.set_cell(0, 0, &simple);

            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &simple));
            drop(gd);
        }
    }

    #[test]
    fn grid_set_padding_after_wide_char() {
        let mut gd = grid_create(80, 24, 0);
        let wide = make_wide_cell();
        gd.set_cell(0, 0, &wide);
        gd.set_padding(1, 0);

        let gc: grid_cell;
        gc = gd.get_cell(1, 0);
        assert!(gc.flags.intersects(grid_flag::PADDING));
        drop(gd);
    }

    #[test]
    fn grid_duplicate_lines_preserves_extended_cells() {
        let mut src = grid_create(80, 5, 0);
        let mut dst = grid_create(80, 5, 0);
        unsafe {
            let extended = make_rgb_fg_cell(b'X', 255, 128, 0);
            let wide = make_wide_cell();
            src.set_cell(0, 0, &extended);
            src.set_cell(5, 2, &wide);

            dst.duplicate_lines(0, &raw mut *src, 0, 5);

            let mut gc: grid_cell;
            gc = dst.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &extended));

            gc = dst.get_cell(5, 2);
            assert!(grid_cells_equal(&gc, &wide));

            drop(src);
            drop(dst);
        }
    }

    // ---------------------------------------------------------------
    // Line expansion
    // ---------------------------------------------------------------

    #[test]
    fn grid_expand_line_grows_cellsize() {
        let mut gd = grid_create(80, 5, 0);
        let gl = gd.get_line(0);
        assert_eq!((*gl).celldata.len() as u32, 0);

        // Setting a cell at position 10 should expand the line.
        let cell = make_cell(b'X', 8, 8);
        gd.set_cell(10, 0, &cell);

        let gl = gd.get_line(0);
        assert!((*gl).celldata.len() as u32 >= 11);
        assert_eq!((*gl).cellused, 11);
        drop(gd);
    }

    #[test]
    fn grid_expand_line_minimum_quarter_sx() {
        // grid_expand_line rounds up to sx/4 for small expansions.
        let mut gd = grid_create(80, 5, 0);
        let cell = make_cell(b'X', 8, 8);
        // Request expansion to column 2 — should round up to sx/4 = 20.
        gd.set_cell(1, 0, &cell);

        let gl = gd.get_line(0);
        assert!(
            (*gl).celldata.len() as u32 >= 20,
            "cellsize {} should be >= sx/4 = 20",
            (*gl).celldata.len() as u32
        );
        drop(gd);
    }

    #[test]
    fn grid_expand_line_cleared_cells_are_default() {
        let mut gd = grid_create(80, 5, 0);
        unsafe {
            let cell = make_cell(b'Z', 8, 8);
            gd.set_cell(5, 0, &cell);

            // Positions 0..5 should be cleared (default-ish) cells.
            let mut gc: grid_cell;
            for x in 0..5u32 {
                gc = gd.get_cell(x, 0);
                // Cleared cells have the CLEARED flag OR are default.
                assert!(
                    grid_cells_equal(&gc, &GRID_CLEARED_CELL)
                        || grid_cells_equal(&gc, &GRID_DEFAULT_CELL),
                    "cell at x={x} was neither cleared nor default"
                );
            }
            drop(gd);
        }
    }

    #[test]
    fn grid_set_cells_bulk_write() {
        let mut gd = grid_create(80, 5, 0);
        unsafe {
            let gc = make_cell(b'A', 8, 8);
            let data = b"Hello";
            gd.set_cells(0, 0, &gc, data.as_ptr(), data.len());

            let mut out: grid_cell;
            for (i, &ch) in data.iter().enumerate() {
                out = gd.get_cell(i as u32, 0);
                assert_eq!(out.data.data[0], ch, "mismatch at i={i}");
            }
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // History scrolling
    // ---------------------------------------------------------------

    #[test]
    fn grid_scroll_history_moves_line_to_history() {
        let mut gd = grid_create(80, 5, 1000);
        unsafe {
            let cell = make_cell(b'H', 8, 8);
            gd.set_cell(0, 0, &cell);

            assert_eq!(gd.hsize, 0);
            gd.scroll_history(8);
            assert_eq!(gd.hsize, 1);
            assert_eq!(gd.hscrolled, 1);

            // The old visible line 0 is now history line 0.
            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &cell));

            // New visible line 0 (= line hsize) should be empty.
            let gl = gd.get_line(gd.hsize);
            assert_eq!((*gl).cellused, 0);

            drop(gd);
        }
    }

    #[test]
    fn grid_scroll_history_multiple_times() {
        let mut gd = grid_create(80, 3, 1000);
        // Fill 3 visible lines.
        for y in 0..3u32 {
            let cell = make_cell(b'0' + y as u8, 8, 8);
            gd.set_cell(0, y, &cell);
        }

        // Scroll twice.
        gd.scroll_history(8);
        gd.scroll_history(8);
        assert_eq!(gd.hsize, 2);

        // History lines should contain '0' and '1'.
        let mut gc: grid_cell;
        gc = gd.get_cell(0, 0);
        assert_eq!(gc.data.data[0], b'0');
        gc = gd.get_cell(0, 1);
        assert_eq!(gc.data.data[0], b'1');

        drop(gd);
    }

    #[test]
    fn grid_scroll_history_region_moves_upper_to_history() {
        let mut gd = grid_create(80, 5, 1000);
        // Fill lines with distinct characters.
        for y in 0..5u32 {
            let cell = make_cell(b'A' + y as u8, 8, 8);
            gd.set_cell(0, y, &cell);
        }

        // Scroll region [1..3] — line at upper=1 moves to history.
        gd.scroll_history_region(1, 3, 8);
        assert_eq!(gd.hsize, 1);

        // History line 0 should be line that had 'B'.
        let gc: grid_cell;
        gc = gd.get_cell(0, 0);
        assert_eq!(gc.data.data[0], b'B');

        drop(gd);
    }

    #[test]
    fn grid_clear_history_removes_all_history() {
        let mut gd = grid_create(80, 3, 1000);
        let cell = make_cell(b'H', 8, 8);
        gd.set_cell(0, 0, &cell);

        gd.scroll_history(8);
        gd.scroll_history(8);
        assert_eq!(gd.hsize, 2);

        gd.clear_history();
        assert_eq!(gd.hsize, 0);
        assert_eq!(gd.hscrolled, 0);
        drop(gd);
    }

    #[test]
    fn grid_collect_history_trims_oldest() {
        // hlimit=10, fill 10 history lines, collect should trim ~10%.
        let mut gd = grid_create(80, 3, 10);
        for _ in 0..10 {
            let cell = make_cell(b'.', 8, 8);
            gd.set_cell(0, gd.hsize, &cell);
            gd.scroll_history(8);
        }
        assert_eq!(gd.hsize, 10);

        gd.collect_history();
        // Should have trimmed 1 line (10% of 10, minimum 1).
        assert_eq!(gd.hsize, 9);
        drop(gd);
    }

    #[test]
    fn grid_remove_history_removes_from_bottom() {
        let mut gd = grid_create(80, 3, 1000);
        for y in 0..3u32 {
            let cell = make_cell(b'0' + y as u8, 8, 8);
            gd.set_cell(0, y, &cell);
        }
        gd.scroll_history(8);
        gd.scroll_history(8);
        assert_eq!(gd.hsize, 2);

        gd.remove_history(1);
        assert_eq!(gd.hsize, 1);

        // Remaining history line should be the oldest ('0').
        let gc: grid_cell;
        gc = gd.get_cell(0, 0);
        assert_eq!(gc.data.data[0], b'0');
        drop(gd);
    }

    // ---------------------------------------------------------------
    // Grid clear and move cells
    // ---------------------------------------------------------------

    #[test]
    fn grid_clear_lines_empties_range() {
        let mut gd = grid_create(80, 5, 0);
        unsafe {
            let cell = make_cell(b'X', 8, 8);
            for y in 0..5u32 {
                gd.set_cell(0, y, &cell);
            }

            gd.clear_lines(1, 2, 8);

            let mut gc: grid_cell;
            // Lines 1 and 2 should be empty.
            assert_eq!(gd.line_length(1), 0);
            assert_eq!(gd.line_length(2), 0);
            // Lines 0, 3, 4 should still have content.
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &cell));
            gc = gd.get_cell(0, 3);
            assert!(grid_cells_equal(&gc, &cell));
            drop(gd);
        }
    }

    #[test]
    fn grid_move_cells_shifts_within_line() {
        let mut gd = grid_create(80, 5, 0);
        unsafe {
            let cell_a = make_cell(b'A', 8, 8);
            let cell_b = make_cell(b'B', 8, 8);
            gd.set_cell(0, 0, &cell_a);
            gd.set_cell(1, 0, &cell_b);

            // Move 2 cells from position 0 to position 5.
            gd.move_cells(5, 0, 0, 2, 8);

            let mut gc: grid_cell;
            gc = gd.get_cell(5, 0);
            assert_eq!(gc.data.data[0], b'A');
            gc = gd.get_cell(6, 0);
            assert_eq!(gc.data.data[0], b'B');

            // Original positions should be cleared.
            gc = gd.get_cell(0, 0);
            assert!(
                grid_cells_equal(&gc, &GRID_CLEARED_CELL)
                    || grid_cells_equal(&gc, &GRID_DEFAULT_CELL)
            );
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Reflow
    // ---------------------------------------------------------------

    #[test]
    fn grid_reflow_narrower_splits_long_lines() {
        let mut gd = grid_create(20, 3, 1000);
        unsafe {
            // Write 20 characters across line 0 then scroll into history.
            let cell = make_cell(b'.', 8, 8);
            for x in 0..20u32 {
                gd.set_cell(x, 0, &cell);
            }
            // Mark line as wrapped (as tmux does for long lines).
            (*gd.get_line(0)).flags |= grid_line_flag::WRAPPED;
            gd.scroll_history(8);
            let hsize_before = gd.hsize;

            // Reflow to width 10 — the 20-char history line should split into 2.
            gd.reflow(10);
            assert!(
                gd.hsize >= hsize_before + 1,
                "hsize should grow when lines split: was {hsize_before}, now {}",
                gd.hsize
            );

            // Content should be preserved.
            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert_eq!(gc.data.data[0], b'.');
            drop(gd);
        }
    }

    #[test]
    fn grid_reflow_wider_joins_wrapped_lines() {
        let mut gd = grid_create(10, 3, 1000);
        unsafe {
            // Write 10 chars on line 0 (wrapped), 5 chars on line 1.
            let cell = make_cell(b'A', 8, 8);
            for x in 0..10u32 {
                gd.set_cell(x, 0, &cell);
            }
            (*gd.get_line(0)).flags |= grid_line_flag::WRAPPED;
            let cell_b = make_cell(b'B', 8, 8);
            for x in 0..5u32 {
                gd.set_cell(x, 1, &cell_b);
            }

            // Scroll both into history.
            gd.scroll_history(8);
            gd.scroll_history(8);
            let hsize_before = gd.hsize;

            // Reflow to width 20 — the two wrapped lines should join.
            gd.reflow(20);
            assert!(
                gd.hsize <= hsize_before,
                "hsize should shrink when lines join: was {hsize_before}, now {}",
                gd.hsize
            );

            drop(gd);
        }
    }

    #[test]
    fn grid_reflow_preserves_unwrapped_lines() {
        let mut gd = grid_create(20, 3, 1000);
        unsafe {
            // Write a short line (not wrapped) and scroll to history.
            let cell = make_cell(b'S', 8, 8);
            for x in 0..5u32 {
                gd.set_cell(x, 0, &cell);
            }
            // Don't set WRAPPED flag — this is a short line.
            gd.scroll_history(8);

            // Reflow to width 10 — short unwrapped line should stay as-is.
            gd.reflow(10);

            let mut gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert_eq!(gc.data.data[0], b'S');
            gc = gd.get_cell(4, 0);
            assert_eq!(gc.data.data[0], b'S');
            drop(gd);
        }
    }

    #[test]
    fn grid_reflow_same_width_is_identity() {
        let mut gd = grid_create(80, 3, 1000);
        unsafe {
            let cell = make_cell(b'I', 8, 8);
            for x in 0..10u32 {
                gd.set_cell(x, 0, &cell);
            }
            gd.scroll_history(8);
            let hsize_before = gd.hsize;

            // Reflow to same width — should be essentially a no-op.
            gd.reflow(80);
            assert_eq!(gd.hsize, hsize_before);

            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert_eq!(gc.data.data[0], b'I');
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Grid string conversion with extended cells
    // ---------------------------------------------------------------

    #[test]
    fn grid_string_cells_with_wide_chars() {
        let mut gd = grid_create(80, 24, 0);
        unsafe {
            let wide = make_wide_cell();
            gd.set_cell(0, 0, &wide);
            gd.set_padding(1, 0);
            let ascii = make_cell(b'!', 8, 8);
            gd.set_cell(2, 0, &ascii);

            let mut lastgc: *mut grid_cell = null_mut();
            let buf = gd.string_cells(
                0,
                0,
                80,
                &mut lastgc,
                grid_string_flags::empty(),
                null_mut(),
            );
            assert!(!buf.is_null());

            let s = std::ffi::CStr::from_ptr(buf as *const i8);
            let s = s.to_str().unwrap();
            // Should contain the UTF-8 bytes of '世' followed by '!'.
            assert!(s.contains('世'), "expected '世' in output, got: {s}");
            assert!(s.ends_with('!'), "expected trailing '!' in output, got: {s}");

            free_(buf);
            drop(gd);
        }
    }

    #[test]
    fn grid_line_length_with_trailing_spaces_and_extended() {
        let mut gd = grid_create(80, 5, 0);
        let extended = make_rgb_fg_cell(b' ', 255, 0, 0);
        let cell = make_cell(b'A', 8, 8);

        gd.set_cell(0, 0, &cell);
        // Trailing spaces (even with color) count as spaces for length trimming.
        gd.set_cell(1, 0, &extended);

        let len = gd.line_length(0);
        // grid_line_length trims trailing spaces regardless of style.
        assert_eq!(len, 1);
        drop(gd);
    }

    // ---------------------------------------------------------------
    // History with extended cells
    // ---------------------------------------------------------------

    #[test]
    fn grid_scroll_history_preserves_extended_cells() {
        let mut gd = grid_create(80, 3, 1000);
        unsafe {
            let extended = make_rgb_fg_cell(b'C', 0, 255, 0);
            gd.set_cell(0, 0, &extended);

            gd.scroll_history(8);

            // Extended cell should survive in history.
            let gc: grid_cell;
            gc = gd.get_cell(0, 0);
            assert!(grid_cells_equal(&gc, &extended));
            assert_eq!(gc.fg, colour_join_rgb(0, 255, 0));
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn grid_get_cell_out_of_cellsize_returns_default() {
        let mut gd = grid_create(80, 5, 0);
        unsafe {
            // Only set cell at position 0, then read beyond cellsize.
            let cell = make_cell(b'X', 8, 8);
            gd.set_cell(0, 0, &cell);

            let gc: grid_cell;
            gc = gd.get_cell(50, 0);
            assert!(grid_cells_equal(&gc, &GRID_DEFAULT_CELL));
            drop(gd);
        }
    }

    #[test]
    fn grid_compare_with_extended_cells() {
        let mut g1 = grid_create(80, 5, 0);
        let mut g2 = grid_create(80, 5, 0);
        unsafe {
            let extended = make_rgb_fg_cell(b'E', 128, 64, 32);
            g1.set_cell(0, 0, &extended);
            g2.set_cell(0, 0, &extended);

            assert_eq!(grid_compare(&raw mut *g1, &raw mut *g2), 0);

            // Change one — should differ.
            let other = make_rgb_fg_cell(b'E', 128, 64, 33);
            g2.set_cell(0, 0, &other);
            assert_ne!(grid_compare(&raw mut *g1, &raw mut *g2), 0);

            drop(g1);
            drop(g2);
        }
    }

    #[test]
    fn grid_clear_with_non_default_bg() {
        let mut gd = grid_create(80, 5, 0);
        let cell = make_cell(b'X', 8, 8);
        for x in 0..10 {
            gd.set_cell(x, 0, &cell);
        }

        // Clear with a non-default background (256 color).
        let bg = (COLOUR_FLAG_256 | 42) as u32;
        gd.clear(2, 0, 3, 1, bg);

        // Cleared cells should have a non-default bg.
        let gc: grid_cell;
        gc = gd.get_cell(2, 0);
        assert!(gc.flags.intersects(grid_flag::CLEARED));
        drop(gd);
    }

    #[test]
    fn grid_move_lines_overlapping_regions() {
        // Move overlapping regions: lines [0..3] → [2..5].
        let mut gd = grid_create(80, 10, 0);
        unsafe {
            for y in 0..3u32 {
                let cell = make_cell(b'A' + y as u8, 8, 8);
                gd.set_cell(0, y, &cell);
            }

            gd.move_lines(2, 0, 3, 8);

            let mut gc: grid_cell;
            gc = gd.get_cell(0, 2);
            assert_eq!(gc.data.data[0], b'A');
            gc = gd.get_cell(0, 3);
            assert_eq!(gc.data.data[0], b'B');
            gc = gd.get_cell(0, 4);
            assert_eq!(gc.data.data[0], b'C');
            drop(gd);
        }
    }

    // ---------------------------------------------------------------
    // view_* coordinate translation
    //
    // The grid stores both scrollback history and the visible screen.
    // History occupies rows `0..hsize`, and the visible area occupies
    // `hsize..hsize+sy`. The view_* methods translate "view coordinates"
    // (where y=0 is the top of the visible screen) into absolute grid
    // coordinates by adding `hsize` to y.
    // ---------------------------------------------------------------

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
            let gc: grid_cell;
            gc = (*gd).view_get_cell(px, py);
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

    #[test]
    fn view_set_and_get_cell_no_history() {
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
    fn view_set_and_get_cell_with_history() {
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
            let gc_read: grid_cell;
            gc_read = (*gd_ptr).get_cell(0, 0);
            assert_eq!(gc_read.data.data[0], b'X');

            drop(gd);
        }
    }

    #[test]
    fn view_clear_region() {
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

    #[test]
    fn view_string_cells_reads_visible() {
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
    fn view_string_cells_with_history_offset() {
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

    #[test]
    fn view_delete_cells_shifts_left() {
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
    fn view_insert_cells_shifts_right() {
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

    #[test]
    fn view_scroll_region_down_inserts_blank() {
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
        }
    }

    #[test]
    fn view_clear_history_moves_content() {
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
