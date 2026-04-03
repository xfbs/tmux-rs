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
use std::collections::BTreeMap;

use crate::*;
use crate::options_::*;

pub struct paste_buffer {
    pub data: *mut u8,
    pub size: usize,

    pub name: Cow<'static, str>,
    pub created: time_t,
    pub automatic: i32,
    pub order: u32,
}

static mut PASTE_NEXT_INDEX: u32 = 0;
static mut PASTE_NEXT_ORDER: u32 = 0;
static mut PASTE_NUM_AUTOMATIC: u32 = 0;

/// Primary store: name → buffer. Owns all paste buffers.
static mut PASTE_BY_NAME: BTreeMap<String, Box<paste_buffer>> = BTreeMap::new();
/// Order index: buffer names sorted by insertion order (ascending order field).
/// Maintained in sync with PASTE_BY_NAME.
static mut PASTE_ORDER: Vec<String> = Vec::new();

/// Insert a name into PASTE_ORDER maintaining sort by order field.
unsafe fn paste_order_insert(name: &str, order: u32) {
    unsafe {
        // Find insertion point — keep sorted by order ascending
        let pos = (*(&raw mut PASTE_ORDER))
            .iter()
            .position(|n| {
                (*(&raw mut PASTE_BY_NAME))
                    .get(n)
                    .is_none_or(|pb| pb.order > order)
            })
            .unwrap_or((*(&raw mut PASTE_ORDER)).len());
        (*(&raw mut PASTE_ORDER)).insert(pos, name.to_string());
    }
}

/// Remove a name from PASTE_ORDER.
unsafe fn paste_order_remove(name: &str) {
    unsafe {
        (*(&raw mut PASTE_ORDER)).retain(|n| n != name);
    }
}

pub unsafe fn paste_buffer_name<'a>(pb: NonNull<paste_buffer>) -> &'a str {
    unsafe { &(*pb.as_ptr()).name }
}

pub unsafe fn paste_buffer_order(pb: NonNull<paste_buffer>) -> u32 {
    unsafe { (*pb.as_ptr()).order }
}

pub unsafe fn paste_buffer_created(pb: NonNull<paste_buffer>) -> time_t {
    unsafe { (*pb.as_ptr()).created }
}

pub unsafe fn paste_buffer_data(pb: *mut paste_buffer, size: *mut usize) -> *const u8 {
    unsafe {
        if !size.is_null() {
            *size = (*pb).size;
        }
        (*pb).data
    }
}

pub unsafe fn paste_buffer_data_(pb: NonNull<paste_buffer>, size: &mut usize) -> *const u8 {
    unsafe {
        *size = (*pb.as_ptr()).size;
        (*pb.as_ptr()).data
    }
}

/// Iterate buffers in reverse order (most recent first, matching C tmux's
/// RB-tree iteration which was descending by order field).
/// Pass null to get the first (most recent), pass a buffer to get the next.
pub unsafe fn paste_walk(pb: *mut paste_buffer) -> *mut paste_buffer {
    unsafe {
        let order_vec = &*(&raw mut PASTE_ORDER);
        let names = &*(&raw mut PASTE_BY_NAME);
        if pb.is_null() {
            // Return most recent buffer (last in order vec)
            for name in order_vec.iter().rev() {
                if let Some(buf) = names.get(name) {
                    return &**buf as *const paste_buffer as *mut paste_buffer;
                }
            }
            return null_mut();
        }
        // Find the next buffer after pb (going toward oldest)
        let current_name = &(*pb).name;
        let mut found = false;
        for name in order_vec.iter().rev() {
            if found {
                if let Some(buf) = names.get(name) {
                    return &**buf as *const paste_buffer as *mut paste_buffer;
                }
            }
            if name.as_str() == current_name.as_ref() {
                found = true;
            }
        }
        null_mut()
    }
}

pub unsafe fn paste_is_empty() -> bool {
    unsafe { (*(&raw mut PASTE_BY_NAME)).is_empty() }
}

pub unsafe fn paste_get_top(name: *mut Option<&str>) -> *mut paste_buffer {
    unsafe {
        // Walk in reverse order (most recent first) to find the newest automatic buffer.
        let order_vec = &*(&raw const PASTE_ORDER);
        for buf_name in order_vec.iter().rev() {
            let map = &mut *(&raw mut PASTE_BY_NAME);
            if let Some(buf) = map.get_mut(buf_name) {
                if buf.automatic != 0 {
                    let ptr = &mut **buf as *mut paste_buffer;
                    if !name.is_null() {
                        *name = Some(&(*ptr).name);
                    }
                    return ptr;
                }
            }
        }
        null_mut()
    }
}

pub unsafe fn paste_get_name(name: Option<&str>) -> *mut paste_buffer {
    unsafe {
        let Some(name) = name else {
            return null_mut();
        };
        if name.is_empty() {
            return null_mut();
        }
        (*(&raw mut PASTE_BY_NAME))
            .get_mut(name)
            .map_or(null_mut(), |pb| &mut **pb as *mut paste_buffer)
    }
}

pub unsafe fn paste_free(pb: NonNull<paste_buffer>) {
    unsafe {
        let pb = pb.as_ptr();
        let name = (*pb).name.to_string();
        notify_paste_buffer(&name, true);

        if (*pb).automatic != 0 {
            PASTE_NUM_AUTOMATIC -= 1;
        }

        paste_order_remove(&name);
        if let Some(removed) = (*(&raw mut PASTE_BY_NAME)).remove(&name) {
            free_(removed.data);
        }
    }
}

