// Copyright (c) 2021 Will <author@will.party>
// Copyright (c) 2022 Jeff Chiang <pobomp@gmail.com>
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
use std::collections::HashMap;
use std::collections::VecDeque;

use crate::*;

const MAX_HYPERLINKS: usize = 5000;

static HYPERLINKS_NEXT_EXTERNAL_ID: AtomicU64 = AtomicU64::new(1);

/// Global LRU list: (owning hyperlinks tree, inner ID). Front = oldest, back = newest.
static mut GLOBAL_HYPERLINKS_LRU: VecDeque<(*mut hyperlinks, u32)> = VecDeque::new();

pub struct hyperlinks_uri {
    pub internal_id: *mut u8,
    pub external_id: *mut u8,
    pub uri: *mut u8,
}

pub struct hyperlinks {
    pub next_inner: u32,
    /// Primary store: inner ID → hyperlink data (owns the data).
    pub by_inner: BTreeMap<u32, Box<hyperlinks_uri>>,
    /// Secondary index: (`internal_id`, uri) → inner ID.
    /// Anonymous URIs (empty `internal_id`) are NOT indexed here.
    pub by_uri: HashMap<(String, String), u32>,
    pub references: u32,
}

/// Resolve cell-level `link` ids back to their URI + external id for the
/// Grid's OSC-8 emission path. The Grid crate owns the
/// [`HyperlinkLookup`](crate::HyperlinkLookup) trait; this impl is the
/// tmux-rs side of the abstraction boundary, so the Grid code can stay
/// free of tmux-rs-specific types.
impl HyperlinkLookup for hyperlinks {
    fn hyperlink(&self, id: u32) -> Option<Hyperlink<'_>> {
        let hlu = self.by_inner.get(&id)?;
        // SAFETY: `uri` and `external_id` are NUL-terminated C strings
        // owned by the Box whose lifetime is tied to `&self`. The
        // returned byte slices borrow from the Box and are valid for
        // the duration of this borrow.
        unsafe {
            Some(Hyperlink {
                uri: std::ffi::CStr::from_ptr(hlu.uri.cast()).to_bytes(),
                external_id: std::ffi::CStr::from_ptr(hlu.external_id.cast()).to_bytes(),
            })
        }
    }
}

unsafe fn hyperlinks_remove_inner(hl: *mut hyperlinks, inner: u32) {
    unsafe {
        // Remove from primary store — get owned Box back
        if let Some(hlu) = (*hl).by_inner.remove(&inner) {
            // Remove from global LRU
            let lru = &mut *(&raw mut GLOBAL_HYPERLINKS_LRU);
            lru.retain(|&(tree, id)| !(tree == hl && id == inner));

            // Remove from URI secondary index
            let int_id = cstr_to_str(hlu.internal_id).to_string();
            let uri_str = cstr_to_str(hlu.uri).to_string();
            if !int_id.is_empty() {
                (*hl).by_uri.remove(&(int_id, uri_str));
            }

            // Free C strings
            free_(hlu.internal_id);
            free_(hlu.external_id);
            free_(hlu.uri);
            // Box dropped here
        }
    }
}

pub unsafe fn hyperlinks_put(
    hl: *mut hyperlinks,
    uri_in: *const u8,
    mut internal_id_in: *const u8,
) -> u32 {
    unsafe {
        if internal_id_in.is_null() {
            internal_id_in = c!("");
        }

        let uri_v = utf8_stravis(uri_in, vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE);
        let internal_id_v =
            utf8_stravis(internal_id_in, vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE);
        let uri_cs = CString::new(uri_v).unwrap_or_default();
        let internal_id_cs = CString::new(internal_id_v).unwrap_or_default();
        // Hand out xstrdup'd copies so the existing `hyperlinks_uri`
        // storage (raw `*mut u8`, freed via `free_`) keeps working.
        let uri = xstrdup(uri_cs.as_ptr().cast()).as_ptr();
        let internal_id = xstrdup(internal_id_cs.as_ptr().cast()).as_ptr();

        // Check if this (internal_id, uri) pair already exists
        if *internal_id_in != b'\0' {
            let int_id_str = cstr_to_str(internal_id).to_string();
            let uri_str = cstr_to_str(uri).to_string();
            if let Some(&existing_inner) = (*hl).by_uri.get(&(int_id_str, uri_str)) {
                free_(uri);
                free_(internal_id);
                return existing_inner;
            }
        }

        let id = HYPERLINKS_NEXT_EXTERNAL_ID.fetch_add(1, atomic::Ordering::Relaxed);
        let external_id: *mut u8 = format_nul!("tmux{:X}", id);

        let inner = (*hl).next_inner;
        (*hl).next_inner += 1;

        let hlu = Box::new(hyperlinks_uri {
            internal_id,
            external_id,
            uri,
        });

        // Add to URI index (only for non-anonymous URIs)
        let int_id_str = cstr_to_str(internal_id).to_string();
        if !int_id_str.is_empty() {
            let uri_str = cstr_to_str(uri).to_string();
            (*hl).by_uri.insert((int_id_str, uri_str), inner);
        }

        // Add to global LRU
        let lru = &mut *(&raw mut GLOBAL_HYPERLINKS_LRU);
        lru.push_back((hl, inner));
        if lru.len() >= MAX_HYPERLINKS {
            // Evict oldest
            let (oldest_hl, oldest_inner) = lru[0];
            hyperlinks_remove_inner(oldest_hl, oldest_inner);
        }

        // Insert into primary store
        (*hl).by_inner.insert(inner, hlu);

        inner
    }
}

pub unsafe fn hyperlinks_get(
    hl: *mut hyperlinks,
    inner: u32,
    uri_out: *mut *const u8,
    internal_id_out: *mut *const u8,
    external_id_out: *mut *const u8,
) -> bool {
    unsafe {
        let Some(hlu) = (*hl).by_inner.get(&inner) else {
            return false;
        };
        if !internal_id_out.is_null() {
            *internal_id_out = hlu.internal_id;
        }
        if !external_id_out.is_null() {
            *external_id_out = hlu.external_id;
        }
        *uri_out = hlu.uri as _;
        true
    }
}

pub unsafe fn hyperlinks_init() -> *mut hyperlinks {
    let hl = Box::new(hyperlinks {
        next_inner: 1,
        by_inner: BTreeMap::new(),
        by_uri: HashMap::new(),
        references: 1,
    });
    Box::into_raw(hl)
}

pub unsafe fn hyperlinks_copy(hl: *mut hyperlinks) -> *mut hyperlinks {
    unsafe {
        (*hl).references += 1;
    }
    hl
}

pub unsafe fn hyperlinks_reset(hl: *mut hyperlinks) {
    unsafe {
        let inners: Vec<u32> = (*hl).by_inner.keys().copied().collect();
        for inner in inners {
            hyperlinks_remove_inner(hl, inner);
        }
    }
}

pub unsafe fn hyperlinks_free(hl: *mut hyperlinks) {
    unsafe {
        (*hl).references -= 1;
        if (*hl).references == 0 {
            hyperlinks_reset(hl);
            drop(Box::from_raw(hl));
        }
    }
}
