// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::options_::*;

/// Selected area in screen.
#[derive(Clone)]
pub struct screen_sel {
    pub hidden: i32,
    pub rectangle: i32,
    pub modekeys: modekey,

    pub sx: u32,
    pub sy: u32,

    pub ex: u32,
    pub ey: u32,

    pub cell: grid_cell,
}

/// Free titles stack.
pub unsafe fn screen_free_titles(s: *mut screen) {
    unsafe {
        (*s).titles.clear();
    }
}

/// Return a valid but uninitialized-equivalent screen placeholder.
///
/// Every field is set to a safe zero/null/None value.  The caller must
/// follow up with `screen_init` (which overwrites via `ptr::write`)
/// before using the screen.  This exists so that struct literals can
/// embed a `screen` field without `zeroed()`.
pub fn screen_placeholder() -> screen {
    screen {
        title: CString::default(),
        path: None,
        titles: Vec::new(),
        grid: grid_create(0, 0, 0),
        cx: 0,
        cy: 0,
        cstyle: screen_cursor_style::SCREEN_CURSOR_DEFAULT,
        default_cstyle: screen_cursor_style::SCREEN_CURSOR_DEFAULT,
        ccolour: 0,
        default_ccolour: 0,
        rupper: 0,
        rlower: 0,
        mode: mode_flag::empty(),
        default_mode: mode_flag::empty(),
        saved_cx: 0,
        saved_cy: 0,
        saved_grid: None,
        saved_cell: unsafe { zeroed() },
        saved_flags: 0,
        tabs: None,
        sel: None,
        #[cfg(feature = "sixel")]
        images: Vec::new(),
        write_list: None,
        hyperlinks: None,
    }
}

/// Create a new screen.
pub unsafe fn screen_init(s: *mut screen, sx: u32, sy: u32, hlimit: u32) {
    unsafe {
        // Use ptr::write to atomically write a valid screen value, so the
        // memory never contains an invalid intermediate state.  This is
        // important for fields like `tabs` (Option<Rc<…>>) and will be
        // essential when `images` is migrated from tailq_head to Vec.
        std::ptr::write(
            s,
            screen {
                grid: grid_create(sx, sy, hlimit),
                saved_grid: None,

                title: CString::default(),
                titles: Vec::new(),
                path: None,

                cx: 0,
                cy: 0,

                cstyle: screen_cursor_style::SCREEN_CURSOR_DEFAULT,
                default_cstyle: screen_cursor_style::SCREEN_CURSOR_DEFAULT,
                ccolour: -1,
                default_ccolour: -1,

                rupper: 0,
                rlower: 0,

                mode: mode_flag::MODE_CURSOR,
                default_mode: mode_flag::empty(),

                saved_cx: 0,
                saved_cy: 0,
                saved_cell: zeroed(),
                saved_flags: 0,

                tabs: None,
                sel: None,

                #[cfg(feature = "sixel")]
                images: Vec::new(),

                write_list: None,
                hyperlinks: None,
            },
        );

        screen_reinit(s);
    }
}

/// Reinitialise screen.
pub unsafe fn screen_reinit(s: *mut screen) {
    unsafe {
        (*s).cx = 0;
        (*s).cy = 0;

        (*s).rupper = 0;
        (*s).rlower = screen_size_y(s) - 1;

        (*s).mode =
            mode_flag::MODE_CURSOR | mode_flag::MODE_WRAP | ((*s).mode & mode_flag::MODE_CRLF);

        if options_get_number_(GLOBAL_OPTIONS, "extended-keys") == 2 {
            (*s).mode = ((*s).mode & !EXTENDED_KEY_MODES) | mode_flag::MODE_KEYS_EXTENDED;
        }

        if (*s).saved_grid.is_some() {
            screen_alternate_off(s, null_mut(), 0);
        }
        (*s).saved_cx = u32::MAX;
        (*s).saved_cy = u32::MAX;

        screen_reset_tabs(s);

        grid_clear_lines(&raw mut *(*s).grid, (*s).grid.hsize, (*s).grid.sy, 8);

        screen_clear_selection(s);
        screen_free_titles(s);

        #[cfg(feature = "sixel")]
        crate::image_::image_free_all(s);

        screen_reset_hyperlinks(s);
    }
}

/// Reset hyperlinks of a screen.
pub unsafe fn screen_reset_hyperlinks(s: *mut screen) {
    unsafe {
        if let Some(hl) = (*s).hyperlinks {
            hyperlinks_reset(hl);
        } else {
            (*s).hyperlinks = Some(hyperlinks_init());
        }
    }
}

