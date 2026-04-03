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

//! Paste buffer management for tmux.
//!
//! Maintains a global set of named paste buffers used for copy/paste operations.
//! Buffers come in two flavors:
//!
//! - **Automatic** buffers: created by copy operations, auto-named (e.g. `buffer0`,
//!   `buffer1`), and subject to the `buffer-limit` option which caps how many are kept.
//!   Oldest automatic buffers are evicted when the limit is reached.
//!
//! - **Named** (manual) buffers: created explicitly by the user with `set-buffer -b name`.
//!   Not subject to the automatic buffer limit, not evicted.
//!
//! Storage uses two parallel data structures:
//! - `PASTE_BY_NAME`: a `BTreeMap<String, Box<PasteBuffer>>` — primary owner, keyed by name.
//! - `PASTE_ORDER`: a `Vec<String>` — names sorted by ascending `order` field (insertion order).
//!
//! Iteration via [`paste_walk`] goes in reverse order (newest first), matching the
//! original C tmux RB-tree behavior. [`paste_get_top`] returns the newest automatic buffer.

use std::collections::BTreeMap;

use crate::*;
use crate::options_::*;

/// A single paste buffer entry.
///
/// Contains the buffer data as an owned `Vec<u8>`, a name, creation
/// timestamp, whether it was automatically created, and an order field
/// for sorting by insertion time.
pub struct PasteBuffer {
    pub data: Vec<u8>,

    pub name: Cow<'static, str>,
    pub created: time_t,
    pub automatic: i32,
    pub order: u32,
}

static mut PASTE_NEXT_INDEX: u32 = 0;
static mut PASTE_NEXT_ORDER: u32 = 0;
static mut PASTE_NUM_AUTOMATIC: u32 = 0;

/// Primary store: name → buffer. Owns all paste buffers.
static mut PASTE_BY_NAME: BTreeMap<String, Box<PasteBuffer>> = BTreeMap::new();
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

/// Returns the name of the given paste buffer.
pub unsafe fn paste_buffer_name<'a>(pb: NonNull<PasteBuffer>) -> &'a str {
    unsafe { &(*pb.as_ptr()).name }
}

/// Returns the insertion order of the given paste buffer.
pub unsafe fn paste_buffer_order(pb: NonNull<PasteBuffer>) -> u32 {
    unsafe { (*pb.as_ptr()).order }
}

/// Returns the creation timestamp of the given paste buffer.
pub unsafe fn paste_buffer_created(pb: NonNull<PasteBuffer>) -> time_t {
    unsafe { (*pb.as_ptr()).created }
}

/// Returns the buffer data as a byte slice and optionally writes the size.
pub unsafe fn paste_buffer_data(pb: *mut PasteBuffer, size: *mut usize) -> *const u8 {
    unsafe {
        if !size.is_null() {
            *size = (*pb).data.len();
        }
        (*pb).data.as_ptr()
    }
}

/// Returns the buffer data as a byte slice. Safe-reference variant.
pub unsafe fn paste_buffer_data_(pb: NonNull<PasteBuffer>, size: &mut usize) -> *const u8 {
    unsafe {
        *size = (*pb.as_ptr()).data.len();
        (*pb.as_ptr()).data.as_ptr()
    }
}

/// Iterate buffers in reverse order (most recent first, matching C tmux's
/// RB-tree iteration which was descending by order field).
/// Pass null to get the first (most recent), pass a buffer to get the next.
pub unsafe fn paste_walk(pb: *mut PasteBuffer) -> *mut PasteBuffer {
    unsafe {
        let order_vec = &*(&raw mut PASTE_ORDER);
        let names = &*(&raw mut PASTE_BY_NAME);
        if pb.is_null() {
            // Return most recent buffer (last in order vec)
            for name in order_vec.iter().rev() {
                if let Some(buf) = names.get(name) {
                    return &**buf as *const PasteBuffer as *mut PasteBuffer;
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
                    return &**buf as *const PasteBuffer as *mut PasteBuffer;
                }
            }
            if name.as_str() == current_name.as_ref() {
                found = true;
            }
        }
        null_mut()
    }
}

/// Returns `true` if there are no paste buffers.
pub unsafe fn paste_is_empty() -> bool {
    unsafe { (*(&raw mut PASTE_BY_NAME)).is_empty() }
}

