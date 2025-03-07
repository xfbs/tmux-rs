use compat_rs::{
    queue::{tailq_first, tailq_foreach, tailq_insert_tail, tailq_last, tailq_next},
    strlcat,
};
use libc::{isdigit, sscanf};

use crate::*;

unsafe extern "C" {
    // pub fn layout_dump(_: *mut layout_cell) -> *mut c_char;
    // pub fn layout_parse(_: *mut window, _: *const c_char, _: *mut *mut c_char) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_find_bottomright(mut lc: *mut layout_cell) -> *mut layout_cell {
    unsafe {
        if ((*lc).type_ == layout_type::LAYOUT_WINDOWPANE) {
            return lc;
        }
        lc = tailq_last(&raw mut (*lc).cells);
        layout_find_bottomright(lc)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_checksum(mut layout: *const c_char) -> u16 {
    unsafe {
        let mut csum = 0u16;
        while *layout != b'\0' as _ {
            csum = (csum >> 1) + ((csum & 1) << 15);
            csum += *layout as u16;
            layout = layout.add(1);
        }
        csum
    }
}

/// Dump layout as a string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_dump(root: *mut layout_cell) -> *mut c_char {
    unsafe {
        let mut layout: MaybeUninit<[c_char; 8192]> = MaybeUninit::<[c_char; 8192]>::uninit();
        let mut layout = layout.as_mut_ptr() as *mut i8;

        let mut out: *mut c_char = null_mut();

        *layout = b'\0' as _;
        if (layout_append(root, layout, 8192) != 0) {
            return null_mut();
        }

        xasprintf(
            &raw mut out,
            c"%04hx,%s".as_ptr(),
            layout_checksum(layout) as u32,
            layout,
        );
        out
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_append(lc: *mut layout_cell, buf: *mut c_char, len: usize) -> i32 {
    unsafe {
        let sizeof_tmp = 64;
        let mut tmp = MaybeUninit::<[c_char; 64]>::uninit();
        let tmp = tmp.as_mut_ptr() as *mut i8;
        // struct layout_cell *lcchild;
        // char tmp[64];
        // size_t tmplen;

        let mut brackets = c"][".as_ptr();

        if (len == 0) {
            return -1;
        }

        let tmplen = if (!(*lc).wp.is_null()) {
            xsnprintf(
                tmp,
                sizeof_tmp,
                c"%ux%u,%u,%u,%u".as_ptr(),
                (*lc).sx,
                (*lc).sy,
                (*lc).xoff,
                (*lc).yoff,
                (*(*lc).wp).id,
            )
        } else {
            xsnprintf(
                tmp,
                sizeof_tmp,
                c"%ux%u,%u,%u".as_ptr(),
                (*lc).sx,
                (*lc).sy,
                (*lc).xoff,
                (*lc).yoff,
            )
        };

        if (tmplen > sizeof_tmp as i32 - 1) {
            return -1;
        }
        if (strlcat(buf, tmp, len) >= len) {
            return -1;
        }

        if ((*lc).type_) == layout_type::LAYOUT_LEFTRIGHT {
            brackets = c"}{".as_ptr();
        }

        match ((*lc).type_) {
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                if (strlcat(buf, brackets.add(1), len) >= len) {
                    return -1;
                }
                if tailq_foreach(&raw mut (*lc).cells, |lcchild| {
                    if (layout_append(lcchild, buf, len) != 0) {
                        return ControlFlow::<(), ()>::Break(());
                    }
                    if (strlcat(buf, c",".as_ptr(), len) >= len) {
                        return ControlFlow::<(), ()>::Break(());
                    }
                    ControlFlow::<(), ()>::Continue(())
                })
                .is_break()
                {
                    return -1;
                }
                *buf.add(strlen(buf) - 1) = *brackets;
            }
            layout_type::LAYOUT_WINDOWPANE => (),
        }
    }
    0
}

/// Check layout sizes fit.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_check(lc: *mut layout_cell) -> i32 {
    unsafe {
        let mut n = 0u32;

        match ((*lc).type_) {
            layout_type::LAYOUT_WINDOWPANE => (),
            layout_type::LAYOUT_LEFTRIGHT => {
                if tailq_foreach(&raw mut (*lc).cells, |lcchild| {
                    if ((*lcchild).sy != (*lc).sy) {
                        return ControlFlow::Break(());
                    }
                    if (layout_check(lcchild) == 0) {
                        return ControlFlow::Break(());
                    }
                    n += (*lcchild).sx + 1;
                    ControlFlow::Continue(())
                })
                .is_break()
                {
                    return 0;
                }
                if (n - 1 != (*lc).sx) {
                    return 0;
                }
            }
            layout_type::LAYOUT_TOPBOTTOM => {
                if tailq_foreach(&raw mut (*lc).cells, |lcchild| {
                    if ((*lcchild).sx != (*lc).sx) {
                        return ControlFlow::Break(());
                    }
                    if (layout_check(lcchild) == 0) {
                        return ControlFlow::Break(());
                    }
                    n += (*lcchild).sy + 1;
                    ControlFlow::Break(())
                })
                .is_break()
                {
                    return 0;
                }
                if (n - 1 != (*lc).sy) {
                    return 0;
                }
            }
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn layout_parse(w: *mut window, mut layout: *const c_char, cause: *mut *mut c_char) -> i32 {
    let __func__ = c"layout_parse".as_ptr();
    unsafe {
        let mut lc: *mut layout_cell = null_mut();
        // struct layout_cell *lc, *lcchild;
        // struct window_pane *wp;
        // u_int npanes, ncells, sx = 0, sy = 0;
        // u_short csum;
        let mut csum: u16 = 0;

        'fail: {
            /* Check validity. */
            if (sscanf(layout, c"%hx,".as_ptr(), &raw mut csum) != 1) {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                return -1;
            }
            layout = layout.add(5);
            if (csum != layout_checksum(layout)) {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                return -1;
            }

            /* Build the layout. */
            lc = layout_construct(null_mut(), &raw mut layout);
            if (lc.is_null()) {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                return -1;
            }
            if (*layout != b'\0' as _) {
                *cause = xstrdup_(c"invalid layout").as_ptr();
                break 'fail;
            }

            /* Check this window will fit into the layout. */
            loop {
                let mut npanes = window_count_panes(w);
                let mut ncells = layout_count_cells(lc);
                if (npanes > ncells) {
                    xasprintf(cause, c"have %u panes but need %u".as_ptr(), npanes, ncells);
                    break 'fail;
                }
                if (npanes == ncells) {
                    break;
                }

                /* Fewer panes than cells - close the bottom right. */
                let lcchild = layout_find_bottomright(lc);
                layout_destroy_cell(w, lcchild, &raw mut lc);
            }

            /*
             * It appears older versions of tmux were able to generate layouts with
             * an incorrect top cell size - if it is larger than the top child then
             * correct that (if this is still wrong the check code will catch it).
             */
            let mut sy = 0;
            let mut sx = 0;
            match ((*lc).type_) {
                layout_type::LAYOUT_WINDOWPANE => (),
                layout_type::LAYOUT_LEFTRIGHT => {
                    tailq_foreach(&raw mut (*lc).cells, |lcchild| {
                        sy = (*lcchild).sy + 1;
                        sx += (*lcchild).sx + 1;
                        ControlFlow::<(), ()>::Continue(())
                    });
                }
                layout_type::LAYOUT_TOPBOTTOM => {
                    tailq_foreach(&raw mut (*lc).cells, |lcchild| {
                        sx = (*lcchild).sx + 1;
                        sy += (*lcchild).sy + 1;
                        ControlFlow::<(), ()>::Continue(())
                    });
                }
            }
            if ((*lc).type_ != layout_type::LAYOUT_WINDOWPANE && ((*lc).sx != sx || (*lc).sy != sy)) {
                log_debug(c"fix layout %u,%u to %u,%u".as_ptr(), (*lc).sx, (*lc).sy, sx, sy);
                layout_print_cell(lc, __func__, 0);
                (*lc).sx = sx - 1;
                (*lc).sy = sy - 1;
            }

            /* Check the new layout. */
            if (layout_check(lc) == 0) {
                *cause = xstrdup_(c"size mismatch after applying layout").as_ptr();
                break 'fail;
            }

            /* Resize to the layout size. */
            window_resize(w, (*lc).sx, (*lc).sy, -1, -1);

            /* Destroy the old layout and swap to the new. */
            layout_free_cell((*w).layout_root);
            (*w).layout_root = lc;

            /* Assign the panes into the cells. */
            let mut wp = tailq_first(&raw mut (*w).panes);
            layout_assign(&raw mut wp, lc);

            /* Update pane offsets and sizes. */
            layout_fix_offsets(w);
            layout_fix_panes(w, null_mut());
            recalculate_sizes();

            layout_print_cell(lc, __func__, 0);

            notify_window(c"window-layout-changed".as_ptr(), w);

            return 0;
        }
        // fail:
        layout_free_cell(lc);
        -1
    }
}

/* Assign panes into cells. */
#[unsafe(no_mangle)]
unsafe extern "C" fn layout_assign(wp: *mut *mut window_pane, lc: *mut layout_cell) {
    unsafe {
        match ((*lc).type_) {
            layout_type::LAYOUT_WINDOWPANE => {
                layout_make_leaf(lc, *wp);
                *wp = tailq_next::<_, _, discr_entry>(*wp);
            }
            layout_type::LAYOUT_LEFTRIGHT | layout_type::LAYOUT_TOPBOTTOM => {
                tailq_foreach(&raw mut (*lc).cells, |lcchild| {
                    layout_assign(wp, lcchild);
                    ControlFlow::<(), ()>::Continue(())
                });
            }
        }
    }
}

/* Construct a cell from all or part of a layout tree. */
#[unsafe(no_mangle)]
unsafe extern "C" fn layout_construct(lcparent: *mut layout_cell, layout: *mut *const c_char) -> *mut layout_cell {
    unsafe {
        let mut lc = null_mut();
        // struct layout_cell *lc, *lcchild;
        // u_int sx, sy, xoff, yoff;
        // const char *saved;
        let mut sx = 0u32;
        let mut sy = 0u32;
        let mut xoff = 0u32;
        let mut yoff = 0u32;

        'fail: {
            if (isdigit(**layout as i32) == 0) {
                return null_mut();
            }
            if (sscanf(
                *layout,
                c"%ux%u,%u,%u".as_ptr(),
                &raw mut sx,
                &raw mut sy,
                &raw mut xoff,
                &raw mut yoff,
            ) != 4)
            {
                return null_mut();
            }

            while (isdigit(**layout as i32) != 0) {
                (*layout) = (*layout).add(1);
            }
            if (**layout != b'x' as _) {
                return null_mut();
            }
            (*layout) = (*layout).add(1);
            while (isdigit(**layout as i32) != 0) {
                (*layout) = (*layout).add(1);
            }
            if (**layout != b',' as _) {
                return null_mut();
            }
            (*layout) = (*layout).add(1);
            while (isdigit(**layout as i32) != 0) {
                (*layout) = (*layout).add(1);
            }
            if (**layout != b',' as _) {
                return null_mut();
            }
            (*layout) = (*layout).add(1);
            while (isdigit(**layout as i32) != 0) {
                (*layout) = (*layout).add(1);
            }
            if (**layout == b',' as _) {
                let saved = *layout;
                (*layout) = (*layout).add(1);
                while (isdigit(**layout as i32) != 0) {
                    (*layout) = (*layout).add(1);
                }
                if (**layout == b'x' as _) {
                    *layout = saved;
                }
            }

            lc = layout_create_cell(lcparent);
            (*lc).sx = sx;
            (*lc).sy = sy;
            (*lc).xoff = xoff;
            (*lc).yoff = yoff;

            match (**layout as u8) {
                b',' | b'}' | b']' | b'\0' => return lc,
                b'{' => (*lc).type_ = layout_type::LAYOUT_LEFTRIGHT,
                b'[' => (*lc).type_ = layout_type::LAYOUT_TOPBOTTOM,
                _ => break 'fail,
            }

            loop {
                (*layout) = (*layout).add(1);
                let lcchild = layout_construct(lc, layout);
                if (lcchild.is_null()) {
                    break 'fail;
                }
                tailq_insert_tail(&raw mut (*lc).cells, lcchild);
                if (**layout != b',' as _) {
                    break;
                }
            }

            match ((*lc).type_) {
                layout_type::LAYOUT_LEFTRIGHT => {
                    if (**layout != b'}' as _) {
                        break 'fail;
                    }
                }
                layout_type::LAYOUT_TOPBOTTOM => {
                    if (**layout != b']' as _) {
                        break 'fail;
                    }
                }
                _ => break 'fail,
            }
            (*layout) = (*layout).add(1);

            return lc;
        }
        // fail:
        layout_free_cell(lc);
        null_mut()
    }
}
