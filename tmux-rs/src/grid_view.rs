use ::core::{
    ffi::{c_char, c_uint},
    ptr::null_mut,
};

use crate::{
    GRID_HISTORY, grid, grid_cell, grid_clear, grid_collect_history, grid_get_cell, grid_get_line,
    grid_move_cells, grid_move_lines, grid_scroll_history, grid_scroll_history_region,
    grid_set_cell, grid_set_cells, grid_set_padding, grid_string_cells,
};

unsafe extern "C" {
    // pub fn grid_view_get_cell(_: *mut grid, _: c_uint, _: c_uint, _: *mut grid_cell);
    // pub fn grid_view_set_cell(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell);
    // pub fn grid_view_set_padding(_: *mut grid, _: c_uint, _: c_uint);
    // pub fn grid_view_set_cells(_: *mut grid, _: c_uint, _: c_uint, _: *const grid_cell, _: *const c_char, _: usize);
    // pub fn grid_view_clear_history(_: *mut grid, _: c_uint);
    // pub fn grid_view_clear(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_scroll_region_up(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_scroll_region_down(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_insert_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_insert_lines_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_delete_lines(_: *mut grid, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_delete_lines_region(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_insert_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_delete_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    // pub fn grid_view_string_cells(_: *mut grid, _: c_uint, _: c_uint, _: c_uint) -> *mut c_char;
}

fn grid_view_x(gd: *mut grid, x: u32) -> u32 {
    x
}
unsafe fn grid_view_y(gd: *mut grid, y: u32) -> u32 {
    unsafe { (*gd).hsize + (y) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_get_cell(gd: *mut grid, px: u32, py: u32, gc: *mut grid_cell) {
    unsafe {
        grid_get_cell(gd, grid_view_x(gd, px), grid_view_y(gd, py), gc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_set_cell(gd: *mut grid, px: u32, py: u32, gc: *const grid_cell) {
    unsafe {
        grid_set_cell(gd, grid_view_x(gd, px), grid_view_y(gd, py), gc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_set_padding(gd: *mut grid, px: u32, py: u32) {
    unsafe {
        grid_set_padding(gd, grid_view_x(gd, px), grid_view_y(gd, py));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_set_cells(
    gd: *mut grid,
    px: u32,
    py: u32,
    gc: *const grid_cell,
    s: *const c_char,
    slen: usize,
) {
    unsafe {
        grid_set_cells(gd, grid_view_x(gd, px), grid_view_y(gd, py), gc, s, slen);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_clear_history(gd: *mut grid, bg: u32) {
    unsafe {
        let mut last = 0u32;

        for yy in 0..(*gd).sy {
            let gl = grid_get_line(gd, grid_view_y(gd, yy));
            if (*gl).cellused != 0 {
                last = yy + 1;
            }
        }
        if (last == 0) {
            grid_view_clear(gd, 0, 0, (*gd).sx, (*gd).sy, bg);
            return;
        }

        for yy in 0..(*gd).sy {
            grid_collect_history(gd);
            grid_scroll_history(gd, bg);
        }
        if last < (*gd).sy {
            grid_view_clear(gd, 0, 0, (*gd).sx, (*gd).sy - last, bg);
        }
        (*gd).hscrolled = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_clear(
    gd: *mut grid,
    mut px: u32,
    mut py: u32,
    nx: u32,
    ny: u32,
    bg: u32,
) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        grid_clear(gd, px, py, nx, ny, bg);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_scroll_region_up(
    gd: *mut grid,
    mut rupper: u32,
    mut rlower: u32,
    bg: u32,
) {
    unsafe {
        if ((*gd).flags & GRID_HISTORY != 0) {
            grid_collect_history(gd);
            if (rupper == 0 && rlower == (*gd).sy - 1) {
                grid_scroll_history(gd, bg);
            } else {
                rupper = grid_view_y(gd, rupper);
                rlower = grid_view_y(gd, rlower);
                grid_scroll_history_region(gd, rupper, rlower, bg);
            }
        } else {
            rupper = grid_view_y(gd, rupper);
            rlower = grid_view_y(gd, rlower);
            grid_move_lines(gd, rupper, rupper + 1, rlower - rupper, bg);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_scroll_region_down(
    gd: *mut grid,
    mut rupper: u32,
    mut rlower: u32,
    bg: u32,
) {
    unsafe {
        rupper = grid_view_y(gd, rupper);
        rlower = grid_view_y(gd, rlower);

        grid_move_lines(gd, rupper + 1, rupper, rlower - rupper, bg);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_insert_lines(gd: *mut grid, mut py: u32, ny: u32, bg: u32) {
    unsafe {
        py = grid_view_y(gd, py);

        let sy = grid_view_y(gd, (*gd).sy);

        grid_move_lines(gd, py + ny, py, sy - py - ny, bg);
    }
}

/// Insert lines in region.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_insert_lines_region(
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
        grid_move_lines(gd, rlower + 1 - ny2, py, ny2, bg);
        // TODO does this bug exist upstream?
        grid_clear(gd, 0, py + ny2, (*gd).sx, ny.saturating_sub(ny2), bg);
    }
}

/// Delete lines.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_delete_lines(gd: *mut grid, mut py: u32, ny: u32, bg: u32) {
    unsafe {
        py = grid_view_y(gd, py);

        let sy = grid_view_y(gd, (*gd).sy);

        // TODO does this bug exist upstream?
        grid_move_lines(
            gd,
            py,
            py + ny,
            sy.saturating_sub(py).saturating_sub(ny),
            bg,
        );
        grid_clear(
            gd,
            0,
            sy.saturating_sub(ny),
            (*gd).sx,
            (py + ny + ny).saturating_sub(sy),
            bg,
        );
    }
}

/// Delete lines inside scroll region.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_delete_lines_region(
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
        grid_move_lines(gd, py, py + ny, ny2, bg);
        // TODO does this bug exist in the tmux source code too
        grid_clear(gd, 0, py + ny2, (*gd).sx, ny.saturating_sub(ny2), bg);
    }
}

/// Insert characters.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_insert_cells(
    gd: *mut grid,
    mut px: u32,
    mut py: u32,
    nx: u32,
    bg: u32,
) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        let sx = grid_view_x(gd, (*gd).sx);

        if (px >= sx - 1) {
            grid_clear(gd, px, py, 1, 1, bg);
        } else {
            grid_move_cells(gd, px + nx, px, py, sx - px - nx, bg);
        }
    }
}

/// Delete characters.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_delete_cells(
    gd: *mut grid,
    mut px: u32,
    mut py: u32,
    nx: u32,
    bg: u32,
) {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        let sx = grid_view_x(gd, (*gd).sx);

        grid_move_cells(gd, px, px + nx, py, sx - px - nx, bg);
        grid_clear(gd, sx - nx, py, px + nx - (sx - nx), 1, bg);
    }
}

/// Convert cells into a string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_view_string_cells(
    gd: *mut grid,
    mut px: u32,
    mut py: u32,
    nx: u32,
) -> *mut c_char {
    unsafe {
        px = grid_view_x(gd, px);
        py = grid_view_y(gd, py);

        grid_string_cells(gd, px, py, nx, null_mut(), 0, null_mut())
    }
}
