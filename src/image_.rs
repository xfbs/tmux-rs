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

static mut ALL_IMAGES: Vec<*mut image> = Vec::new();

static mut ALL_IMAGES_COUNT: u32 = 0;

unsafe fn image_free_one(im: *mut image) {
    unsafe {
        (*(&raw mut ALL_IMAGES)).retain(|&p| p != im);
        ALL_IMAGES_COUNT -= 1;

        crate::image_sixel::sixel_free((*im).data);
        free_((*im).fallback);
        free_(im);
    }
}

pub unsafe fn image_free_all(s: *mut screen) -> bool {
    unsafe {
        let redraw = !(*s).images.is_empty();

        for &im in &(*s).images {
            (*(&raw mut ALL_IMAGES)).retain(|&p| p != im);
            ALL_IMAGES_COUNT -= 1;

            crate::image_sixel::sixel_free((*im).data);
            free_((*im).fallback);
            free_(im);
        }
        (*s).images.clear();
        redraw
    }
}

/// Create text placeholder for an image.
pub fn image_fallback(sx: u32, sy: u32) -> CString {
    let sx = sx as usize;
    let sy = sy as usize;

    let label = CString::new(format!("SIXEL IMAGE ({sx}x{sy})\r\n")).unwrap();

    // Allocate first line.
    let lsize = label.to_bytes_with_nul().len();
    let size = if sx < lsize - 3 { lsize - 1 } else { sx + 2 };
    // Remaining lines. Every placeholder line has \r\n at the end.
    let size = size + (sx + 2) * (sy - 1) + 1;

    let mut buf: Vec<u8> = Vec::with_capacity(size);

    // Render first line.
    if sx < lsize - 3 {
        buf.extend_from_slice(label.as_bytes());
    } else {
        buf.extend_from_slice(&label.as_bytes()[..(lsize - 3)]);
        buf.extend(std::iter::repeat_n(b'+', sx - lsize + 3));
        buf.extend_from_slice("\r\n".as_bytes());
    }

    // Remaining lines.
    for _ in 1..sy {
        buf.extend(std::iter::repeat_n(b'+', sx));
        buf.extend_from_slice("\r\n".as_bytes());
    }

    CString::new(buf).unwrap()
}

pub unsafe fn image_store(s: *mut screen, si: *mut sixel_image) -> *mut image {
    unsafe {
        let mut im = Box::new(image {
            s,
            data: si,
            px: (*s).cx,
            py: (*s).cy,
            sx: 0,
            sy: 0,
            fallback: null_mut(),
        });

        (im.sx, im.sy) = crate::image_sixel::sixel_size_in_cells(&*si);

        im.fallback = image_fallback(im.sx, im.sy).into_raw().cast();

        let im = Box::leak(im);
        (*s).images.push(im);
        (*(&raw mut ALL_IMAGES)).push(im);
        ALL_IMAGES_COUNT += 1;
        if ALL_IMAGES_COUNT == 10 {
            let oldest = *(*(&raw mut ALL_IMAGES)).first().unwrap();
            let oldest_screen = (*oldest).s;
            (*oldest_screen).images.retain(|&p| p != oldest);
            image_free_one(oldest);
        }

        im
    }
}

pub unsafe fn image_check_line(s: *mut screen, py: u32, ny: u32) -> bool {
    unsafe {
        let mut redraw = false;
        (*s).images.retain(|&im| {
            if py + ny > (*im).py && py < (*im).py + (*im).sy {
                image_free_one(im);
                redraw = true;
                false
            } else {
                true
            }
        });
        redraw
    }
}

pub unsafe fn image_check_area(s: *mut screen, px: u32, py: u32, nx: u32, ny: u32) -> bool {
    unsafe {
        let mut redraw = false;
        (*s).images.retain(|&im| {
            if py + ny <= (*im).py || py >= (*im).py + (*im).sy {
                return true;
            }
            if px + nx <= (*im).px || px >= (*im).px + (*im).sx {
                return true;
            }
            image_free_one(im);
            redraw = true;
            false
        });
        redraw
    }
}

pub unsafe fn image_scroll_up(s: *mut screen, lines: u32) -> bool {
    unsafe {
        let mut redraw = false;
        (*s).images.retain(|&im| {
            if (*im).py >= lines {
                (*im).py -= lines;
                redraw = true;
                return true;
            }
            if (*im).py + (*im).sy <= lines {
                image_free_one(im);
                redraw = true;
                return false;
            }
            let sx = (*im).sx;
            let sy = ((*im).py + (*im).sy) - lines;

            let new = crate::image_sixel::sixel_scale(
                (*im).data,
                0,
                0,
                0,
                (*im).sy - sy,
                sx,
                sy,
                1,
            );
            crate::image_sixel::sixel_free((*im).data);
            (*im).data = new;

            (*im).py = 0;
            ((*im).sx, (*im).sy) =
                crate::image_sixel::sixel_size_in_cells(&*(*im).data);

            free_((*im).fallback);
            (*im).fallback = image_fallback((*im).sx, (*im).sy)
                .into_raw()
                .cast();
            redraw = true;
            true
        });
        redraw
    }
}