/// Destroy a screen.
pub unsafe fn screen_free(s: *mut screen) {
    unsafe {
        (*s).sel = None;
        (*s).tabs = None;
        // path and title: CString/Option<CString> drop automatically

        if (*s).write_list.is_some() {
            screen_write_free_list(s);
        }

        (*s).saved_grid = None;
        // grid: Box<grid> drops automatically when screen is freed

        if let Some(hl) = (*s).hyperlinks {
            hyperlinks_free(hl);
        }
        screen_free_titles(s);

        #[cfg(feature = "sixel")]
        crate::image_::image_free_all(s);
    }
}

/// Reset tabs to default, eight spaces apart.
pub unsafe fn screen_reset_tabs(s: *mut screen) {
    unsafe {
        (*s).tabs = Some(Rc::new(RefCell::new(BitStr::new(screen_size_x(s)))));

        let mut i = 8;
        while i < screen_size_x(s) {
            (*s).tabs.as_ref().unwrap().borrow_mut().bit_set(i);
            i += 8;
        }
    }
}

/// Set screen cursor style and mode.
pub unsafe fn screen_set_cursor_style(
    style: u32,
    cstyle: *mut screen_cursor_style,
    mode: *mut mode_flag,
) {
    unsafe {
        match style {
            0 => *cstyle = screen_cursor_style::SCREEN_CURSOR_DEFAULT,
            1 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BLOCK;
                *mode |= mode_flag::MODE_CURSOR_BLINKING;
            }
            2 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BLOCK;
                *mode &= !mode_flag::MODE_CURSOR_BLINKING;
            }
            3 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_UNDERLINE;
                *mode |= mode_flag::MODE_CURSOR_BLINKING;
            }
            4 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_UNDERLINE;
                *mode &= !mode_flag::MODE_CURSOR_BLINKING;
            }
            5 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BAR;
                *mode |= mode_flag::MODE_CURSOR_BLINKING;
            }
            6 => {
                *cstyle = screen_cursor_style::SCREEN_CURSOR_BAR;
                *mode &= !mode_flag::MODE_CURSOR_BLINKING;
            }
            _ => (),
        }
    }
}

/// Set screen cursor colour.
pub unsafe fn screen_set_cursor_colour(s: *mut screen, colour: c_int) {
    unsafe {
        (*s).ccolour = colour;
    }
}

/// Set screen title.
pub unsafe fn screen_set_title(s: *mut screen, title: *const u8) -> c_int {
    unsafe {
        if !utf8_isvalid(title) {
            return 0;
        }
        (*s).title = CStr::from_ptr(title.cast()).to_owned();
        1
    }
}

