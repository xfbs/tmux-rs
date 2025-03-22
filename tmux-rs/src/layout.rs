use compat_rs::queue::{tailq_empty, tailq_init, tailq_remove};

use crate::*;

unsafe extern "C" {
    pub fn layout_count_cells(_: *mut layout_cell) -> c_uint;
    // pub fn layout_create_cell(_: *mut layout_cell) -> *mut layout_cell;
    // pub fn layout_free_cell(_: *mut layout_cell);
    // pub fn layout_print_cell(_: *mut layout_cell, _: *const c_char, _: c_uint);
    pub fn layout_destroy_cell(_: *mut window, _: *mut layout_cell, _: *mut *mut layout_cell);
    pub fn layout_resize_layout(_: *mut window, _: *mut layout_cell, _: layout_type, _: c_int, _: c_int);
    // pub fn layout_search_by_border(_: *mut layout_cell, _: c_uint, _: c_uint) -> *mut layout_cell;
    pub fn layout_set_size(_: *mut layout_cell, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn layout_make_leaf(_: *mut layout_cell, _: *mut window_pane);
    pub fn layout_make_node(_: *mut layout_cell, _: layout_type);
    pub fn layout_fix_offsets(_: *mut window);
    pub fn layout_fix_panes(_: *mut window, _: *mut window_pane);
    pub fn layout_resize_adjust(_: *mut window, _: *mut layout_cell, _: layout_type, _: c_int);
    pub fn layout_init(_: *mut window, _: *mut window_pane);
    pub fn layout_free(_: *mut window);
    pub fn layout_resize(_: *mut window, _: c_uint, _: c_uint);
    pub fn layout_resize_pane(_: *mut window_pane, _: layout_type, _: c_int, _: c_int);
    pub fn layout_resize_pane_to(_: *mut window_pane, _: layout_type, _: c_uint);
    pub fn layout_assign_pane(_: *mut layout_cell, _: *mut window_pane, _: c_int);
    pub fn layout_split_pane(_: *mut window_pane, _: layout_type, _: c_int, _: c_int) -> *mut layout_cell;
    pub fn layout_close_pane(_: *mut window_pane);
    pub fn layout_spread_cell(_: *mut window, _: *mut layout_cell) -> c_int;
    pub fn layout_spread_out(_: *mut window_pane);
}

/*

const layout_sets_len: usize = 7;
static mut layout_sets: [layout_sets_entry; layout_sets_len] = [
    layout_sets_entry::new(c"even-horizontal", layout_set_even_h),
    layout_sets_entry::new(c"even-vertical", layout_set_even_v),
    layout_sets_entry::new(c"main-horizontal", layout_set_main_h),
    layout_sets_entry::new(c"main-horizontal-mirrored", layout_set_main_h_mirrored),
    layout_sets_entry::new(c"main-vertical", layout_set_main_v),
    layout_sets_entry::new(c"main-vertical-mirrored", layout_set_main_v_mirrored),
    layout_sets_entry::new(c"tiled", layout_set_tiled),
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_lookup(name: *const c_char) -> i32 {
    unsafe {
        let mut matched: i32 = -1;

        for i in 0..layout_sets_len {
            if (strcmp(layout_sets[i].name, name) == 0) {
                return i as i32;
            }
        }
        for i in 0..layout_sets_len {
            if (strncmp(layout_sets[i].name, name, strlen(name)) == 0) {
                if (matched != -1) {
                    /* ambiguous */
                    return -1;
                }
                matched = i as i32;
            }
        }

        matched
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_select(w: *mut window, mut layout: u32) -> u32 {
    unsafe {
        if (layout > layout_sets_len as u32 - 1) {
            layout = layout_sets_len as u32 - 1;
        }

        if let Some(arrange) = layout_sets[layout as usize].arrange {
            arrange(w);
        }

        (*w).lastlayout = layout as i32;
        layout
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_next(w: *mut window) -> u32 {
    unsafe {
        let mut layout: u32 = 0;

        if ((*w).lastlayout == -1) {
            layout = 0;
        } else {
            layout = ((*w).lastlayout + 1) as u32;
            if (layout > layout_sets_len as u32 - 1) {
                layout = 0;
            }
        }

        if let Some(arrange) = layout_sets[layout as usize].arrange {
            arrange(w);
        }
        (*w).lastlayout = layout as i32;
        layout
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_previous(w: *mut window) -> u32 {
    unsafe {
        let mut layout: u32 = 0;

        if ((*w).lastlayout == -1) {
            layout = (layout_sets_len - 1) as u32;
        } else {
            layout = (*w).lastlayout as u32;
            if (layout == 0) {
                layout = (layout_sets_len - 1) as u32;
            } else {
                layout -= 1;
            }
        }

        if let Some(arrange) = layout_sets[layout as usize].arrange {
            arrange(w);
        }
        (*w).lastlayout = layout as i32;
        layout
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_even(w: *mut window, type_: layout_type) {
    let __func__ = c"layout_set_even".as_ptr();
    unsafe {
        // struct window_pane *wp;
        // struct layout_cell *lc, *lcnew;
        // u_int n, sx, sy;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;

        layout_print_cell((*w).layout_root, __func__, 1);

        /* Get number of panes. */
        let n = window_count_panes(w);
        if (n <= 1) {
            return;
        }

        /* Free the old root and construct a new. */
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        if (type_ == layout_type::LAYOUT_LEFTRIGHT) {
            sx = (n * (PANE_MINIMUM + 1)) - 1;
            if (sx < (*w).sx) {
                sx = (*w).sx;
            }
            sy = (*w).sy;
        } else {
            sy = (n * (PANE_MINIMUM + 1)) - 1;
            if (sy < (*w).sy) {
                sy = (*w).sy;
            }
            sx = (*w).sx;
        }
        layout_set_size(lc, sx, sy, 0, 0);
        layout_make_node(lc, type_);

        /* Build new leaf cells. */
        for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
            let lcnew = layout_create_cell(lc);
            layout_make_leaf(lcnew, wp);
            (*lcnew).sx = (*w).sx;
            (*lcnew).sy = (*w).sy;
            tailq_insert_tail(&raw mut (*lc).cells, lcnew);
        }

        /* Spread out cells. */
        layout_spread_cell(w, lc);

        /* Fix cell offsets. */
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed".as_ptr(), w);
        server_redraw_window(w);
    }
}

unsafe extern "C" fn layout_set_even_h(w: *mut window) {
    unsafe {
        layout_set_even(w, layout_type::LAYOUT_LEFTRIGHT);
    }
}

unsafe extern "C" fn layout_set_even_v(w: *mut window) {
    unsafe {
        layout_set_even(w, layout_type::LAYOUT_TOPBOTTOM);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_main_h(w: *mut window) {
    let __func__ = c"layout_set_main_h".as_ptr();
    unsafe {
        // struct window_pane *wp;
        // struct layout_cell *lc, *lcmain, *lcother, *lcchild;
        // u_int n, mainh, otherh, sx, sy;
        // char *cause;
        // const char *s;
        let mut cause = null_mut();

        layout_print_cell((*w).layout_root, __func__, 1);

        /* Get number of panes. */
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; /* take off main pane */

        /* Find available height - take off one line for the border. */
        let sy = (*w).sy - 1;

        /* Get the main pane height. */
        let mut s = options_get_string((*w).options, c"main-pane-height".as_ptr());
        let mut mainh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
        if (!cause.is_null()) {
            mainh = 24;
            free_(cause);
        }

        let mut otherh: u32 = 0;
        /* Work out the other pane height. */
        if (mainh + PANE_MINIMUM >= sy) {
            if (sy <= PANE_MINIMUM + PANE_MINIMUM) {
                mainh = PANE_MINIMUM;
            } else {
                mainh = sy - PANE_MINIMUM;
            }
            otherh = PANE_MINIMUM;
        } else {
            s = options_get_string((*w).options, c"other-pane-height".as_ptr());
            otherh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
            if (!cause.is_null() || otherh == 0) {
                otherh = sy - mainh;
                free_(cause);
            } else if (otherh > sy || sy - otherh < mainh) {
                otherh = sy - mainh;
            } else {
                mainh = sy - otherh;
            }
        }

        /* Work out what width is needed. */
        let mut sx = (n * (PANE_MINIMUM + 1)) - 1;
        if (sx < (*w).sx) {
            sx = (*w).sx;
        }

        /* Free old tree and create a new root. */
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, sx, mainh + otherh + 1, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_TOPBOTTOM);

        /* Create the main pane. */
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, sx, mainh, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        /* Create the other pane. */
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, sx, otherh, 0, 0);
        if (n == 1) {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_LEFTRIGHT);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            /* Add the remaining panes as children. */
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if (wp == tailq_first(&raw mut (*w).panes)) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, PANE_MINIMUM, otherh, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        /* Fix cell offsets. */
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed".as_ptr(), w);
        server_redraw_window(w);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_main_h_mirrored(w: *mut window) {
    let __func__ = c"layout_set_main_h_mirrored".as_ptr();
    unsafe {
        let mut otherh: u32;
        let mut cause: *mut c_char = null_mut();

        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; // take off main pane

        // Find available height - take off one line for the border.
        let mut sy = (*w).sy - 1;

        // Get the main pane height.
        let s = options_get_string((*w).options, c"main-pane-height".as_ptr());
        let mut mainh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
        if !cause.is_null() {
            mainh = 24;
            free_(cause);
        }

        // Work out the other pane height.
        if mainh + PANE_MINIMUM >= sy {
            if sy <= PANE_MINIMUM + PANE_MINIMUM {
                mainh = PANE_MINIMUM;
            } else {
                mainh = sy - PANE_MINIMUM;
            }
            otherh = PANE_MINIMUM;
        } else {
            let s = options_get_string((*w).options, c"other-pane-height".as_ptr());
            otherh = args_string_percentage(s, 0, sy as i64, sy as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherh == 0 {
                otherh = sy - mainh;
                free_(cause);
            } else if otherh > sy || sy - otherh < mainh {
                otherh = sy - mainh;
            } else {
                mainh = sy - otherh;
            }
        }

        // Work out what width is needed.
        let mut sx = (n * (PANE_MINIMUM + 1)) - 1;
        if sx < (*w).sx {
            sx = (*w).sx;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, sx, mainh + otherh + 1, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_TOPBOTTOM);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, sx, otherh, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_LEFTRIGHT);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, PANE_MINIMUM, otherh, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, sx, mainh, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed".as_ptr(), w);
        server_redraw_window(w);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_main_v(w: *mut window) {
    let __func__ = c"layout_set_main_v".as_ptr();
    let mut cause = null_mut();

    unsafe {
        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; // take off main pane

        // Find available width - take off one line for the border.
        let sx = (*w).sx - 1;

        // Get the main pane width.
        let s = options_get_string((*w).options, c"main-pane-width".as_ptr());
        let mut mainw: u32 = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
        if cause.is_null() {
            mainw = 80;
            free_(cause);
        }

        // Work out the other pane width.
        let mut otherw;
        if mainw + PANE_MINIMUM >= sx {
            if sx <= PANE_MINIMUM + PANE_MINIMUM {
                mainw = PANE_MINIMUM;
            } else {
                mainw = sx - PANE_MINIMUM;
            }
            otherw = PANE_MINIMUM;
        } else {
            let s = options_get_string((*w).options, c"other-pane-width".as_ptr());
            otherw = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherw == 0 {
                otherw = sx - mainw;
                free_(cause);
            } else if otherw > sx || sx - otherw < mainw {
                otherw = sx - mainw;
            } else {
                mainw = sx - otherw;
            }
        }

        // Work out what height is needed.
        let mut sy = (n * (PANE_MINIMUM + 1)) - 1;
        if sy < (*w).sy {
            sy = (*w).sy;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, mainw + otherw + 1, sy, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_LEFTRIGHT);

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, mainw, sy, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, otherw, sy, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_TOPBOTTOM);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, otherw, PANE_MINIMUM, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed".as_ptr(), w);
        server_redraw_window(w);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_main_v_mirrored(w: *mut window) {
    let __func__ = c"layout_set_main_v_mirrored".as_ptr();
    unsafe {
        let mut cause: *mut c_char = null_mut();

        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let mut n = window_count_panes(w);
        if n <= 1 {
            return;
        }
        n -= 1; // take off main pane

        // Find available width - take off one line for the border.
        let sx = (*w).sx - 1;

        // Get the main pane width.
        let s = options_get_string((*w).options, c"main-pane-width".as_ptr());
        let mut mainw = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
        if !cause.is_null() {
            mainw = 80;
            free_(cause);
        }

        // Work out the other pane width.
        let mut otherw: u32;
        if mainw + PANE_MINIMUM >= sx {
            if sx <= PANE_MINIMUM + PANE_MINIMUM {
                mainw = PANE_MINIMUM;
            } else {
                mainw = sx - PANE_MINIMUM;
            }
            otherw = PANE_MINIMUM;
        } else {
            let s = options_get_string((*w).options, c"other-pane-width".as_ptr());
            otherw = args_string_percentage(s, 0, sx as i64, sx as i64, &raw mut cause) as u32;
            if !cause.is_null() || otherw == 0 {
                otherw = sx - mainw;
                free_(cause);
            } else if otherw > sx || sx - otherw < mainw {
                otherw = sx - mainw;
            } else {
                mainw = sx - otherw;
            }
        }

        // Work out what height is needed.
        let mut sy = (n * (PANE_MINIMUM + 1)) - 1;
        if sy < (*w).sy {
            sy = (*w).sy;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        layout_set_size(lc, mainw + otherw + 1, sy, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_LEFTRIGHT);

        // Create the other pane.
        let lcother = layout_create_cell(lc);
        layout_set_size(lcother, otherw, sy, 0, 0);
        if n == 1 {
            let wp = tailq_next::<_, _, discr_entry>(tailq_first(&raw mut (*w).panes));
            layout_make_leaf(lcother, wp);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);
        } else {
            layout_make_node(lcother, layout_type::LAYOUT_TOPBOTTOM);
            tailq_insert_tail(&raw mut (*lc).cells, lcother);

            // Add the remaining panes as children.
            for wp in tailq_foreach::<_, discr_entry>(&raw mut (*w).panes).map(NonNull::as_ptr) {
                if wp == tailq_first(&raw mut (*w).panes) {
                    continue;
                }
                let lcchild = layout_create_cell(lcother);
                layout_set_size(lcchild, otherw, PANE_MINIMUM, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcother).cells, lcchild);
            }
            layout_spread_cell(w, lcother);
        }

        // Create the main pane.
        let lcmain = layout_create_cell(lc);
        layout_set_size(lcmain, mainw, sy, 0, 0);
        layout_make_leaf(lcmain, tailq_first(&raw mut (*w).panes));
        tailq_insert_tail(&raw mut (*lc).cells, lcmain);

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed".as_ptr(), w);
        server_redraw_window(w);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_set_tiled(w: *mut window) {
    let __func__ = c"layout_set_tiled".as_ptr();

    unsafe {
        layout_print_cell((*w).layout_root, __func__, 1);

        // Get number of panes.
        let n = window_count_panes(w);
        if n <= 1 {
            return;
        }

        // How many rows and columns are wanted?
        let mut rows = 1;
        let mut columns = 1;
        while rows * columns < n {
            rows += 1;
            if rows * columns < n {
                columns += 1;
            }
        }

        // What width and height should they be?
        let mut width = ((*w).sx - (columns - 1)) / columns;
        if width < PANE_MINIMUM {
            width = PANE_MINIMUM;
        }
        let mut height = ((*w).sy - (rows - 1)) / rows;
        if height < PANE_MINIMUM {
            height = PANE_MINIMUM;
        }

        // Free old tree and create a new root.
        layout_free(w);
        let lc = layout_create_cell(null_mut());
        (*w).layout_root = lc;
        let mut sx = ((width + 1) * columns) - 1;
        if sx < (*w).sx {
            sx = (*w).sx;
        }
        let mut sy = ((height + 1) * rows) - 1;
        if sy < (*w).sy {
            sy = (*w).sy;
        }
        layout_set_size(lc, sx, sy, 0, 0);
        layout_make_node(lc, layout_type::LAYOUT_TOPBOTTOM);

        // Create a grid of the cells.
        let mut wp = tailq_first(&raw mut (*w).panes);
        for j in 0..rows {
            // If this is the last cell, all done.
            if wp.is_null() {
                break;
            }

            // Create the new row.
            let lcrow = layout_create_cell(lc);
            layout_set_size(lcrow, (*w).sx, height, 0, 0);
            tailq_insert_tail(&raw mut (*lc).cells, lcrow);

            // If only one column, just use the row directly.
            if n - (j * columns) == 1 || columns == 1 {
                layout_make_leaf(lcrow, wp);
                wp = tailq_next::<_, _, discr_entry>(wp);
                continue;
            }

            // Add in the columns.
            layout_make_node(lcrow, layout_type::LAYOUT_LEFTRIGHT);
            let mut i = 0;
            for i_ in 0..columns {
                i = i_;
                // Create and add a pane cell.
                let lcchild = layout_create_cell(lcrow);
                layout_set_size(lcchild, width, height, 0, 0);
                layout_make_leaf(lcchild, wp);
                tailq_insert_tail(&raw mut (*lcrow).cells, lcchild);

                // Move to the next cell.
                wp = tailq_next::<_, _, discr_entry>(wp);
                if wp.is_null() {
                    break;
                }
                i += 1;
            }

            // Adjust the row and columns to fit the full width if necessary.
            if i == columns {
                i -= 1;
            }
            let used = ((i + 1) * (width + 1)) - 1;
            if (*w).sx <= used {
                continue;
            }
            let lcchild = tailq_last(&raw mut (*lcrow).cells);
            layout_resize_adjust(w, lcchild, layout_type::LAYOUT_LEFTRIGHT, ((*w).sx - used) as i32);
        }

        // Adjust the last row height to fit if necessary.
        let used = (rows * height) + rows - 1;
        if (*w).sy > used {
            let lcrow = tailq_last(&raw mut (*lc).cells);
            layout_resize_adjust(w, lcrow, layout_type::LAYOUT_TOPBOTTOM, ((*w).sy - used) as i32);
        }

        // Fix cell offsets.
        layout_fix_offsets(w);
        layout_fix_panes(w, null_mut());

        layout_print_cell((*w).layout_root, __func__, 1);

        window_resize(w, (*lc).sx, (*lc).sy, -1, -1);
        notify_window(c"window-layout-changed".as_ptr(), w);
        server_redraw_window(w);
    }
}


*/


#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_create_cell(lcparent: *mut layout_cell) -> *mut layout_cell {
    let lc = xmalloc_::<layout_cell>().as_ptr();
    (*lc).type_ = layout_type::LAYOUT_WINDOWPANE;
    (*lc).parent = lcparent;

    tailq_init(&raw mut (*lc).cells);

    (*lc).sx = u32::MAX;
    (*lc).sy = u32::MAX;

    (*lc).xoff = u32::MAX;
    (*lc).yoff = u32::MAX;

    (*lc).wp = null_mut();

    lc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_free_cell(lc: *mut layout_cell) {
    unsafe {
        match (*lc).type_ {
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                while !tailq_empty(&raw mut (*lc).cells) {
                    let lcchild = tailq_first(&raw mut (*lc).cells);
                    tailq_remove(&raw mut (*lc).cells, lcchild);
                    layout_free_cell(lcchild);
                }
            }
            layout_type::LAYOUT_WINDOWPANE => {
                if !(*lc).wp.is_null() {
                    (*(*lc).wp).layout_cell = null_mut();
                }
            }
        }

        free_(lc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_print_cell(lc: *mut layout_cell, hdr: *const c_char, n: u32) {
  unsafe {
    let type_str = match (*lc).type_ {
        layout_type::LAYOUT_LEFTRIGHT => c"LEFTRIGHT",
        layout_type::LAYOUT_TOPBOTTOM => c"TOPBOTTOM", 
        layout_type::LAYOUT_WINDOWPANE => c"WINDOWPANE",
        _ => c"UNKNOWN"
    };

    log_debug(
        c"%s:%*s%p type %s [parent %p] wp=%p [%u,%u %ux%u]".as_ptr(),
        hdr,
        n,
        c" ".as_ptr(),
        lc as *mut c_void,
        type_str.as_ptr(),
        (*lc).parent as *mut c_void,
        (*lc).wp as *mut c_void,
        (*lc).xoff,
        (*lc).yoff,
        (*lc).sx,
        (*lc).sy
    );

    match (*lc).type_ {
        layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
            for lcchild in tailq_foreach(&raw mut (*lc).cells) {
                layout_print_cell(lcchild.as_ptr(), hdr, n + 1);
            }
        }
        layout_type::LAYOUT_WINDOWPANE => ()
    }
  }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_search_by_border(lc: *mut layout_cell, x: u32, y: u32) -> *mut layout_cell {
  unsafe {
    let mut last: *mut layout_cell = null_mut();

    for lcchild in tailq_foreach(&raw mut (*lc).cells) {
        let lcchild = lcchild.as_ptr();
        
        if x >= (*lcchild).xoff && x < (*lcchild).xoff + (*lcchild).sx &&
           y >= (*lcchild).yoff && y < (*lcchild).yoff + (*lcchild).sy {
            // Inside the cell - recurse
            return layout_search_by_border(lcchild, x, y);
        }

        if last.is_null() {
            last = lcchild;
            continue;
        }

        match (*lc).type_ {
            layout_type::LAYOUT_LEFTRIGHT => {
                if x < (*lcchild).xoff && x >= (*last).xoff + (*last).sx {
                    return last;
                }
            }
            layout_type::LAYOUT_TOPBOTTOM => {
                if y < (*lcchild).yoff && y >= (*last).yoff + (*last).sy {
                    return last;
                }
            }
            layout_type::LAYOUT_WINDOWPANE => (),
        }

        last = lcchild;
    }

    null_mut()
  }
}

/*

void layout_set_size(struct layout_cell *lc, u_int sx, u_int sy, u_int xoff,
                     u_int yoff) {
  lc->sx = sx;
  lc->sy = sy;

  lc->xoff = xoff;
  lc->yoff = yoff;
}

void layout_make_leaf(struct layout_cell *lc, struct window_pane *wp) {
  lc->type = LAYOUT_WINDOWPANE;

  TAILQ_INIT(&lc->cells);

  wp->layout_cell = lc;
  lc->wp = wp;
}

void layout_make_node(struct layout_cell *lc, enum layout_type type) {
  if (type == LAYOUT_WINDOWPANE) {
    fatalx("bad layout type");
  }
  lc->type = type;

  TAILQ_INIT(&lc->cells);

  if (lc->wp != NULL) {
    lc->wp->layout_cell = NULL;
  }
  lc->wp = NULL;
}

/* Fix cell offsets for a child cell. */
static void layout_fix_offsets1(struct layout_cell *lc) {
  struct layout_cell *lcchild;
  u_int xoff, yoff;

  if (lc->type == LAYOUT_LEFTRIGHT) {
    xoff = lc->xoff;
    TAILQ_FOREACH(lcchild, &lc->cells, entry) {
      lcchild->xoff = xoff;
      lcchild->yoff = lc->yoff;
      if (lcchild->type != LAYOUT_WINDOWPANE) {
        layout_fix_offsets1(lcchild);
      }
      xoff += lcchild->sx + 1;
    }
  } else {
    yoff = lc->yoff;
    TAILQ_FOREACH(lcchild, &lc->cells, entry) {
      lcchild->xoff = lc->xoff;
      lcchild->yoff = yoff;
      if (lcchild->type != LAYOUT_WINDOWPANE) {
        layout_fix_offsets1(lcchild);
      }
      yoff += lcchild->sy + 1;
    }
  }
}

/* Update cell offsets based on their sizes. */
void layout_fix_offsets(struct window *w) {
  struct layout_cell *lc = w->layout_root;

  lc->xoff = 0;
  lc->yoff = 0;

  layout_fix_offsets1(lc);
}

/* Is this a top cell? */
static int layout_cell_is_top(struct window *w, struct layout_cell *lc) {
  struct layout_cell *next;

  while (lc != w->layout_root) {
    next = lc->parent;
    if (next->type == LAYOUT_TOPBOTTOM && lc != TAILQ_FIRST(&next->cells)) {
      return 0;
    }
    lc = next;
  }
  return 1;
}

/* Is this a bottom cell? */
static int layout_cell_is_bottom(struct window *w, struct layout_cell *lc) {
  struct layout_cell *next;

  while (lc != w->layout_root) {
    next = lc->parent;
    if (next->type == LAYOUT_TOPBOTTOM &&
        lc != TAILQ_LAST(&next->cells, layout_cells)) {
      return 0;
    }
    lc = next;
  }
  return 1;
}

/*
 * Returns 1 if we need to add an extra line for the pane status line. This is
 * the case for the most upper or lower panes only.
 */
static int layout_add_border(struct window *w, struct layout_cell *lc,
                             int status) {
  if (status == PANE_STATUS_TOP) {
    return layout_cell_is_top(w, lc);
  }
  if (status == PANE_STATUS_BOTTOM) {
    return layout_cell_is_bottom(w, lc);
  }
  return 0;
}

/* Update pane offsets and sizes based on their cells. */
void layout_fix_panes(struct window *w, struct window_pane *skip) {
  struct window_pane *wp;
  struct layout_cell *lc;
  int status;

  status = options_get_number(w->options, "pane-border-status");
  TAILQ_FOREACH(wp, &w->panes, entry) {
    if ((lc = wp->layout_cell) == NULL || wp == skip) {
      continue;
    }

    wp->xoff = lc->xoff;
    wp->yoff = lc->yoff;

    if (layout_add_border(w, lc, status)) {
      if (status == PANE_STATUS_TOP) {
        wp->yoff++;
      }
      window_pane_resize(wp, lc->sx, lc->sy - 1);
    } else {
      window_pane_resize(wp, lc->sx, lc->sy);
    }
  }
}

/* Count the number of available cells in a layout. */
u_int layout_count_cells(struct layout_cell *lc) {
  struct layout_cell *lcchild;
  u_int count;

  switch (lc->type) {
  case LAYOUT_WINDOWPANE:
    return 1;
  case LAYOUT_LEFTRIGHT:
  case LAYOUT_TOPBOTTOM:
    count = 0;
    TAILQ_FOREACH(lcchild, &lc->cells, entry)
    count += layout_count_cells(lcchild);
    return count;
  default:
    fatalx("bad layout type");
  }
}

/* Calculate how much size is available to be removed from a cell. */
static u_int layout_resize_check(struct window *w, struct layout_cell *lc,
                                 enum layout_type type) {
  struct layout_cell *lcchild;
  u_int available, minimum;
  int status;

  status = options_get_number(w->options, "pane-border-status");
  if (lc->type == LAYOUT_WINDOWPANE) {
    /* Space available in this cell only. */
    if (type == LAYOUT_LEFTRIGHT) {
      available = lc->sx;
      minimum = PANE_MINIMUM;
    } else {
      available = lc->sy;
      if (layout_add_border(w, lc, status)) {
        minimum = PANE_MINIMUM + 1;
      } else {
        minimum = PANE_MINIMUM;
      }
    }
    if (available > minimum) {
      available -= minimum;
    } else {
      available = 0;
    }
  } else if (lc->type == type) {
    /* Same type: total of available space in all child cells. */
    available = 0;
    TAILQ_FOREACH(lcchild, &lc->cells, entry)
    available += layout_resize_check(w, lcchild, type);
  } else {
    /* Different type: minimum of available space in child cells. */
    minimum = UINT_MAX;
    TAILQ_FOREACH(lcchild, &lc->cells, entry) {
      available = layout_resize_check(w, lcchild, type);
      if (available < minimum) {
        minimum = available;
      }
    }
    available = minimum;
  }

  return available;
}

/*
 * Adjust cell size evenly, including altering its children. This function
 * expects the change to have already been bounded to the space available.
 */
void layout_resize_adjust(struct window *w, struct layout_cell *lc,
                          enum layout_type type, int change) {
  struct layout_cell *lcchild;

  /* Adjust the cell size. */
  if (type == LAYOUT_LEFTRIGHT) {
    lc->sx += change;
  } else {
    lc->sy += change;
  }

  /* If this is a leaf cell, that is all that is necessary. */
  if (type == LAYOUT_WINDOWPANE) {
    return;
  }

  /* Child cell runs in a different direction. */
  if (lc->type != type) {
    TAILQ_FOREACH(lcchild, &lc->cells, entry)
    layout_resize_adjust(w, lcchild, type, change);
    return;
  }

  /*
   * Child cell runs in the same direction. Adjust each child equally
   * until no further change is possible.
   */
  while (change != 0) {
    TAILQ_FOREACH(lcchild, &lc->cells, entry) {
      if (change == 0) {
        break;
      }
      if (change > 0) {
        layout_resize_adjust(w, lcchild, type, 1);
        change--;
        continue;
      }
      if (layout_resize_check(w, lcchild, type) > 0) {
        layout_resize_adjust(w, lcchild, type, -1);
        change++;
      }
    }
  }
}

/* Destroy a cell and redistribute the space. */
void layout_destroy_cell(struct window *w, struct layout_cell *lc,
                         struct layout_cell **lcroot) {
  struct layout_cell *lcother, *lcparent;

  /*
   * If no parent, this is the last pane so window close is imminent and
   * there is no need to resize anything.
   */
  lcparent = lc->parent;
  if (lcparent == NULL) {
    layout_free_cell(lc);
    *lcroot = NULL;
    return;
  }

  /* Merge the space into the previous or next cell. */
  if (lc == TAILQ_FIRST(&lcparent->cells)) {
    lcother = TAILQ_NEXT(lc, entry);
  } else {
    lcother = TAILQ_PREV(lc, layout_cells, entry);
  }
  if (lcother != NULL && lcparent->type == LAYOUT_LEFTRIGHT) {
    layout_resize_adjust(w, lcother, lcparent->type, lc->sx + 1);
  } else if (lcother != NULL) {
    layout_resize_adjust(w, lcother, lcparent->type, lc->sy + 1);
  }

  /* Remove this from the parent's list. */
  TAILQ_REMOVE(&lcparent->cells, lc, entry);
  layout_free_cell(lc);

  /*
   * If the parent now has one cell, remove the parent from the tree and
   * replace it by that cell.
   */
  lc = TAILQ_FIRST(&lcparent->cells);
  if (TAILQ_NEXT(lc, entry) == NULL) {
    TAILQ_REMOVE(&lcparent->cells, lc, entry);

    lc->parent = lcparent->parent;
    if (lc->parent == NULL) {
      lc->xoff = 0;
      lc->yoff = 0;
      *lcroot = lc;
    } else {
      TAILQ_REPLACE(&lc->parent->cells, lcparent, lc, entry);
    }

    layout_free_cell(lcparent);
  }
}

void layout_init(struct window *w, struct window_pane *wp) {
  struct layout_cell *lc;

  lc = w->layout_root = layout_create_cell(NULL);
  layout_set_size(lc, w->sx, w->sy, 0, 0);
  layout_make_leaf(lc, wp);
  layout_fix_panes(w, NULL);
}

void layout_free(struct window *w) { layout_free_cell(w->layout_root); }

/* Resize the entire layout after window resize. */
void layout_resize(struct window *w, u_int sx, u_int sy) {
  struct layout_cell *lc = w->layout_root;
  int xlimit, ylimit, xchange, ychange;

  /*
   * Adjust horizontally. Do not attempt to reduce the layout lower than
   * the minimum (more than the amount returned by layout_resize_check).
   *
   * This can mean that the window size is smaller than the total layout
   * size: redrawing this is handled at a higher level, but it does leave
   * a problem with growing the window size here: if the current size is
   * < the minimum, growing proportionately by adding to each pane is
   * wrong as it would keep the layout size larger than the window size.
   * Instead, spread the difference between the minimum and the new size
   * out proportionately - this should leave the layout fitting the new
   * window size.
   */
  xchange = sx - lc->sx;
  xlimit = layout_resize_check(w, lc, LAYOUT_LEFTRIGHT);
  if (xchange < 0 && xchange < -xlimit) {
    xchange = -xlimit;
  }
  if (xlimit == 0) {
    if (sx <= lc->sx) { /* lc->sx is minimum possible */
      xchange = 0;
    } else {
      xchange = sx - lc->sx;
    }
  }
  if (xchange != 0) {
    layout_resize_adjust(w, lc, LAYOUT_LEFTRIGHT, xchange);
  }

  /* Adjust vertically in a similar fashion. */
  ychange = sy - lc->sy;
  ylimit = layout_resize_check(w, lc, LAYOUT_TOPBOTTOM);
  if (ychange < 0 && ychange < -ylimit) {
    ychange = -ylimit;
  }
  if (ylimit == 0) {
    if (sy <= lc->sy) { /* lc->sy is minimum possible */
      ychange = 0;
    } else {
      ychange = sy - lc->sy;
    }
  }
  if (ychange != 0) {
    layout_resize_adjust(w, lc, LAYOUT_TOPBOTTOM, ychange);
  }

  /* Fix cell offsets. */
  layout_fix_offsets(w);
  layout_fix_panes(w, NULL);
}

/* Resize a pane to an absolute size. */
void layout_resize_pane_to(struct window_pane *wp, enum layout_type type,
                           u_int new_size) {
  struct layout_cell *lc, *lcparent;
  int change, size;

  lc = wp->layout_cell;

  /* Find next parent of the same type. */
  lcparent = lc->parent;
  while (lcparent != NULL && lcparent->type != type) {
    lc = lcparent;
    lcparent = lc->parent;
  }
  if (lcparent == NULL) {
    return;
  }

  /* Work out the size adjustment. */
  if (type == LAYOUT_LEFTRIGHT) {
    size = lc->sx;
  } else {
    size = lc->sy;
  }
  if (lc == TAILQ_LAST(&lcparent->cells, layout_cells)) {
    change = size - new_size;
  } else {
    change = new_size - size;
  }

  /* Resize the pane. */
  layout_resize_pane(wp, type, change, 1);
}

void layout_resize_layout(struct window *w, struct layout_cell *lc,
                          enum layout_type type, int change, int opposite) {
  int needed, size;

  /* Grow or shrink the cell. */
  needed = change;
  while (needed != 0) {
    if (change > 0) {
      size = layout_resize_pane_grow(w, lc, type, needed, opposite);
      needed -= size;
    } else {
      size = layout_resize_pane_shrink(w, lc, type, needed);
      needed += size;
    }

    if (size == 0) { /* no more change possible */
      break;
    }
  }

  /* Fix cell offsets. */
  layout_fix_offsets(w);
  layout_fix_panes(w, NULL);
  notify_window("window-layout-changed", w);
}

/* Resize a single pane within the layout. */
void layout_resize_pane(struct window_pane *wp, enum layout_type type,
                        int change, int opposite) {
  struct layout_cell *lc, *lcparent;

  lc = wp->layout_cell;

  /* Find next parent of the same type. */
  lcparent = lc->parent;
  while (lcparent != NULL && lcparent->type != type) {
    lc = lcparent;
    lcparent = lc->parent;
  }
  if (lcparent == NULL) {
    return;
  }

  /* If this is the last cell, move back one. */
  if (lc == TAILQ_LAST(&lcparent->cells, layout_cells)) {
    lc = TAILQ_PREV(lc, layout_cells, entry);
  }

  layout_resize_layout(wp->window, lc, type, change, opposite);
}

/* Helper function to grow pane. */
static int layout_resize_pane_grow(struct window *w, struct layout_cell *lc,
                                   enum layout_type type, int needed,
                                   int opposite) {
  struct layout_cell *lcadd, *lcremove;
  u_int size = 0;

  /* Growing. Always add to the current cell. */
  lcadd = lc;

  /* Look towards the tail for a suitable cell for reduction. */
  lcremove = TAILQ_NEXT(lc, entry);
  while (lcremove != NULL) {
    size = layout_resize_check(w, lcremove, type);
    if (size > 0) {
      break;
    }
    lcremove = TAILQ_NEXT(lcremove, entry);
  }

  /* If none found, look towards the head. */
  if (opposite && lcremove == NULL) {
    lcremove = TAILQ_PREV(lc, layout_cells, entry);
    while (lcremove != NULL) {
      size = layout_resize_check(w, lcremove, type);
      if (size > 0) {
        break;
      }
      lcremove = TAILQ_PREV(lcremove, layout_cells, entry);
    }
  }
  if (lcremove == NULL) {
    return 0;
  }

  /* Change the cells. */
  if (size > (u_int)needed) {
    size = needed;
  }
  layout_resize_adjust(w, lcadd, type, size);
  layout_resize_adjust(w, lcremove, type, -size);
  return size;
}

/* Helper function to shrink pane. */
static int layout_resize_pane_shrink(struct window *w, struct layout_cell *lc,
                                     enum layout_type type, int needed) {
  struct layout_cell *lcadd, *lcremove;
  u_int size;

  /* Shrinking. Find cell to remove from by walking towards head. */
  lcremove = lc;
  do {
    size = layout_resize_check(w, lcremove, type);
    if (size != 0) {
      break;
    }
    lcremove = TAILQ_PREV(lcremove, layout_cells, entry);
  } while (lcremove != NULL);
  if (lcremove == NULL) {
    return 0;
  }

  /* And add onto the next cell (from the original cell). */
  lcadd = TAILQ_NEXT(lc, entry);
  if (lcadd == NULL) {
    return 0;
  }

  /* Change the cells. */
  if (size > (u_int)-needed) {
    size = -needed;
  }
  layout_resize_adjust(w, lcadd, type, size);
  layout_resize_adjust(w, lcremove, type, -size);
  return size;
}

/* Assign window pane to newly split cell. */
void layout_assign_pane(struct layout_cell *lc, struct window_pane *wp,
                        int do_not_resize) {
  layout_make_leaf(lc, wp);
  if (do_not_resize) {
    layout_fix_panes(wp->window, wp);
  } else {
    layout_fix_panes(wp->window, NULL);
  }
}

/* Calculate the new pane size for resized parent. */
static u_int layout_new_pane_size(struct window *w, u_int previous,
                                  struct layout_cell *lc, enum layout_type type,
                                  u_int size, u_int count_left,
                                  u_int size_left) {
  u_int new_size, min, max, available;

  /* If this is the last cell, it can take all of the remaining size. */
  if (count_left == 1) {
    return size_left;
  }

  /* How much is available in this parent? */
  available = layout_resize_check(w, lc, type);

  /*
   * Work out the minimum size of this cell and the new size
   * proportionate to the previous size.
   */
  min = (PANE_MINIMUM + 1) * (count_left - 1);
  if (type == LAYOUT_LEFTRIGHT) {
    if (lc->sx - available > min) {
      min = lc->sx - available;
    }
    new_size = (lc->sx * size) / previous;
  } else {
    if (lc->sy - available > min) {
      min = lc->sy - available;
    }
    new_size = (lc->sy * size) / previous;
  }

  /* Check against the maximum and minimum size. */
  max = size_left - min;
  if (new_size > max) {
    new_size = max;
  }
  if (new_size < PANE_MINIMUM) {
    new_size = PANE_MINIMUM;
  }
  return new_size;
}

/* Check if the cell and all its children can be resized to a specific size. */
static int layout_set_size_check(struct window *w, struct layout_cell *lc,
                                 enum layout_type type, int size) {
  struct layout_cell *lcchild;
  u_int new_size, available, previous, count, idx;

  /* Cells with no children must just be bigger than minimum. */
  if (lc->type == LAYOUT_WINDOWPANE) {
    return size >= PANE_MINIMUM;
  }
  available = size;

  /* Count number of children. */
  count = 0;
  TAILQ_FOREACH(lcchild, &lc->cells, entry)
  count++;

  /* Check new size will work for each child. */
  if (lc->type == type) {
    if (available < (count * 2) - 1) {
      return 0;
    }

    if (type == LAYOUT_LEFTRIGHT) {
      previous = lc->sx;
    } else {
      previous = lc->sy;
    }

    idx = 0;
    TAILQ_FOREACH(lcchild, &lc->cells, entry) {
      new_size = layout_new_pane_size(w, previous, lcchild, type, size,
                                      count - idx, available);
      if (idx == count - 1) {
        if (new_size > available) {
          return 0;
        }
        available -= new_size;
      } else {
        if (new_size + 1 > available) {
          return 0;
        }
        available -= new_size + 1;
      }
      if (!layout_set_size_check(w, lcchild, type, new_size)) {
        return 0;
      }
      idx++;
    }
  } else {
    TAILQ_FOREACH(lcchild, &lc->cells, entry) {
      if (lcchild->type == LAYOUT_WINDOWPANE) {
        continue;
      }
      if (!layout_set_size_check(w, lcchild, type, size)) {
        return 0;
      }
    }
  }

  return 1;
}

/* Resize all child cells to fit within the current cell. */
static void layout_resize_child_cells(struct window *w,
                                      struct layout_cell *lc) {
  struct layout_cell *lcchild;
  u_int previous, available, count, idx;

  if (lc->type == LAYOUT_WINDOWPANE) {
    return;
  }

  /* What is the current size used? */
  count = 0;
  previous = 0;
  TAILQ_FOREACH(lcchild, &lc->cells, entry) {
    count++;
    if (lc->type == LAYOUT_LEFTRIGHT) {
      previous += lcchild->sx;
    } else if (lc->type == LAYOUT_TOPBOTTOM) {
      previous += lcchild->sy;
    }
  }
  previous += (count - 1);

  /* And how much is available? */
  available = 0;
  if (lc->type == LAYOUT_LEFTRIGHT) {
    available = lc->sx;
  } else if (lc->type == LAYOUT_TOPBOTTOM) {
    available = lc->sy;
  }

  /* Resize children into the new size. */
  idx = 0;
  TAILQ_FOREACH(lcchild, &lc->cells, entry) {
    if (lc->type == LAYOUT_TOPBOTTOM) {
      lcchild->sx = lc->sx;
      lcchild->xoff = lc->xoff;
    } else {
      lcchild->sx = layout_new_pane_size(w, previous, lcchild, lc->type, lc->sx,
                                         count - idx, available);
      available -= (lcchild->sx + 1);
    }
    if (lc->type == LAYOUT_LEFTRIGHT) {
      lcchild->sy = lc->sy;
    } else {
      lcchild->sy = layout_new_pane_size(w, previous, lcchild, lc->type, lc->sy,
                                         count - idx, available);
      available -= (lcchild->sy + 1);
    }
    layout_resize_child_cells(w, lcchild);
    idx++;
  }
}

/*
 * Split a pane into two. size is a hint, or -1 for default half/half
 * split. This must be followed by layout_assign_pane before much else happens!
 */
struct layout_cell *layout_split_pane(struct window_pane *wp,
                                      enum layout_type type, int size,
                                      int flags) {
  struct layout_cell *lc, *lcparent, *lcnew, *lc1, *lc2;
  u_int sx, sy, xoff, yoff, size1, size2, minimum;
  u_int new_size, saved_size, resize_first = 0;
  int full_size = (flags & SPAWN_FULLSIZE), status;

  /*
   * If full_size is specified, add a new cell at the top of the window
   * layout. Otherwise, split the cell for the current pane.
   */
  if (full_size) {
    lc = wp->window->layout_root;
  } else {
    lc = wp->layout_cell;
  }
  status = options_get_number(wp->window->options, "pane-border-status");

  /* Copy the old cell size. */
  sx = lc->sx;
  sy = lc->sy;
  xoff = lc->xoff;
  yoff = lc->yoff;

  /* Check there is enough space for the two new panes. */
  switch (type) {
  case LAYOUT_LEFTRIGHT:
    if (sx < PANE_MINIMUM * 2 + 1) {
      return NULL;
    }
    break;
  case LAYOUT_TOPBOTTOM:
    if (layout_add_border(wp->window, lc, status)) {
      minimum = PANE_MINIMUM * 2 + 2;
    } else {
      minimum = PANE_MINIMUM * 2 + 1;
    }
    if (sy < minimum) {
      return NULL;
    }
    break;
  default:
    fatalx("bad layout type");
  }

  /*
   * Calculate new cell sizes. size is the target size or -1 for middle
   * split, size1 is the size of the top/left and size2 the bottom/right.
   */
  if (type == LAYOUT_LEFTRIGHT) {
    saved_size = sx;
  } else {
    saved_size = sy;
  }
  if (size < 0) {
    size2 = ((saved_size + 1) / 2) - 1;
  } else if (flags & SPAWN_BEFORE) {
    size2 = saved_size - size - 1;
  } else {
    size2 = size;
  }
  if (size2 < PANE_MINIMUM) {
    size2 = PANE_MINIMUM;
  } else if (size2 > saved_size - 2) {
    size2 = saved_size - 2;
  }
  size1 = saved_size - 1 - size2;

  /* Which size are we using? */
  if (flags & SPAWN_BEFORE) {
    new_size = size2;
  } else {
    new_size = size1;
  }

  /* Confirm there is enough space for full size pane. */
  if (full_size && !layout_set_size_check(wp->window, lc, type, new_size)) {
    return NULL;
  }

  if (lc->parent != NULL && lc->parent->type == type) {
    /*
     * If the parent exists and is of the same type as the split,
     * create a new cell and insert it after this one.
     */
    lcparent = lc->parent;
    lcnew = layout_create_cell(lcparent);
    if (flags & SPAWN_BEFORE) {
      TAILQ_INSERT_BEFORE(lc, lcnew, entry);
    } else {
      TAILQ_INSERT_AFTER(&lcparent->cells, lc, lcnew, entry);
    }
  } else if (full_size && lc->parent == NULL && lc->type == type) {
    /*
     * If the new full size pane is the same type as the root
     * split, insert the new pane under the existing root cell
     * instead of creating a new root cell. The existing layout
     * must be resized before inserting the new cell.
     */
    if (lc->type == LAYOUT_LEFTRIGHT) {
      lc->sx = new_size;
      layout_resize_child_cells(wp->window, lc);
      lc->sx = saved_size;
    } else if (lc->type == LAYOUT_TOPBOTTOM) {
      lc->sy = new_size;
      layout_resize_child_cells(wp->window, lc);
      lc->sy = saved_size;
    }
    resize_first = 1;

    /* Create the new cell. */
    lcnew = layout_create_cell(lc);
    size = saved_size - 1 - new_size;
    if (lc->type == LAYOUT_LEFTRIGHT) {
      layout_set_size(lcnew, size, sy, 0, 0);
    } else if (lc->type == LAYOUT_TOPBOTTOM) {
      layout_set_size(lcnew, sx, size, 0, 0);
    }
    if (flags & SPAWN_BEFORE) {
      TAILQ_INSERT_HEAD(&lc->cells, lcnew, entry);
    } else {
      TAILQ_INSERT_TAIL(&lc->cells, lcnew, entry);
    }
  } else {
    /*
     * Otherwise create a new parent and insert it.
     */

    /* Create and insert the replacement parent. */
    lcparent = layout_create_cell(lc->parent);
    layout_make_node(lcparent, type);
    layout_set_size(lcparent, sx, sy, xoff, yoff);
    if (lc->parent == NULL) {
      wp->window->layout_root = lcparent;
    } else {
      TAILQ_REPLACE(&lc->parent->cells, lc, lcparent, entry);
    }

    /* Insert the old cell. */
    lc->parent = lcparent;
    TAILQ_INSERT_HEAD(&lcparent->cells, lc, entry);

    /* Create the new child cell. */
    lcnew = layout_create_cell(lcparent);
    if (flags & SPAWN_BEFORE) {
      TAILQ_INSERT_HEAD(&lcparent->cells, lcnew, entry);
    } else {
      TAILQ_INSERT_TAIL(&lcparent->cells, lcnew, entry);
    }
  }
  if (flags & SPAWN_BEFORE) {
    lc1 = lcnew;
    lc2 = lc;
  } else {
    lc1 = lc;
    lc2 = lcnew;
  }

  /*
   * Set new cell sizes. size1 is the size of the top/left and size2 the
   * bottom/right.
   */
  if (!resize_first && type == LAYOUT_LEFTRIGHT) {
    layout_set_size(lc1, size1, sy, xoff, yoff);
    layout_set_size(lc2, size2, sy, xoff + lc1->sx + 1, yoff);
  } else if (!resize_first && type == LAYOUT_TOPBOTTOM) {
    layout_set_size(lc1, sx, size1, xoff, yoff);
    layout_set_size(lc2, sx, size2, xoff, yoff + lc1->sy + 1);
  }
  if (full_size) {
    if (!resize_first) {
      layout_resize_child_cells(wp->window, lc);
    }
    layout_fix_offsets(wp->window);
  } else {
    layout_make_leaf(lc, wp);
  }

  return lcnew;
}

/* Destroy the cell associated with a pane. */
void layout_close_pane(struct window_pane *wp) {
  struct window *w = wp->window;

  /* Remove the cell. */
  layout_destroy_cell(w, wp->layout_cell, &w->layout_root);

  /* Fix pane offsets and sizes. */
  if (w->layout_root != NULL) {
    layout_fix_offsets(w);
    layout_fix_panes(w, NULL);
  }
  notify_window("window-layout-changed", w);
}

int layout_spread_cell(struct window *w, struct layout_cell *parent) {
  struct layout_cell *lc;
  u_int number, each, size, this;
  int change, changed, status;

  number = 0;
  TAILQ_FOREACH(lc, &parent->cells, entry)
  number++;
  if (number <= 1) {
    return 0;
  }
  status = options_get_number(w->options, "pane-border-status");

  if (parent->type == LAYOUT_LEFTRIGHT) {
    size = parent->sx;
  } else if (parent->type == LAYOUT_TOPBOTTOM) {
    if (layout_add_border(w, parent, status)) {
      size = parent->sy - 1;
    } else {
      size = parent->sy;
    }
  } else {
    return 0;
  }
  if (size < number - 1) {
    return 0;
  }
  each = (size - (number - 1)) / number;
  if (each == 0) {
    return 0;
  }

  changed = 0;
  TAILQ_FOREACH(lc, &parent->cells, entry) {
    if (TAILQ_NEXT(lc, entry) == NULL) {
      each = size - ((each + 1) * (number - 1));
    }
    change = 0;
    if (parent->type == LAYOUT_LEFTRIGHT) {
      change = each - (int)lc->sx;
      layout_resize_adjust(w, lc, LAYOUT_LEFTRIGHT, change);
    } else if (parent->type == LAYOUT_TOPBOTTOM) {
      if (layout_add_border(w, lc, status)) {
        this = each + 1;
      } else {
        this = each;
      }
      change = this - (int)lc->sy;
      layout_resize_adjust(w, lc, LAYOUT_TOPBOTTOM, change);
    }
    if (change != 0) {
      changed = 1;
    }
  }
  return changed;
}

void layout_spread_out(struct window_pane *wp) {
  struct layout_cell *parent;
  struct window *w = wp->window;

  parent = wp->layout_cell->parent;
  if (parent == NULL) {
    return;
  }

  do {
    if (layout_spread_cell(w, parent)) {
      layout_fix_offsets(w);
      layout_fix_panes(w, NULL);
      break;
    }
  } while ((parent = parent->parent) != NULL);
}
*/