/// Returns the newest automatic paste buffer, or null if none exist.
/// If `name` is non-null, writes the buffer's name into it.
pub unsafe fn paste_get_top(name: *mut Option<&str>) -> *mut PasteBuffer {
    unsafe {
        // Walk in reverse order (most recent first) to find the newest automatic buffer.
        let order_vec = &*(&raw const PASTE_ORDER);
        for buf_name in order_vec.iter().rev() {
            let map = &mut *(&raw mut PASTE_BY_NAME);
            if let Some(buf) = map.get_mut(buf_name) {
                if buf.automatic != 0 {
                    let ptr = &mut **buf as *mut PasteBuffer;
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

/// Looks up a paste buffer by name. Returns null if not found or name is empty/None.
pub unsafe fn paste_get_name(name: Option<&str>) -> *mut PasteBuffer {
    unsafe {
        let Some(name) = name else {
            return null_mut();
        };
        if name.is_empty() {
            return null_mut();
        }
        (*(&raw mut PASTE_BY_NAME))
            .get_mut(name)
            .map_or(null_mut(), |pb| &mut **pb as *mut PasteBuffer)
    }
}

/// Frees a paste buffer, removing it from both `PASTE_BY_NAME` and `PASTE_ORDER`.
/// Decrements `PASTE_NUM_AUTOMATIC` if the buffer was automatic.
/// Sends a `paste-buffer-deleted` notification.
pub unsafe fn paste_free(pb: NonNull<PasteBuffer>) {
    unsafe {
        let pb = pb.as_ptr();
        let name = (*pb).name.to_string();
        notify_paste_buffer(&name, true);

        if (*pb).automatic != 0 {
            PASTE_NUM_AUTOMATIC -= 1;
        }

        paste_order_remove(&name);
        (*(&raw mut PASTE_BY_NAME)).remove(&name);
        // Box<PasteBuffer> dropped here — Vec<u8> data freed automatically
    }
}

/// Adds an automatic paste buffer with an auto-generated name (`<prefix><N>`).
/// Evicts oldest automatic buffers if `buffer-limit` is exceeded.
/// If `prefix` is null, defaults to `"buffer"`.
/// If `size` is 0, the data is freed and no buffer is created.
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
                            as *mut PasteBuffer,
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

        let owned_data = std::slice::from_raw_parts(data, size).to_vec();
        free_(data);

        let pb = Box::new(PasteBuffer {
            data: owned_data,
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

/// Renames a paste buffer. If `newname` already exists, the old buffer with that
/// name is freed first. The renamed buffer becomes non-automatic.
/// Returns 0 on success, -1 on error (writes error message to `cause`).
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

/// Creates or replaces a named paste buffer. If `name` is None, delegates to
/// [`paste_add`] to create an automatic buffer. If a buffer with the same name
/// exists, it is freed first. Returns 0 on success, -1 on error.
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

        let owned_data = std::slice::from_raw_parts(data, size).to_vec();
        free_(data);

        let pb = Box::new(PasteBuffer {
            data: owned_data,
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

/// Replaces the data in an existing paste buffer without changing its name or order.
pub unsafe fn paste_replace(pb: NonNull<PasteBuffer>, data: *mut u8, size: usize) {
    unsafe {
        let owned_data = std::slice::from_raw_parts(data, size).to_vec();
        free_(data);
        (*pb.as_ptr()).data = owned_data;

        notify_paste_buffer(&(*pb.as_ptr()).name, false);
    }
}

/// Creates a display-friendly sample of a paste buffer's contents.
/// Truncates to 200 characters and appends "..." if the buffer is longer.
/// Non-printable characters are vis-encoded.
pub unsafe fn paste_make_sample(pb: *mut PasteBuffer) -> String {
    unsafe {
        let width = 200;
        let data = &(*pb).data;

        let mut len = data.len();
        if len > width {
            len = width;
        }
        let mut buf: Vec<u8> = Vec::with_capacity(len * (4 + 4));

        utf8_strvis_(
            &mut buf,
            data.as_ptr(),
            len,
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL,
        );
        if data.len() > width || buf.len() > width {
            buf.extend(b"...");
        }
        String::from_utf8(buf).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Initialize global options needed by paste_add (reads "buffer-limit").
    unsafe fn init_globals() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            use crate::options_table::OPTIONS_TABLE;
            use crate::tmux::{GLOBAL_OPTIONS, GLOBAL_S_OPTIONS, GLOBAL_W_OPTIONS};

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

    /// Mutex to serialize tests that mutate paste buffer global state.
    static PASTE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Reset all paste buffer global state so tests are independent.
    unsafe fn reset_paste_state() {
        unsafe {
            // Vec<u8> data is freed automatically when Box<PasteBuffer> drops.
            (*(&raw mut PASTE_BY_NAME)).clear();
            (*(&raw mut PASTE_ORDER)).clear();
            PASTE_NEXT_INDEX = 0;
            PASTE_NEXT_ORDER = 0;
            PASTE_NUM_AUTOMATIC = 0;
        }
    }

    /// Allocate a test data buffer with the given content.
    /// Returns (pointer, size) suitable for paste_set/paste_add.
    unsafe fn make_data(s: &[u8]) -> (*mut u8, usize) {
        unsafe {
            let ptr = crate::xmalloc::xmalloc(s.len()).as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len());
            (ptr, s.len())
        }
    }

    /// Collect all buffer names from paste_walk in order (newest first).
    unsafe fn walk_names() -> Vec<String> {
        unsafe {
            let mut names = Vec::new();
            let mut pb = paste_walk(null_mut());
            while !pb.is_null() {
                names.push((*pb).name.to_string());
                pb = paste_walk(pb);
            }
            names
        }
    }

    // ---------------------------------------------------------------
    // paste_is_empty
    // ---------------------------------------------------------------

    #[test]
    fn empty_initially() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();
            assert!(paste_is_empty());
        }
    }

    // ---------------------------------------------------------------
    // paste_set — named (manual) buffers
    // ---------------------------------------------------------------

    #[test]
    fn set_named_buffer() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"hello");
            let rc = paste_set(data, size, Some("mybuf"), null_mut());
            assert_eq!(rc, 0);
            assert!(!paste_is_empty());

            let pb = paste_get_name(Some("mybuf"));
            assert!(!pb.is_null());
            assert_eq!((*pb).data.len(), 5);
            assert_eq!((*pb).automatic, 0);
        }
    }

    #[test]
    fn set_replaces_existing_name() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (d1, s1) = make_data(b"first");
            paste_set(d1, s1, Some("buf"), null_mut());

            let (d2, s2) = make_data(b"second");
            paste_set(d2, s2, Some("buf"), null_mut());

            // Should still be one buffer, with updated data.
            let pb = paste_get_name(Some("buf"));
            assert!(!pb.is_null());
            assert_eq!((*pb).data.len(), 6);
            assert_eq!(walk_names().len(), 1);
        }
    }

    #[test]
    fn set_empty_name_returns_error() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"hello");
            let mut cause: *mut u8 = null_mut();
            let rc = paste_set(data, size, Some(""), &raw mut cause);
            assert_eq!(rc, -1);
            assert!(!cause.is_null());
            free_(cause);
        }
    }

    #[test]
    fn set_zero_size_frees_data() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, _) = make_data(b"x");
            let rc = paste_set(data, 0, Some("buf"), null_mut());
            assert_eq!(rc, 0);
            assert!(paste_is_empty());
        }
    }

    #[test]
    fn set_none_name_delegates_to_add() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"auto");
            let rc = paste_set(data, size, None, null_mut());
            assert_eq!(rc, 0);
            assert!(!paste_is_empty());

            // Should have created an automatic buffer named "buffer0".
            let pb = paste_get_name(Some("buffer0"));
            assert!(!pb.is_null());
            assert_eq!((*pb).automatic, 1);
        }
    }

    // ---------------------------------------------------------------
    // paste_add — automatic buffers
    // ---------------------------------------------------------------

    #[test]
    fn add_creates_automatic_buffer() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"copied");
            paste_add(null_mut(), data, size);

            let pb = paste_get_name(Some("buffer0"));
            assert!(!pb.is_null());
            assert_eq!((*pb).automatic, 1);
            assert_eq!((*pb).data.len(), 6);
        }
    }

    #[test]
    fn add_auto_increments_name() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            for i in 0..3 {
                let (data, size) = make_data(format!("buf{i}").as_bytes());
                paste_add(null_mut(), data, size);
            }

            assert!(!paste_get_name(Some("buffer0")).is_null());
            assert!(!paste_get_name(Some("buffer1")).is_null());
            assert!(!paste_get_name(Some("buffer2")).is_null());
        }
    }

    #[test]
    fn add_zero_size_is_noop() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, _) = make_data(b"x");
            paste_add(null_mut(), data, 0);
            assert!(paste_is_empty());
        }
    }

    #[test]
    fn add_with_custom_prefix() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"hello");
            paste_add(c!("custom"), data, size);

            let pb = paste_get_name(Some("custom0"));
            assert!(!pb.is_null());
        }
    }

    #[test]
    fn add_evicts_oldest_when_over_limit() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            // Set buffer-limit to 3.
            options_set_number(
                crate::tmux::GLOBAL_OPTIONS,
                "buffer-limit",
                3,
            );

            // Add 4 automatic buffers — the oldest should be evicted.
            for i in 0..4u8 {
                let (data, size) = make_data(&[b'a' + i]);
                paste_add(null_mut(), data, size);
            }

            // buffer0 should have been evicted.
            assert!(paste_get_name(Some("buffer0")).is_null());
            assert!(!paste_get_name(Some("buffer1")).is_null());
            assert!(!paste_get_name(Some("buffer2")).is_null());
            assert!(!paste_get_name(Some("buffer3")).is_null());
        }
    }

    // ---------------------------------------------------------------
    // paste_walk — iteration order (newest first)
    // ---------------------------------------------------------------

    #[test]
    fn walk_empty() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();
            assert!(paste_walk(null_mut()).is_null());
        }
    }

    #[test]
    fn walk_returns_newest_first() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (d1, s1) = make_data(b"first");
            paste_set(d1, s1, Some("aaa"), null_mut());
            let (d2, s2) = make_data(b"second");
            paste_set(d2, s2, Some("bbb"), null_mut());
            let (d3, s3) = make_data(b"third");
            paste_set(d3, s3, Some("ccc"), null_mut());

            let names = walk_names();
            assert_eq!(names, vec!["ccc", "bbb", "aaa"]);
        }
    }

    // ---------------------------------------------------------------
    // paste_get_top — newest automatic buffer
    // ---------------------------------------------------------------

    #[test]
    fn get_top_returns_newest_automatic() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            // Add a named buffer then two automatic ones.
            let (d1, s1) = make_data(b"named");
            paste_set(d1, s1, Some("manual"), null_mut());
            let (d2, s2) = make_data(b"auto1");
            paste_add(null_mut(), d2, s2);
            let (d3, s3) = make_data(b"auto2");
            paste_add(null_mut(), d3, s3);

            let pb = paste_get_top(null_mut());
            assert!(!pb.is_null());
            // Newest automatic should be buffer1 (added last).
            assert_eq!((*pb).name.as_ref(), "buffer1");
        }
    }

    #[test]
    fn get_top_returns_null_when_no_automatic() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            // Only named buffers — get_top should return null.
            let (d, s) = make_data(b"named");
            paste_set(d, s, Some("manual"), null_mut());

            assert!(paste_get_top(null_mut()).is_null());
        }
    }

    // ---------------------------------------------------------------
    // paste_get_name — lookup
    // ---------------------------------------------------------------

    #[test]
    fn get_name_none_returns_null() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();
            assert!(paste_get_name(None).is_null());
        }
    }

    #[test]
    fn get_name_empty_returns_null() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();
            assert!(paste_get_name(Some("")).is_null());
        }
    }

    #[test]
    fn get_name_missing_returns_null() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();
            assert!(paste_get_name(Some("nonexistent")).is_null());
        }
    }

    // ---------------------------------------------------------------
    // paste_free
    // ---------------------------------------------------------------

    #[test]
    fn free_removes_buffer() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"to-delete");
            paste_set(data, size, Some("del"), null_mut());
            assert!(!paste_get_name(Some("del")).is_null());

            let pb = NonNull::new(paste_get_name(Some("del"))).unwrap();
            paste_free(pb);

            assert!(paste_get_name(Some("del")).is_null());
            assert!(paste_is_empty());
        }
    }

    #[test]
    fn free_automatic_decrements_count() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (d1, s1) = make_data(b"a1");
            paste_add(null_mut(), d1, s1);
            let (d2, s2) = make_data(b"a2");
            paste_add(null_mut(), d2, s2);
            assert_eq!(*(&raw const PASTE_NUM_AUTOMATIC), 2);

            let pb = NonNull::new(paste_get_name(Some("buffer0"))).unwrap();
            paste_free(pb);
            assert_eq!(*(&raw const PASTE_NUM_AUTOMATIC), 1);
        }
    }

    // ---------------------------------------------------------------
    // paste_rename
    // ---------------------------------------------------------------

    #[test]
    fn rename_success() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"content");
            paste_set(data, size, Some("old"), null_mut());

            let rc = paste_rename(Some("old"), Some("new"), null_mut());
            assert_eq!(rc, 0);

            assert!(paste_get_name(Some("old")).is_null());
            let pb = paste_get_name(Some("new"));
            assert!(!pb.is_null());
            assert_eq!((*pb).data.len(), 7);
            // Renamed buffer becomes non-automatic.
            assert_eq!((*pb).automatic, 0);
        }
    }

    #[test]
    fn rename_overwrites_existing_target() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (d1, s1) = make_data(b"src");
            paste_set(d1, s1, Some("from"), null_mut());
            let (d2, s2) = make_data(b"dst-old");
            paste_set(d2, s2, Some("to"), null_mut());

            let rc = paste_rename(Some("from"), Some("to"), null_mut());
            assert_eq!(rc, 0);

            assert!(paste_get_name(Some("from")).is_null());
            let pb = paste_get_name(Some("to"));
            assert!(!pb.is_null());
            // Should have the source data, not the old target data.
            assert_eq!((*pb).data.len(), 3);
        }
    }

    #[test]
    fn rename_missing_source_returns_error() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let mut cause: *mut u8 = null_mut();
            let rc = paste_rename(Some("nope"), Some("new"), &raw mut cause);
            assert_eq!(rc, -1);
            assert!(!cause.is_null());
            free_(cause);
        }
    }

    #[test]
    fn rename_none_oldname_returns_error() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let rc = paste_rename(None, Some("new"), null_mut());
            assert_eq!(rc, -1);
        }
    }

    #[test]
    fn rename_none_newname_returns_error() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"x");
            paste_set(data, size, Some("buf"), null_mut());

            let rc = paste_rename(Some("buf"), None, null_mut());
            assert_eq!(rc, -1);
        }
    }

    // ---------------------------------------------------------------
    // paste_replace — in-place data replacement
    // ---------------------------------------------------------------

    #[test]
    fn replace_updates_data() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (d1, s1) = make_data(b"old");
            paste_set(d1, s1, Some("buf"), null_mut());

            let pb = NonNull::new(paste_get_name(Some("buf"))).unwrap();
            let (d2, s2) = make_data(b"new-data");
            paste_replace(pb, d2, s2);

            assert_eq!((*pb.as_ptr()).data.len(), 8);
        }
    }

    // ---------------------------------------------------------------
    // PasteBuffer accessors
    // ---------------------------------------------------------------

    #[test]
    fn buffer_accessors() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"test");
            paste_set(data, size, Some("acc"), null_mut());

            let pb = NonNull::new(paste_get_name(Some("acc"))).unwrap();
            assert_eq!(paste_buffer_name(pb), "acc");
            assert!(paste_buffer_created(pb) > 0);

            let mut sz: usize = 0;
            let ptr = paste_buffer_data_(pb, &mut sz);
            assert_eq!(sz, 4);
            assert!(!ptr.is_null());
        }
    }

    // ---------------------------------------------------------------
    // paste_make_sample
    // ---------------------------------------------------------------

    #[test]
    fn make_sample_short_text() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            let (data, size) = make_data(b"hello world");
            paste_set(data, size, Some("s"), null_mut());

            let pb = paste_get_name(Some("s"));
            let sample = paste_make_sample(pb);
            assert_eq!(sample, "hello world");
        }
    }

    #[test]
    fn make_sample_truncates_long_text() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            // Create a buffer longer than 200 bytes.
            let long_data = vec![b'x'; 300];
            let (data, size) = make_data(&long_data);
            paste_set(data, size, Some("long"), null_mut());

            let pb = paste_get_name(Some("long"));
            let sample = paste_make_sample(pb);
            assert!(sample.ends_with("..."));
            // The sample (before "...") should be at most 200 chars.
            assert!(sample.len() <= 203);
        }
    }

    // ---------------------------------------------------------------
    // Order consistency: walk + set + free
    // ---------------------------------------------------------------

    #[test]
    fn order_consistent_after_mixed_operations() {
        unsafe {
            init_globals();
            let _lock = PASTE_LOCK.lock().unwrap();
            reset_paste_state();

            // Add mix of named and automatic buffers.
            let (d1, s1) = make_data(b"n1");
            paste_set(d1, s1, Some("named1"), null_mut());
            let (d2, s2) = make_data(b"a1");
            paste_add(null_mut(), d2, s2);
            let (d3, s3) = make_data(b"n2");
            paste_set(d3, s3, Some("named2"), null_mut());

            // Walk should return newest first.
            let names = walk_names();
            assert_eq!(names, vec!["named2", "buffer0", "named1"]);

            // Free the middle one.
            let pb = NonNull::new(paste_get_name(Some("buffer0"))).unwrap();
            paste_free(pb);

            let names = walk_names();
            assert_eq!(names, vec!["named2", "named1"]);
        }
    }
}