/// Set screen path.
pub unsafe fn screen_set_path(s: *mut screen, path: *const u8) {
    unsafe {
        let vis = utf8_stravis_(
            path,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        (*s).path = Some(CString::new(vis).unwrap_or_default());
    }
}

/// Push the current title onto the stack.
pub unsafe fn screen_push_title(s: *mut screen) {
    unsafe {
        // Push to front (index 0 = top of stack)
        (*s).titles.insert(0, (*s).title.clone());
    }
}

/// Pop a title from the stack and set it as the screen title. If the stack is empty, do nothing.
pub unsafe fn screen_pop_title(s: *mut screen) {
    unsafe {
        if !(*s).titles.is_empty() {
            let text = (*s).titles.remove(0);
            screen_set_title(s, text.as_ptr() as *const u8);
        }
    }
}

/// Resize screen with options.
pub unsafe fn screen_resize_cursor(
    s: *mut screen,
    sx: u32,
    sy: u32,
    mut reflow: i32,
    eat_empty: i32,
    cursor: i32,
) {
    let __func__ = "screen_resize_cursor";
    unsafe {
        let mut cx = (*s).cx;
        let mut cy = (*(*s).grid).hsize + (*s).cy;

        let had_write_list = (*s).write_list.is_some();
        if had_write_list {
            screen_write_free_list(s);
        }

        log_debug!(
            "{}: new size {}{}, now {}x{} (cursor {},{} = {},{})",
            __func__,
            sx,
            sy,
            screen_size_x(s),
            screen_size_y(s),
            (*s).cx,
            (*s).cy,
            cx,
            cy,
        );

        let sx = if sx < 1 { 1 } else { sx };
        let sy = if sy < 1 { 1 } else { sy };

        if sx != screen_size_x(s) {
            (*(*s).grid).sx = sx;
            screen_reset_tabs(s);
        } else {
            reflow = 0;
        }

        if sy != screen_size_y(s) {
            screen_resize_y(s, sy, eat_empty, &mut cy);
        }

        #[cfg(feature = "sixel")]
        crate::image_::image_free_all(s);

        if reflow != 0 {
            screen_reflow(s, sx, &mut cx, &mut cy, cursor);
        }

        if cy >= (*(*s).grid).hsize {
            (*s).cx = cx;
            (*s).cy = cy - (*(*s).grid).hsize;
        } else {
            (*s).cx = 0;
            (*s).cy = 0;
        }

        log_debug!(
            "{}: cursor finished at {},{} = {},{}",
            __func__,
            (*s).cx,
            (*s).cy,
            cx,
            cy,
        );

        if had_write_list {
            screen_write_make_list(s);
        }
    }
}

/// Resize screen.
pub unsafe fn screen_resize(s: *mut screen, sx: u32, sy: u32, reflow: i32) {
    unsafe {
        screen_resize_cursor(s, sx, sy, reflow, 1, 1);
    }
}

/// Resize screen vertically.
unsafe fn screen_resize_y(s: *mut screen, sy: u32, eat_empty: i32, cy: *mut u32) {
    unsafe {
        let gd = &raw mut *(*s).grid;

        if sy == 0 {
            fatalx("zero size");
        }
        let oldy = screen_size_y(s);

        // When resizing:
        //
        // If the height is decreasing, delete lines from the bottom until
        // hitting the cursor, then push lines from the top into the history.
        //
        // When increasing, pull as many lines as possible from scrolled
        // history (not explicitly cleared from view) to the top, then fill the
        // remaining with blanks at the bottom.

        // Size decreasing
        if sy < oldy {
            let mut needed = oldy - sy;

            // Delete as many lines as possible from the bottom
            if eat_empty != 0 {
                let mut available = oldy - 1 - (*s).cy;
                if available > 0 {
                    if available > needed {
                        available = needed;
                    }
                    grid_view_delete_lines(gd, oldy - available, available, 8);
                }
                needed -= available;
            }

            // Now just increase the history size, if possible, to take
            // over the lines which are left. If history is off, delete
            // lines from the top.
            let mut available = (*s).cy;
            if (*gd).flags & GRID_HISTORY != 0 {
                (*gd).hscrolled += needed;
                (*gd).hsize += needed;
            } else if needed > 0 && available > 0 {
                if available > needed {
                    available = needed;
                }
                grid_view_delete_lines(gd, 0, available, 8);
                *cy -= available;
            }
        }

        // Resize line array
        grid_adjust_lines(gd, (*gd).hsize + sy);

        // Size increasing
        if sy > oldy {
            let mut needed = sy - oldy;

            // Try to pull as much as possible out of scrolled history, if
            // it is enabled.
            let mut available = (*gd).hscrolled;
            if (*gd).flags & GRID_HISTORY != 0 && available > 0 {
                if available > needed {
                    available = needed;
                }
                (*gd).hscrolled -= available;
                (*gd).hsize -= available;
            } else {
                available = 0;
            }
            needed -= available;

            // Then fill the rest in with blanks
            for i in ((*gd).hsize + sy - needed)..((*gd).hsize + sy) {
                grid_empty_line(gd, i, 8);
            }
        }

        // Set the new size, and reset the scroll region
        (*gd).sy = sy;
        (*s).rupper = 0;
        (*s).rlower = screen_size_y(s) - 1;
    }
}

/// Set selection.
pub unsafe fn screen_set_selection(
    s: *mut screen,
    sx: u32,
    sy: u32,
    ex: u32,
    ey: u32,
    rectangle: u32,
    modekeys: modekey,
    gc: *mut grid_cell,
) {
    unsafe {
        let sel = (*s).sel.get_or_insert_with(|| Box::new(zeroed()));

        sel.cell = *gc;
        sel.hidden = 0;
        sel.rectangle = rectangle as i32;
        sel.modekeys = modekeys;

        sel.sx = sx;
        sel.sy = sy;
        sel.ex = ex;
        sel.ey = ey;
    }
}

/// Clear selection.
pub unsafe fn screen_clear_selection(s: *mut screen) {
    unsafe {
        (*s).sel = None;
    }
}

/// Hide selection.
pub unsafe fn screen_hide_selection(s: *mut screen) {
    unsafe {
        if let Some(sel) = (*s).sel.as_mut() {
            sel.hidden = 1;
        }
    }
}

/// Check if cell in selection.
pub unsafe fn screen_check_selection(s: *mut screen, px: u32, py: u32) -> c_int {
    unsafe {
        let sel = match (*s).sel.as_ref() {
            Some(sel) if sel.hidden == 0 => sel,
            _ => return 0,
        };
        let xx: u32;

        if sel.rectangle != 0 {
            match sel.sy.cmp(&sel.ey) {
                cmp::Ordering::Less => {
                    // start line < end line -- downward selection.
                    if py < sel.sy || py > sel.ey {
                        return 0;
                    }
                }
                cmp::Ordering::Greater => {
                    // start line > end line -- upward selection.
                    if py > sel.sy || py < sel.ey {
                        return 0;
                    }
                }
                cmp::Ordering::Equal => {
                    // starting line == ending line.
                    if py != sel.sy {
                        return 0;
                    }
                }
            }

            // Need to include the selection start row, but not the cursor
            // row, which means the selection changes depending on which
            // one is on the left.
            if sel.ex < sel.sx {
                // Cursor (ex) is on the left.
                if px < sel.ex {
                    return 0;
                }

                if px > sel.sx {
                    return 0;
                }
            } else {
                // Selection start (sx) is on the left.
                if px < sel.sx {
                    return 0;
                }

                if px > sel.ex {
                    return 0;
                }
            }
        } else {

            // Like emacs, keep the top-left-most character, and drop the
            // bottom-right-most, regardless of copy direction.
            match sel.sy.cmp(&(sel.ey)) {
                cmp::Ordering::Less => {
                    // starting line < ending line -- downward selection.
                    if py < sel.sy || py > sel.ey {
                        return 0;
                    }

                    if py == sel.sy && px < sel.sx {
                        return 0;
                    }

                    if sel.modekeys == modekey::MODEKEY_EMACS {
                        xx = if sel.ex == 0 { 0 } else { sel.ex - 1 };
                    } else {
                        xx = sel.ex;
                    }
                    if py == sel.ey && px > xx {
                        return 0;
                    }
                }
                cmp::Ordering::Greater => {
                    // starting line > ending line -- upward selection.
                    if py > sel.sy || py < sel.ey {
                        return 0;
                    }

                    if py == sel.ey && px < sel.ex {
                        return 0;
                    }

                    if sel.modekeys == modekey::MODEKEY_EMACS {
                        xx = sel.sx - 1;
                    } else {
                        xx = sel.sx;
                    }
                    if py == sel.sy && (sel.sx == 0 || px > xx) {
                        return 0;
                    }
                }
                cmp::Ordering::Equal => {
                    // starting line == ending line.
                    if py != sel.sy {
                        return 0;
                    }

                    if sel.ex < sel.sx {
                        // cursor (ex) is on the left
                        if sel.modekeys == modekey::MODEKEY_EMACS {
                            xx = sel.sx - 1;
                        } else {
                            xx = sel.sx;
                        }
                        if px > xx || px < sel.ex {
                            return 0;
                        }
                    } else {
                        // selection start (sx) is on the left
                        if sel.modekeys == modekey::MODEKEY_EMACS {
                            xx = if sel.ex == 0 { 0 } else { sel.ex - 1 };
                        } else {
                            xx = sel.ex;
                        }
                        if px < sel.sx || px > xx {
                            return 0;
                        }
                    }
                }
            }
        }

        1
    }
}

/// Get selected grid cell.
pub unsafe fn screen_select_cell(s: *mut screen, dst: *mut grid_cell, src: *const grid_cell) {
    unsafe {
        let sel = match (*s).sel.as_ref() {
            Some(sel) if sel.hidden == 0 => sel,
            _ => return,
        };

        *dst = sel.cell;

        utf8_copy(&mut (*dst).data, &(*src).data);
        (*dst).attr &= !grid_attr::GRID_ATTR_CHARSET;
        (*dst).attr |= (*src).attr & grid_attr::GRID_ATTR_CHARSET;
        (*dst).flags = (*src).flags;
    }
}

/// Reflow wrapped lines.
unsafe fn screen_reflow(s: *mut screen, new_x: u32, cx: *mut u32, cy: *mut u32, cursor: i32) {
    unsafe {
        let mut wx: u32 = 0;
        let mut wy: u32 = 0;

        if cursor != 0 {
            grid_wrap_position(&raw mut *(*s).grid, *cx, *cy, &mut wx, &mut wy);
            log_debug!(
                "{}: cursor {},{} is {},{}",
                "screen_reflow",
                *cx,
                *cy,
                wx,
                wy,
            );
        }

        grid_reflow(&raw mut *(*s).grid, new_x);

        if cursor != 0 {
            grid_unwrap_position(&raw mut *(*s).grid, cx, cy, wx, wy);
            log_debug!("{}: new cursor is {},{}", "screen_reflow", *cx, *cy);
        } else {
            *cx = 0;
            *cy = (*s).grid.hsize;
        }
    }
}

/// Enter alternative screen mode. A copy of the visible screen is saved and the
/// history is not updated.
pub unsafe fn screen_alternate_on(s: *mut screen, gc: *mut grid_cell, cursor: i32) {
    unsafe {
        if (*s).saved_grid.is_some() {
            return;
        }
        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        (*s).saved_grid = Some(grid_create(sx, sy, 0));
        let sg: *mut grid = &raw mut **(*s).saved_grid.as_mut().unwrap();
        grid_duplicate_lines(sg, 0, &raw mut *(*s).grid, screen_hsize(s), sy);
        if cursor != 0 {
            (*s).saved_cx = (*s).cx;
            (*s).saved_cy = (*s).cy;
        }
        memcpy__(&raw mut (*s).saved_cell, gc);

        grid_view_clear(&raw mut *(*s).grid, 0, 0, sx, sy, 8);

        (*s).saved_flags = (*s).grid.flags;
        (*s).grid.flags &= !GRID_HISTORY;
    }
}

/// Exit alternate screen mode and restore the copied grid.
pub unsafe fn screen_alternate_off(s: *mut screen, gc: *mut grid_cell, cursor: i32) {
    unsafe {
        let sx = screen_size_x(s);
        let sy = screen_size_y(s);

        // If the current size is different, temporarily resize to the old size
        // before copying back.
        if let Some(ref sg) = (*s).saved_grid {
            screen_resize(s, sg.sx, sg.sy, 0);
        }

        // Restore the cursor position and cell. This happens even if not
        // currently in the alternate screen.
        if cursor != 0 && (*s).saved_cx != u32::MAX && (*s).saved_cy != u32::MAX {
            (*s).cx = (*s).saved_cx;
            (*s).cy = (*s).saved_cy;
            if !gc.is_null() {
                memcpy__(gc, &raw const (*s).saved_cell);
            }
        }

        // If not in the alternate screen, do nothing more.
        if (*s).saved_grid.is_none() {
            if (*s).cx > screen_size_x(s) - 1 {
                (*s).cx = screen_size_x(s) - 1;
            }
            if (*s).cy > screen_size_y(s) - 1 {
                (*s).cy = screen_size_y(s) - 1;
            }
            return;
        }

        // Restore the saved grid.
        let sg = (*s).saved_grid.as_ref().unwrap();
        let sg_sy = sg.sy;
        let sg_ptr: *mut grid = &**sg as *const grid as *mut grid;
        grid_duplicate_lines(
            &raw mut *(*s).grid,
            screen_hsize(s),
            sg_ptr,
            0,
            sg_sy,
        );

        // Turn history back on (so resize can use it) and then resize back to
        // the current size.
        if (*s).saved_flags & GRID_HISTORY != 0 {
            (*s).grid.flags |= GRID_HISTORY;
        }
        screen_resize(s, sx, sy, 1);

        drop((*s).saved_grid.take());

        if (*s).cx > screen_size_x(s) - 1 {
            (*s).cx = screen_size_x(s) - 1;
        }
        if (*s).cy > screen_size_y(s) - 1 {
            (*s).cy = screen_size_y(s) - 1;
        }
    }
}

/// Get mode as a string.
pub unsafe fn screen_mode_to_string(mode: mode_flag) -> *const u8 {
    const TMP_LEN: usize = 1024;
    static mut TMP: [MaybeUninit<u8>; 1024] = [MaybeUninit::uninit(); 1024];

    unsafe {
        if mode == mode_flag::empty() {
            return c!("NONE");
        }
        if mode.is_all() {
            return c!("ALL");
        }

        *TMP[0].as_mut_ptr().cast() = 0i8;

        if mode.intersects(mode_flag::MODE_CURSOR) {
            strlcat(addr_of_mut!(TMP).cast(), c!("CURSOR,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_INSERT) {
            strlcat(addr_of_mut!(TMP).cast(), c!("INSERT,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KCURSOR) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KCURSOR,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KKEYPAD) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KKEYPAD,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_WRAP) {
            strlcat(addr_of_mut!(TMP).cast(), c!("WRAP,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_STANDARD) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_STANDARD,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_BUTTON) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_BUTTON,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_CURSOR_BLINKING) {
            strlcat(addr_of_mut!(TMP).cast(), c!("CURSOR_BLINKING,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_CURSOR_VERY_VISIBLE) {
            strlcat(
                addr_of_mut!(TMP).cast(),
                c!("CURSOR_VERY_VISIBLE,"),
                TMP_LEN,
            );
        }
        if mode.intersects(mode_flag::MODE_MOUSE_UTF8) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_UTF8,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_SGR) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_SGR,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_BRACKETPASTE) {
            strlcat(addr_of_mut!(TMP).cast(), c!("BRACKETPASTE,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_FOCUSON) {
            strlcat(addr_of_mut!(TMP).cast(), c!("FOCUSON,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_MOUSE_ALL) {
            strlcat(addr_of_mut!(TMP).cast(), c!("MOUSE_ALL,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_ORIGIN) {
            strlcat(addr_of_mut!(TMP).cast(), c!("ORIGIN,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_CRLF) {
            strlcat(addr_of_mut!(TMP).cast(), c!("CRLF,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KEYS_EXTENDED) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KEYS_EXTENDED,"), TMP_LEN);
        }
        if mode.intersects(mode_flag::MODE_KEYS_EXTENDED_2) {
            strlcat(addr_of_mut!(TMP).cast(), c!("KEYS_EXTENDED_2,"), TMP_LEN);
        }

        let len = strlen(addr_of!(TMP).cast());
        if len > 0 {
            *TMP[len - 1].as_mut_ptr().cast() = 0i8;
        }
        &raw mut TMP as *mut u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::zeroed;

    /// Initialize global options (required for screen_reinit which reads
    /// the "extended-keys" option).
    unsafe fn init_globals() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            use crate::tmux::{GLOBAL_OPTIONS, GLOBAL_S_OPTIONS, GLOBAL_W_OPTIONS};
            use crate::options_::*;
            GLOBAL_OPTIONS = options_create(null_mut());
            GLOBAL_S_OPTIONS = options_create(null_mut());
            GLOBAL_W_OPTIONS = options_create(null_mut());
            for oe in &OPTIONS_TABLE {
                if oe.scope & OPTIONS_TABLE_SERVER != 0 {
                    options_default(GLOBAL_OPTIONS, oe);
                }
                if oe.scope & OPTIONS_TABLE_SESSION != 0 {
                    options_default(GLOBAL_S_OPTIONS, oe);
                }
                if oe.scope & OPTIONS_TABLE_WINDOW != 0 {
                    options_default(GLOBAL_W_OPTIONS, oe);
                }
            }
        });
    }

    /// Helper: create and initialize a screen for testing.
    unsafe fn make_screen(sx: u32, sy: u32) -> *mut screen {
        unsafe {
            init_globals();
            let s = Box::into_raw(Box::new(screen_placeholder()));
            screen_init(s, sx, sy, 0);
            s
        }
    }

    /// Helper: destroy a test screen and free its Box.
    unsafe fn destroy_screen(s: *mut screen) {
        unsafe {
            screen_free(s);
            drop(Box::from_raw(s));
        }
    }

    // ---------------------------------------------------------------
    // Lifecycle
    // ---------------------------------------------------------------

    #[test]
    fn screen_init_sets_dimensions() {
        unsafe {
            let s = make_screen(80, 24);
            assert_eq!(screen_size_x(s), 80);
            assert_eq!(screen_size_y(s), 24);
            assert_eq!((*s).cx, 0);
            assert_eq!((*s).cy, 0);
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_init_sets_scroll_region() {
        unsafe {
            let s = make_screen(80, 24);
            assert_eq!((*s).rupper, 0);
            assert_eq!((*s).rlower, 23);
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_init_sets_cursor_mode() {
        unsafe {
            let s = make_screen(80, 24);
            assert!((*s).mode.contains(mode_flag::MODE_CURSOR));
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_init_creates_grid() {
        unsafe {
            let s = make_screen(40, 10);
            assert_eq!((*s).grid.sx, 40);
            assert_eq!((*s).grid.sy, 10);
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_free_does_not_crash() {
        unsafe {
            let s = make_screen(80, 24);
            destroy_screen(s);
            // If we reach here, free did not crash.
        }
    }

    // ---------------------------------------------------------------
    // Tab stops
    // ---------------------------------------------------------------

    #[test]
    fn screen_reset_tabs_sets_every_eighth() {
        unsafe {
            let s = make_screen(80, 24);
            let tabs = (*s).tabs.as_ref().unwrap().borrow();
            // Tab stops at 8, 16, 24, ..., 72. Not at 0.
            assert!(!tabs.bit_test(0));
            assert!(tabs.bit_test(8));
            assert!(tabs.bit_test(16));
            assert!(tabs.bit_test(24));
            assert!(tabs.bit_test(72));
            assert!(!tabs.bit_test(79));
            drop(tabs);
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_reset_tabs_small_screen() {
        unsafe {
            let s = make_screen(5, 5);
            // Screen too narrow for any tab stop.
            let tabs = (*s).tabs.as_ref().unwrap().borrow();
            assert!(!tabs.bit_test(0));
            // No tab stops possible in a 5-column screen.
            drop(tabs);
            destroy_screen(s);
        }
    }

    // ---------------------------------------------------------------
    // Title management
    // ---------------------------------------------------------------

    #[test]
    fn screen_set_title_stores_title() {
        unsafe {
            let s = make_screen(80, 24);
            screen_set_title(s, c"hello".as_ptr().cast());
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "hello");
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_set_title_replaces_previous() {
        unsafe {
            let s = make_screen(80, 24);
            screen_set_title(s, c"first".as_ptr().cast());
            screen_set_title(s, c"second".as_ptr().cast());
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "second");
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_push_pop_title_stack() {
        unsafe {
            let s = make_screen(80, 24);

            screen_set_title(s, c"base".as_ptr().cast());
            screen_push_title(s);
            screen_set_title(s, c"overlay".as_ptr().cast());

            // Current title is "overlay".
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "overlay");

            // Pop restores "base".
            screen_pop_title(s);
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "base");

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_pop_title_on_empty_stack_does_nothing() {
        unsafe {
            let s = make_screen(80, 24);
            screen_set_title(s, c"keep".as_ptr().cast());
            screen_pop_title(s); // No stack — should be no-op.
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "keep");
            destroy_screen(s);
        }
    }

    #[test]
    fn screen_push_multiple_pop_in_order() {
        unsafe {
            let s = make_screen(80, 24);

            screen_set_title(s, c"A".as_ptr().cast());
            screen_push_title(s);
            screen_set_title(s, c"B".as_ptr().cast());
            screen_push_title(s);
            screen_set_title(s, c"C".as_ptr().cast());

            // Pop C → B
            screen_pop_title(s);
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "B");

            // Pop B → A
            screen_pop_title(s);
            let title = &(*s).title;
            assert_eq!(title.to_str().unwrap(), "A");

            destroy_screen(s);
        }
    }

    // ---------------------------------------------------------------
    // Cursor style
    // ---------------------------------------------------------------

    #[test]
    fn screen_set_cursor_style_mappings() {
        unsafe {
            let mut style = screen_cursor_style::SCREEN_CURSOR_DEFAULT;
            let mut mode = mode_flag::empty();

            // Style 0 → DEFAULT
            screen_set_cursor_style(0, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_DEFAULT);

            // Style 1 → BLOCK + BLINKING
            screen_set_cursor_style(1, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_BLOCK);
            assert!(mode.contains(mode_flag::MODE_CURSOR_BLINKING));

            // Style 2 → BLOCK, no blinking
            screen_set_cursor_style(2, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_BLOCK);
            assert!(!mode.contains(mode_flag::MODE_CURSOR_BLINKING));

            // Style 3 → UNDERLINE + BLINKING
            screen_set_cursor_style(3, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_UNDERLINE);
            assert!(mode.contains(mode_flag::MODE_CURSOR_BLINKING));

            // Style 4 → UNDERLINE, no blinking
            screen_set_cursor_style(4, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_UNDERLINE);
            assert!(!mode.contains(mode_flag::MODE_CURSOR_BLINKING));

            // Style 5 → BAR + BLINKING
            screen_set_cursor_style(5, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_BAR);
            assert!(mode.contains(mode_flag::MODE_CURSOR_BLINKING));

            // Style 6 → BAR, no blinking
            screen_set_cursor_style(6, &mut style, &mut mode);
            assert_eq!(style, screen_cursor_style::SCREEN_CURSOR_BAR);
            assert!(!mode.contains(mode_flag::MODE_CURSOR_BLINKING));

            // Style 99 → no change (unknown)
            let prev_style = style;
            screen_set_cursor_style(99, &mut style, &mut mode);
            assert_eq!(style, prev_style);
        }
    }

    // ---------------------------------------------------------------
    // Selection
    // ---------------------------------------------------------------

    #[test]
    fn screen_set_and_check_selection_rectangle() {
        unsafe {
            let s = make_screen(80, 24);
            let mut gc: grid_cell = zeroed();

            // Set a rectangular selection from (2,1) to (5,3).
            screen_set_selection(s, 2, 1, 5, 3, 1, modekey::MODEKEY_EMACS, &mut gc);

            // Inside selection.
            assert_ne!(screen_check_selection(s, 3, 2), 0);
            // On boundary.
            assert_ne!(screen_check_selection(s, 2, 1), 0);
            assert_ne!(screen_check_selection(s, 5, 3), 0);
            // Outside — wrong column.
            assert_eq!(screen_check_selection(s, 1, 2), 0);
            assert_eq!(screen_check_selection(s, 6, 2), 0);
            // Outside — wrong row.
            assert_eq!(screen_check_selection(s, 3, 0), 0);
            assert_eq!(screen_check_selection(s, 3, 4), 0);

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_clear_selection_removes_it() {
        unsafe {
            let s = make_screen(80, 24);
            let mut gc: grid_cell = zeroed();

            screen_set_selection(s, 0, 0, 10, 10, 1, modekey::MODEKEY_EMACS, &mut gc);
            assert_ne!(screen_check_selection(s, 5, 5), 0);

            screen_clear_selection(s);
            assert_eq!(screen_check_selection(s, 5, 5), 0);

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_hide_selection_hides_it() {
        unsafe {
            let s = make_screen(80, 24);
            let mut gc: grid_cell = zeroed();

            screen_set_selection(s, 0, 0, 10, 10, 1, modekey::MODEKEY_EMACS, &mut gc);
            assert_ne!(screen_check_selection(s, 5, 5), 0);

            screen_hide_selection(s);
            assert_eq!(screen_check_selection(s, 5, 5), 0);

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_check_selection_linear_downward() {
        unsafe {
            let s = make_screen(80, 24);
            let mut gc: grid_cell = zeroed();

            // Linear (non-rectangle) selection from (5,2) to (10,4) in vi mode.
            screen_set_selection(s, 5, 2, 10, 4, 0, modekey::MODEKEY_VI, &mut gc);

            // Start line — before sx.
            assert_eq!(screen_check_selection(s, 4, 2), 0);
            // Start line — at sx.
            assert_ne!(screen_check_selection(s, 5, 2), 0);
            // Middle line — any column should be in selection.
            assert_ne!(screen_check_selection(s, 0, 3), 0);
            assert_ne!(screen_check_selection(s, 79, 3), 0);
            // End line — at ex.
            assert_ne!(screen_check_selection(s, 10, 4), 0);
            // End line — past ex.
            assert_eq!(screen_check_selection(s, 11, 4), 0);
            // Outside rows.
            assert_eq!(screen_check_selection(s, 5, 1), 0);
            assert_eq!(screen_check_selection(s, 5, 5), 0);

            destroy_screen(s);
        }
    }

    // ---------------------------------------------------------------
    // Resize
    // ---------------------------------------------------------------

    #[test]
    fn screen_resize_changes_dimensions() {
        unsafe {
            let s = make_screen(80, 24);

            screen_resize(s, 40, 12, 0);
            assert_eq!(screen_size_x(s), 40);
            assert_eq!(screen_size_y(s), 12);

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_resize_updates_scroll_region() {
        unsafe {
            let s = make_screen(80, 24);

            screen_resize(s, 80, 10, 0);
            assert_eq!((*s).rupper, 0);
            assert_eq!((*s).rlower, 9);

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_resize_grow_then_shrink() {
        unsafe {
            let s = make_screen(80, 24);

            // Grow.
            screen_resize(s, 120, 40, 0);
            assert_eq!(screen_size_x(s), 120);
            assert_eq!(screen_size_y(s), 40);

            // Shrink back.
            screen_resize(s, 80, 24, 0);
            assert_eq!(screen_size_x(s), 80);
            assert_eq!(screen_size_y(s), 24);

            destroy_screen(s);
        }
    }

    #[test]
    fn screen_resize_minimum_1x1() {
        unsafe {
            let s = make_screen(80, 24);

            screen_resize(s, 0, 0, 0);
            // Should clamp to 1x1.
            assert_eq!(screen_size_x(s), 1);
            assert_eq!(screen_size_y(s), 1);

            destroy_screen(s);
        }
    }

    // ---------------------------------------------------------------
    // Reinit
    // ---------------------------------------------------------------

    #[test]
    fn screen_reinit_resets_cursor() {
        unsafe {
            let s = make_screen(80, 24);
            (*s).cx = 40;
            (*s).cy = 12;

            screen_reinit(s);
            assert_eq!((*s).cx, 0);
            assert_eq!((*s).cy, 0);

            destroy_screen(s);
        }
    }

    // ---------------------------------------------------------------
    // Placeholder
    // ---------------------------------------------------------------

    #[test]
    fn screen_placeholder_has_safe_defaults() {
        let s = screen_placeholder();
        assert!(s.title.is_empty());
        assert!(s.path.is_none());
        assert_eq!(s.grid.sx, 0); // placeholder grid is empty
        assert!(s.saved_grid.is_none());
        assert!(s.sel.is_none());
        assert!(s.tabs.is_none());
        assert_eq!(s.cx, 0);
        assert_eq!(s.cy, 0);
    }
}