pub unsafe fn paste_add(mut prefix: *const u8, data: *mut u8, size: usize) {
    unsafe {
        if prefix.is_null() {
            prefix = c!("buffer");
        }

        if size == 0 {
            free_(data);
            return;
        }

        let limit = options_get_number_(GLOBAL_OPTIONS, "buffer-limit");
        // Remove excess automatic buffers (oldest first = lowest order first)
        let names_to_check: Vec<String> = (*(&raw mut PASTE_ORDER)).iter().cloned().collect();
        for buf_name in &names_to_check {
            if (PASTE_NUM_AUTOMATIC as i64) < limit {
                break;
            }
            if let Some(buf) = (*(&raw mut PASTE_BY_NAME)).get(buf_name) {
                if buf.automatic != 0 {
                    let nn = NonNull::new(
                        &mut **(*(&raw mut PASTE_BY_NAME)).get_mut(buf_name).unwrap()
                            as *mut paste_buffer,
                    )
                    .unwrap();
                    paste_free(nn);
                }
            }
        }

        let order = PASTE_NEXT_ORDER;
        PASTE_NUM_AUTOMATIC += 1;
        PASTE_NEXT_ORDER += 1;

        let mut buf_name;
        loop {
            let tmp = PASTE_NEXT_INDEX;
            buf_name = format!("{}{}", _s(prefix), tmp);
            PASTE_NEXT_INDEX += 1;
            if paste_get_name(Some(&buf_name)).is_null() {
                break;
            }
        }

        let pb = Box::new(paste_buffer {
            data,
            size,
            name: Cow::Owned(buf_name.clone()),
            created: libc::time(null_mut()),
            automatic: 1,
            order,
        });

        paste_order_insert(&buf_name, order);
        (*(&raw mut PASTE_BY_NAME)).insert(buf_name.clone(), pb);

        notify_paste_buffer(&buf_name, false);
    }
}

pub unsafe fn paste_rename(
    oldname: Option<&str>,
    newname: Option<&str>,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        if !cause.is_null() {
            *cause = null_mut();
        }

        if oldname.is_none_or(str::is_empty) {
            if !cause.is_null() {
                *cause = xstrdup_(c"no buffer").as_ptr();
            }
            return -1;
        }
        if newname.is_none_or(str::is_empty) {
            if !cause.is_null() {
                *cause = xstrdup_(c"new name is empty").as_ptr();
            }
            return -1;
        }

        let oldname = oldname.unwrap();
        let newname = newname.unwrap();

        if (*(&raw mut PASTE_BY_NAME)).get(oldname).is_none() {
            if !cause.is_null() {
                *cause = format_nul!("no buffer {}", oldname);
            }
            return -1;
        }

        // Remove buffer with new name if it exists
        if let Some(pb_new) = NonNull::new(paste_get_name(Some(newname))) {
            paste_free(pb_new);
        }

        // Remove from map with old name, update name, re-insert with new name
        if let Some(mut pb) = (*(&raw mut PASTE_BY_NAME)).remove(oldname) {
            paste_order_remove(oldname);

            pb.name = Cow::Owned(newname.to_string());
            if pb.automatic != 0 {
                PASTE_NUM_AUTOMATIC -= 1;
            }
            pb.automatic = 0;

            paste_order_insert(newname, pb.order);
            (*(&raw mut PASTE_BY_NAME)).insert(newname.to_string(), pb);
        }

        notify_paste_buffer(oldname, true);
        notify_paste_buffer(newname, false);
    }
    0
}

pub unsafe fn paste_set(
    data: *mut u8,
    size: usize,
    name: Option<&str>,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        if !cause.is_null() {
            *cause = null_mut();
        }

        if size == 0 {
            free_(data);
            return 0;
        }
        let Some(name) = name else {
            paste_add(null_mut(), data, size);
            return 0;
        };

        if name.is_empty() {
            if !cause.is_null() {
                *cause = xstrdup_(c"empty buffer name").as_ptr();
            }
            return -1;
        }

        // Remove existing buffer with this name
        if let Some(old) = NonNull::new(paste_get_name(Some(name))) {
            paste_free(old);
        }

        let order = PASTE_NEXT_ORDER;
        PASTE_NEXT_ORDER += 1;

        let pb = Box::new(paste_buffer {
            data,
            size,
            name: Cow::Owned(name.to_string()),
            created: libc::time(null_mut()),
            automatic: 0,
            order,
        });

        paste_order_insert(name, order);
        (*(&raw mut PASTE_BY_NAME)).insert(name.to_string(), pb);

        notify_paste_buffer(name, false);
    }
    0
}

pub unsafe fn paste_replace(pb: NonNull<paste_buffer>, data: *mut u8, size: usize) {
    unsafe {
        free_((*pb.as_ptr()).data);
        (*pb.as_ptr()).data = data;
        (*pb.as_ptr()).size = size;

        notify_paste_buffer(&(*pb.as_ptr()).name, false);
    }
}

pub unsafe fn paste_make_sample(pb: *mut paste_buffer) -> String {
    unsafe {
        let width = 200;

        let mut len = (*pb).size;
        if len > width {
            len = width;
        }
        let mut buf: Vec<u8> = Vec::with_capacity(len * (4 + 4));

        utf8_strvis_(
            &mut buf,
            (*pb).data,
            len,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        if (*pb).size > width || buf.len() > width {
            buf.extend(b"...");
        }
        String::from_utf8(buf).unwrap()
    }
}
