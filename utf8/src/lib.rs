//! UTF-8 cell types, encoding, width lookup, and intern table.
//!
//! This crate owns the `Utf8Data` / `Utf8Char` pair plus everything
//! needed to translate between them: a process-local intern table for
//! >3-byte sequences, a unicode-width lookup for display columns, and
//! the streaming-decode helpers (`utf8_open` / `utf8_append`) used by
//! the input parser.
//!
//! ## Safe API
//!
//! The underscore-style `utf8_*` freestanding functions retain their
//! historical C-era signatures (raw pointers, out-parameters) because
//! ~40 call sites across tmux-rs depend on them. New code should
//! prefer the method-based API on [`Utf8Data`]:
//!
//! ```ignore
//! let ud = Utf8Data::single(b'A');          // was: utf8_set
//! let width = ud.width();                   // safer wrapper
//! let hit = ud.in_set(b"abc\0");            // was: utf8_cstrhas
//! let uc = ud.encode().unwrap();            // was: utf8_from_data
//! let round = Utf8Data::from_char(uc);      // was: utf8_to_data
//! ```
//!
//! The freestanding fns are shims over these methods — a future
//! refactor can migrate callers and delete them.

// Several of the freestanding fns are translated-from-C with nested
// pointer arithmetic; inline SAFETY comments explain each site.
#![allow(unsafe_op_in_unsafe_fn)]
// `utf8_item_index` / `utf8_item_data` are the snake_case names the
// tmux-rs tree references via re-export. Rename is tracked separately.
#![allow(non_camel_case_types)]

use libc::wchar_t;
use std::cell::RefCell;

// libc crate doesn't expose these on Linux; declare the externs directly.
// (Not widely a portability issue — tmux-rs already ships for Linux/macOS/BSD.)
unsafe extern "C" {
    fn mbtowc(pwc: *mut wchar_t, s: *const u8, n: usize) -> i32;
    fn wctomb(s: *mut u8, wc: wchar_t) -> i32;
}

#[cfg(target_os = "linux")]
fn mb_cur_max() -> usize {
    unsafe extern "C" {
        unsafe fn __ctype_get_mb_cur_max() -> usize;
    }
    unsafe { __ctype_get_mb_cur_max() }
}

#[cfg(not(target_os = "linux"))]
fn mb_cur_max() -> usize {
    unsafe extern "C" {
        unsafe fn ___mb_cur_max() -> i32;
    }
    unsafe { ___mb_cur_max() as usize }
}
use std::collections::BTreeMap;
use std::ffi::c_uchar;
use std::fmt::{self, Display};
use std::mem::MaybeUninit;
use std::ptr::{null, null_mut};
use std::slice;
use unicode_width::UnicodeWidthChar;

mod data;

pub use data::{UTF8_SIZE, Utf8Char, Utf8Data};

/// Result of a streaming UTF-8 decode or an encode-to-packed-form.
///
/// - `More` — need another byte (streaming decode in progress).
/// - `Done` — complete, result is valid.
/// - `Error` — sequence is malformed or cannot be represented.
///
/// Associated consts `UTF8_MORE`/`UTF8_DONE`/`UTF8_ERROR` are provided
/// as aliases so legacy call sites (spelled `utf8_state::UTF8_DONE`)
/// keep working during the migration.
#[repr(i32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Utf8State {
    More,
    Done,
    Error,
}

#[allow(non_upper_case_globals)]
impl Utf8State {
    pub const UTF8_MORE: Self = Self::More;
    pub const UTF8_DONE: Self = Self::Done;
    pub const UTF8_ERROR: Self = Self::Error;
}

// ---------------------------------------------------------------------
// Bit packing for Utf8Char
//
// Layout (u32):
//   bits 31..29 — width + 1 (so 0 is sentinel "unset")
//   bits 28..24 — size (up to UTF8_SIZE)
//   bits 23..0  — either the low bytes of the char (size ≤ 3) or an
//                 intern-table index (size > 3)
// ---------------------------------------------------------------------

fn utf8_get_size(uc: Utf8Char) -> u8 {
    (((uc) >> 24) & 0x1f) as u8
}
fn utf8_get_width(uc: Utf8Char) -> u8 {
    (((uc) >> 29) - 1) as u8
}
fn utf8_set_size(size: u8) -> Utf8Char {
    (size as Utf8Char) << 24
}
fn utf8_set_width(width: u8) -> Utf8Char {
    (width as Utf8Char + 1) << 29
}

// ---------------------------------------------------------------------
// Intern table
//
// Characters wider than 3 bytes can't fit in the packed form's low 24
// bits; instead we store them in a thread-local interning table and
// use the low bits as an index. The table is append-only and caps at
// 2^24 entries.
// ---------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct utf8_item_index {
    pub index: u32,
}

#[derive(Clone, Copy)]
pub struct utf8_item_data {
    data: [MaybeUninit<u8>; UTF8_SIZE],
    size: u8,
}

impl utf8_item_data {
    fn new(bytes: &[u8]) -> Self {
        assert!(bytes.len() <= UTF8_SIZE);
        let mut data = [MaybeUninit::new(0); UTF8_SIZE];
        for (i, ch) in bytes.iter().enumerate() {
            data[i] = MaybeUninit::new(*ch);
        }
        Self { data, size: bytes.len() as u8 }
    }

    fn initialized_slice(&self) -> &[u8] {
        // SAFETY: type invariant — bytes up to `size` were written by `new`.
        unsafe {
            std::slice::from_raw_parts(self.data.as_ptr().cast(), self.size as usize)
        }
    }
}

impl Display for utf8_item_data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            std::str::from_utf8(self.initialized_slice())
                .unwrap_or("invalid utf8 in utf8_item_data"),
        )
    }
}

impl Ord for utf8_item_data {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.initialized_slice().cmp(other.initialized_slice())
    }
}
impl PartialEq for utf8_item_data {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for utf8_item_data {}
impl PartialOrd for utf8_item_data {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

thread_local! {
    static UTF8_DATA_TREE: RefCell<BTreeMap<utf8_item_data, utf8_item_index>> = const { RefCell::new(BTreeMap::new()) };
    static UTF8_INDEX_TREE: RefCell<BTreeMap<utf8_item_index, utf8_item_data>> = const { RefCell::new(BTreeMap::new()) };
}

static mut UTF8_NEXT_INDEX: u32 = 0;

pub fn utf8_item_by_data(item: &utf8_item_data) -> Option<utf8_item_index> {
    UTF8_DATA_TREE.with(|tree| tree.borrow().get(item).copied())
}

pub fn utf8_item_by_index(index: u32) -> Option<utf8_item_data> {
    let ui = utf8_item_index { index };
    UTF8_INDEX_TREE.with(|tree| tree.borrow().get(&ui).copied())
}

/// Insert `data[..size]` into the intern table (if absent) and write
/// its index into `*index`. Returns 0 on success, -1 if the table is
/// full (2^24 entries).
///
/// # Safety
/// `data` must be valid for `size` bytes; `index` must be writable.
pub unsafe fn utf8_put_item(data: *const [u8; UTF8_SIZE], size: usize, index: *mut u32) -> i32 {
    unsafe {
        let ud = &utf8_item_data::new(slice::from_raw_parts(data.cast(), size));
        if let Some(ui) = utf8_item_by_data(ud) {
            *index = ui.index;
            log::debug!("utf8_put_item: found {} = {}", ud, ui.index);
            return 0;
        }

        if UTF8_NEXT_INDEX == 0xffffff + 1 {
            return -1;
        }

        let ui_index = utf8_item_index { index: UTF8_NEXT_INDEX };
        UTF8_NEXT_INDEX += 1;

        let ui_data = *ud;
        UTF8_INDEX_TREE.with(|tree| tree.borrow_mut().insert(ui_index, ui_data));
        UTF8_DATA_TREE.with(|tree| tree.borrow_mut().insert(ui_data, ui_index));

        *index = ui_index.index;
        log::debug!("utf8_put_item: added {} = {}", ui_data, ui_index.index);
        0
    }
}

// ---------------------------------------------------------------------
// Width table and classification
// ---------------------------------------------------------------------

/// Emoji codepoints that should render as double-width even when
/// `wcwidth` reports 1. Kept in sorted order so the lookup is a binary
/// search via [`utf8_in_table`].
static UTF8_FORCE_WIDE: [wchar_t; 162] = [
    0x0261D, 0x026F9, 0x0270A, 0x0270B, 0x0270C, 0x0270D, 0x1F1E6, 0x1F1E7, 0x1F1E8, 0x1F1E9,
    0x1F1EA, 0x1F1EB, 0x1F1EC, 0x1F1ED, 0x1F1EE, 0x1F1EF, 0x1F1F0, 0x1F1F1, 0x1F1F2, 0x1F1F3,
    0x1F1F4, 0x1F1F5, 0x1F1F6, 0x1F1F7, 0x1F1F8, 0x1F1F9, 0x1F1FA, 0x1F1FB, 0x1F1FC, 0x1F1FD,
    0x1F1FE, 0x1F1FF, 0x1F385, 0x1F3C2, 0x1F3C3, 0x1F3C4, 0x1F3C7, 0x1F3CA, 0x1F3CB, 0x1F3CC,
    0x1F3FB, 0x1F3FC, 0x1F3FD, 0x1F3FE, 0x1F3FF, 0x1F442, 0x1F443, 0x1F446, 0x1F447, 0x1F448,
    0x1F449, 0x1F44A, 0x1F44B, 0x1F44C, 0x1F44D, 0x1F44E, 0x1F44F, 0x1F450, 0x1F466, 0x1F467,
    0x1F468, 0x1F469, 0x1F46B, 0x1F46C, 0x1F46D, 0x1F46E, 0x1F470, 0x1F471, 0x1F472, 0x1F473,
    0x1F474, 0x1F475, 0x1F476, 0x1F477, 0x1F478, 0x1F47C, 0x1F481, 0x1F482, 0x1F483, 0x1F485,
    0x1F486, 0x1F487, 0x1F48F, 0x1F491, 0x1F4AA, 0x1F574, 0x1F575, 0x1F57A, 0x1F590, 0x1F595,
    0x1F596, 0x1F645, 0x1F646, 0x1F647, 0x1F64B, 0x1F64C, 0x1F64D, 0x1F64E, 0x1F64F, 0x1F6A3,
    0x1F6B4, 0x1F6B5, 0x1F6B6, 0x1F6C0, 0x1F6CC, 0x1F90C, 0x1F90F, 0x1F918, 0x1F919, 0x1F91A,
    0x1F91B, 0x1F91C, 0x1F91D, 0x1F91E, 0x1F91F, 0x1F926, 0x1F930, 0x1F931, 0x1F932, 0x1F933,
    0x1F934, 0x1F935, 0x1F936, 0x1F937, 0x1F938, 0x1F939, 0x1F93D, 0x1F93E, 0x1F977, 0x1F9B5,
    0x1F9B6, 0x1F9B8, 0x1F9B9, 0x1F9BB, 0x1F9CD, 0x1F9CE, 0x1F9CF, 0x1F9D1, 0x1F9D2, 0x1F9D3,
    0x1F9D4, 0x1F9D5, 0x1F9D6, 0x1F9D7, 0x1F9D8, 0x1F9D9, 0x1F9DA, 0x1F9DB, 0x1F9DC, 0x1F9DD,
    0x1FAC3, 0x1FAC4, 0x1FAC5, 0x1FAF0, 0x1FAF1, 0x1FAF2, 0x1FAF3, 0x1FAF4, 0x1FAF5, 0x1FAF6,
    0x1FAF7, 0x1FAF8,
];

/// Binary search for `find` in a sorted `wchar_t` table. Used by
/// [`utf8_width`] to consult the force-wide override list.
pub fn utf8_in_table(find: wchar_t, table: &[wchar_t]) -> bool {
    table.binary_search(&find).is_ok()
}

// ---------------------------------------------------------------------
// Codec: between expanded Utf8Data and packed Utf8Char
// ---------------------------------------------------------------------

/// Pack a fully-decoded `Utf8Data` into a `Utf8Char`. For sizes ≤ 3
/// the packed form stores the bytes inline; larger sequences are
/// stored in the intern table and the packed form holds an index.
///
/// Returns `Done` on success. Returns `Error` and substitutes a
/// placeholder (space or double-space) when the input is malformed or
/// the intern table is full.
///
/// # Safety
/// `ud` must be a valid read; `uc` must be writable.
pub unsafe fn utf8_from_data(ud: *const Utf8Data, uc: *mut Utf8Char) -> Utf8State {
    unsafe {
        let mut index: u32 = 0;
        'fail: {
            if (*ud).width > 2 {
                panic!("invalid UTF-8 width: {}", (*ud).width);
            }
            if (*ud).size > UTF8_SIZE as u8 {
                break 'fail;
            }
            if (*ud).size <= 3 {
                index = (((*ud).data[2] as u32) << 16)
                    | (((*ud).data[1] as u32) << 8)
                    | ((*ud).data[0] as u32);
            } else if utf8_put_item(
                (&raw const (*ud).data).cast(),
                (*ud).size as usize,
                &raw mut index,
            ) != 0
            {
                break 'fail;
            }
            *uc = utf8_set_size((*ud).size) | utf8_set_width((*ud).width) | index;
            log::debug!(
                "utf8_from_data: width={} size={} -> {:08x}",
                (*ud).width,
                (*ud).size,
                *uc
            );
            return Utf8State::Done;
        }

        // fail:
        *uc = if (*ud).width == 0 {
            utf8_set_size(0) | utf8_set_width(0)
        } else if (*ud).width == 1 {
            utf8_set_size(1) | utf8_set_width(1) | 0x20
        } else {
            utf8_set_size(1) | utf8_set_width(1) | 0x2020
        };
        Utf8State::Error
    }
}

/// Unpack a `Utf8Char` back into a `Utf8Data`. The inverse of
/// [`utf8_from_data`]; for sizes > 3 this consults the intern table.
pub fn utf8_to_data(uc: Utf8Char) -> Utf8Data {
    let mut ud = Utf8Data {
        data: [0; UTF8_SIZE],
        size: utf8_get_size(uc),
        have: utf8_get_size(uc),
        width: utf8_get_width(uc),
    };

    if ud.size <= 3 {
        ud.data[2] = (uc >> 16) as u8;
        ud.data[1] = ((uc >> 8) & 0xff) as u8;
        ud.data[0] = (uc & 0xff) as u8;
    } else {
        let index = uc & 0xffffff;
        if let Some(ui) = utf8_item_by_index(index) {
            ud.data[..ud.size as usize].copy_from_slice(ui.initialized_slice());
        } else {
            ud.data[..ud.size as usize].fill(b' ');
        }
    }

    log::debug!(
        "utf8_to_data: {:08x} -> width={} size={}",
        uc,
        ud.width,
        ud.size
    );
    ud
}

/// Pack a single ASCII byte into a `Utf8Char`: size=1, width=1.
pub fn utf8_build_one(ch: c_uchar) -> u32 {
    utf8_set_size(1) | utf8_set_width(1) | ch as u32
}

/// Initialize `*ud` with a single ASCII byte (size=1, width=1).
///
/// # Safety
/// `ud` must be writable.
pub unsafe fn utf8_set(ud: *mut Utf8Data, ch: c_uchar) {
    unsafe {
        (*ud).have = 1;
        (*ud).size = 1;
        (*ud).width = 1;
        (*ud).data = [0; UTF8_SIZE];
        (*ud).data[0] = ch;
    }
}

/// Copy `*from` into `*to`, zero-filling bytes beyond `to.size`.
///
/// # Safety
/// Both pointers must be valid and non-aliasing.
pub unsafe fn utf8_copy(to: *mut Utf8Data, from: *const Utf8Data) {
    unsafe {
        (*to) = *from;
        for i in (*to).size..(UTF8_SIZE as u8) {
            (*to).data[i as usize] = 0;
        }
    }
}

/// Compute the display width of a character in columns (0, 1, or 2).
/// Consults the force-wide override first; falls back to
/// `unicode-width` with a final adjustment for C0/C1 control ranges.
///
/// # Safety
/// `ud` and `width` must be valid pointers.
pub unsafe fn utf8_width(ud: *mut Utf8Data, width: *mut i32) -> Utf8State {
    unsafe {
        let mut wc: wchar_t = 0;
        if utf8_towc(ud, &raw mut wc) != Utf8State::Done {
            return Utf8State::Error;
        }
        if utf8_in_table(wc, &UTF8_FORCE_WIDE) {
            *width = 2;
            return Utf8State::Done;
        }
        // `unicode-width` is a pure-Rust replacement for the former
        // `utf8proc_wcwidth` / libc `wcwidth` path. Returns None for
        // control chars — we map those to 0 (C1) or 1 (other) to
        // preserve the historical behavior.
        let columns = char::from_u32(wc as u32).and_then(|c| c.width());
        *width = match columns {
            Some(w) => w as i32,
            None => {
                if (0x80..=0x9f).contains(&wc) { 0 } else { 1 }
            }
        };
        log::debug!("utf8_width({:05X}) = {}", wc, *width);
        if *width >= 0 && *width <= 0xff {
            return Utf8State::Done;
        }
        Utf8State::Error
    }
}

/// Decode `*ud` (raw bytes) into a wide character (`*wc`). Uses libc
/// `mbtowc` — the current locale must be UTF-8 for this to produce
/// Unicode codepoints.
///
/// # Safety
/// `ud` and `wc` must be valid pointers.
pub unsafe fn utf8_towc(ud: *const Utf8Data, wc: *mut wchar_t) -> Utf8State {
    unsafe {
        let value = mbtowc(wc, (*ud).data.as_ptr().cast(), (*ud).size as usize);
        match value {
            -1 => {
                log::debug!("mbtowc failed on {} bytes", (*ud).size);
                mbtowc(null_mut(), null(), mb_cur_max());
                return Utf8State::Error;
            }
            0 => return Utf8State::Error,
            _ => (),
        }
    }
    Utf8State::Done
}

/// Encode a wide character `wc` back into a `Utf8Data`, populating
/// bytes and width. Uses libc `wctomb`.
///
/// # Safety
/// `ud` must be writable.
pub unsafe fn utf8_fromwc(wc: wchar_t, ud: *mut Utf8Data) -> Utf8State {
    unsafe {
        let mut width: i32 = 0;
        let size = wctomb((*ud).data.as_mut_ptr().cast(), wc);
        if size < 0 {
            log::debug!("wctomb failed on {}", wc);
            wctomb(null_mut(), 0);
            return Utf8State::Error;
        }
        if size == 0 {
            return Utf8State::Error;
        }
        (*ud).have = size as u8;
        (*ud).size = size as u8;
        if utf8_width(ud, &raw mut width) == Utf8State::Done {
            (*ud).width = width as u8;
            return Utf8State::Done;
        }
    }
    Utf8State::Error
}

// ---------------------------------------------------------------------
// Streaming decode: open with a lead byte, append continuation bytes
// ---------------------------------------------------------------------

/// Start a streaming decode: initialize `*ud` from a lead byte `ch`.
/// Returns `More` if `ch` was a valid lead for a 2–4 byte sequence;
/// `Error` otherwise.
///
/// # Safety
/// `ud` must be writable for a full `Utf8Data`.
pub unsafe fn utf8_open(ud: *mut Utf8Data, ch: c_uchar) -> Utf8State {
    unsafe {
        *ud = Utf8Data { data: [0; UTF8_SIZE], have: 0, size: 0, width: 0 };
        (*ud).size = match ch {
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => return Utf8State::Error,
        };
        utf8_append(ud, ch);
    }
    Utf8State::More
}

/// Append a byte to an in-progress streaming decode. Returns `More`
/// if more bytes are expected, `Done` when the sequence completes,
/// `Error` on invalid continuation.
///
/// # Safety
/// `ud` must be writable.
pub unsafe fn utf8_append(ud: *mut Utf8Data, ch: c_uchar) -> Utf8State {
    unsafe {
        let mut width: i32 = 0;
        if (*ud).have >= (*ud).size {
            panic!("UTF-8 character overflow");
        }
        if (*ud).size > UTF8_SIZE as u8 {
            panic!("UTF-8 character size too large");
        }
        if (*ud).have != 0 && (ch & 0xc0) != 0x80 {
            (*ud).width = 0xff;
        }
        (*ud).data[(*ud).have as usize] = ch;
        (*ud).have += 1;
        if (*ud).have != (*ud).size {
            return Utf8State::More;
        }
        if (*ud).width == 0xff {
            return Utf8State::Error;
        }
        if utf8_width(ud, &raw mut width) != Utf8State::Done {
            return Utf8State::Error;
        }
        (*ud).width = width as u8;
    }
    Utf8State::Done
}

/// Validate a NUL-terminated byte string: every byte must either be a
/// printable ASCII character (0x20..=0x7e) or start/continue a valid
/// UTF-8 sequence.
///
/// # Safety
/// `s` must be a valid NUL-terminated byte string.
pub unsafe fn utf8_isvalid(mut s: *const u8) -> bool {
    unsafe {
        let mut ud: Utf8Data = Utf8Data { data: [0; UTF8_SIZE], have: 0, size: 0, width: 0 };
        let end = {
            let mut p = s;
            while *p != 0 {
                p = p.add(1);
            }
            p
        };
        while s < end {
            let mut more = utf8_open(&raw mut ud, *s);
            if more == Utf8State::More {
                while {
                    s = s.add(1);
                    s < end && more == Utf8State::More
                } {
                    more = utf8_append(&raw mut ud, *s);
                }
                if more == Utf8State::Done {
                    continue;
                }
                return false;
            }
            if *s < 0x20 || *s > 0x7e {
                return false;
            }
            s = s.add(1);
        }
    }
    true
}

/// Word-set membership: does the character described by `ud` appear
/// in the NUL-terminated UTF-8 byte set `set`? Used by the grid
/// reader's word-boundary classification.
///
/// # Safety
/// `set` must be a valid NUL-terminated byte string and `ud` a valid
/// pointer for reading.
pub unsafe fn utf8_cstrhas(set: *const u8, ud: *const Utf8Data) -> bool {
    // Walk `set` byte by byte, accumulating each codepoint via
    // utf8_open/utf8_append, comparing each completed codepoint
    // against `ud`.
    if set.is_null() {
        return false;
    }
    let target = unsafe { (*ud).initialized_slice() };

    unsafe {
        let mut p = set;
        while *p != 0 {
            let mut candidate: Utf8Data = Utf8Data { data: [0; UTF8_SIZE], have: 0, size: 0, width: 0 };
            let byte = *p;
            if (0xc2..=0xf4).contains(&byte) {
                // Multi-byte — try streaming decode.
                let mut state = utf8_open(&raw mut candidate, byte);
                while state == Utf8State::More {
                    p = p.add(1);
                    if *p == 0 {
                        break;
                    }
                    state = utf8_append(&raw mut candidate, *p);
                }
                if state == Utf8State::Done
                    && candidate.size == (*ud).size
                    && &candidate.data[..candidate.size as usize] == target
                {
                    return true;
                }
                if *p != 0 {
                    p = p.add(1);
                }
            } else {
                // Single byte — direct compare.
                if (*ud).size == 1 && (*ud).data[0] == byte {
                    return true;
                }
                p = p.add(1);
            }
        }
    }
    false
}

// ---------------------------------------------------------------------
// Safe method API on Utf8Data
//
// Preferred surface for new code. Each method wraps the corresponding
// freestanding `utf8_*` function but hides the raw-pointer ceremony.
// ---------------------------------------------------------------------

/// Errors from the safe encode path.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Utf8Error {
    /// Input was malformed, too long, or the intern table is full.
    /// The packed form produced in error cases is a replacement char
    /// (space or double-space depending on requested width) — see
    /// [`utf8_from_data`] for details.
    Invalid,
}

impl Utf8Data {
    /// Construct a `Utf8Data` for a single ASCII byte. Convenience
    /// wrapper over [`utf8_set`] for the common "printable char" case.
    pub fn single(ch: u8) -> Self {
        let mut ud = Utf8Data { data: [0; UTF8_SIZE], have: 0, size: 0, width: 0 };
        // SAFETY: `ud` is on the stack, exclusive, fully owned.
        unsafe { utf8_set(&raw mut ud, ch); }
        ud
    }

    /// Encode into the packed [`Utf8Char`] form. Returns `Err` if the
    /// sequence is invalid or the intern table is full — a substitute
    /// placeholder is still produced by [`utf8_from_data`], but the
    /// safe wrapper surfaces the error rather than silently masking.
    pub fn encode(&self) -> Result<Utf8Char, Utf8Error> {
        let mut uc: Utf8Char = 0;
        // SAFETY: `self` is a valid reference; `uc` is a stack local.
        let state = unsafe { utf8_from_data(self as *const Utf8Data, &raw mut uc) };
        match state {
            Utf8State::Done => Ok(uc),
            _ => Err(Utf8Error::Invalid),
        }
    }

    /// Decode a `Utf8Char` back into a `Utf8Data`. Inverse of
    /// [`Utf8Data::encode`].
    pub fn from_char(uc: Utf8Char) -> Self {
        utf8_to_data(uc)
    }

    /// Return the display width (columns) of this character, if it
    /// can be determined. Returns `None` on decode errors; 0-column
    /// control characters report `Some(0)`.
    pub fn column_width(&self) -> Option<u8> {
        let mut this = *self;
        let mut width: i32 = 0;
        // SAFETY: `this` is on the stack, exclusive.
        let state = unsafe { utf8_width(&raw mut this, &raw mut width) };
        match state {
            Utf8State::Done => Some(width as u8),
            _ => None,
        }
    }

    /// Return `true` if this character appears in the NUL-terminated
    /// byte set `set`. Passes through to [`utf8_cstrhas`].
    ///
    /// # Safety
    /// `set` must be a valid NUL-terminated byte string.
    pub unsafe fn in_set(&self, set: *const u8) -> bool {
        // SAFETY: caller upholds NUL-termination; `self` is a valid ref.
        unsafe { utf8_cstrhas(set, self as *const Utf8Data) }
    }

    /// Safe variant of [`Utf8Data::in_set`] taking a `&CStr`. Preferred
    /// entry point for code that already holds Rust-owned strings.
    pub fn in_cstr(&self, set: &std::ffi::CStr) -> bool {
        // SAFETY: `CStr::as_ptr` yields a valid NUL-terminated byte string.
        unsafe { utf8_cstrhas(set.as_ptr().cast(), self as *const Utf8Data) }
    }

    /// Zeroed placeholder — equivalent to the C `{0}` initializer pattern.
    /// Used as the starting state for streaming decode via [`Utf8Data::open`].
    pub const fn empty() -> Self {
        Utf8Data { data: [0; UTF8_SIZE], have: 0, size: 0, width: 0 }
    }

    /// Start a streaming decode: initialize `self` from a lead byte
    /// `ch`. Returns `More` if `ch` was a valid lead for a 2–4 byte
    /// sequence; `Error` otherwise.
    pub fn open(&mut self, ch: u8) -> Utf8State {
        // SAFETY: `self` is a valid mutable reference.
        unsafe { utf8_open(self as *mut Utf8Data, ch) }
    }

    /// Append a continuation byte to an in-progress streaming decode.
    /// Returns `More` if more bytes are expected, `Done` when the
    /// sequence completes, `Error` on invalid continuation.
    pub fn append(&mut self, ch: u8) -> Utf8State {
        // SAFETY: `self` is a valid mutable reference.
        unsafe { utf8_append(self as *mut Utf8Data, ch) }
    }

    /// Decode `self` (raw bytes) into a wide character. Returns `None`
    /// if the byte sequence is not a valid codepoint under the current
    /// locale.
    pub fn to_wchar(&self) -> Option<wchar_t> {
        let mut wc: wchar_t = 0;
        // SAFETY: `self` is a valid reference; `wc` is a stack local.
        let state = unsafe { utf8_towc(self as *const Utf8Data, &raw mut wc) };
        match state {
            Utf8State::Done => Some(wc),
            _ => None,
        }
    }

    /// Encode a wide character `wc` into a `Utf8Data`. Returns `None`
    /// if the codepoint cannot be represented under the current locale.
    pub fn from_wchar(wc: wchar_t) -> Option<Self> {
        let mut ud = Utf8Data::empty();
        // SAFETY: `ud` is on the stack, exclusive.
        let state = unsafe { utf8_fromwc(wc, &raw mut ud) };
        match state {
            Utf8State::Done => Some(ud),
            _ => None,
        }
    }
}

/// Safe variant of [`utf8_isvalid`] taking a `&CStr`.
pub fn utf8_is_valid_cstr(s: &std::ffi::CStr) -> bool {
    // SAFETY: `CStr::as_ptr` yields a valid NUL-terminated byte string.
    unsafe { utf8_isvalid(s.as_ptr().cast()) }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::zeroed;

    // ---------------------------------------------------------------
    // utf8_build_one
    // ---------------------------------------------------------------

    #[test]
    fn build_one_ascii_nul() {
        let uc = utf8_build_one(0);
        assert_eq!(utf8_get_size(uc), 1);
        assert_eq!(utf8_get_width(uc), 1);
        assert_eq!(uc & 0xff, 0);
    }

    #[test]
    fn build_one_ascii_a() {
        let uc = utf8_build_one(b'A');
        assert_eq!(utf8_get_size(uc), 1);
        assert_eq!(utf8_get_width(uc), 1);
        assert_eq!(uc & 0xff, b'A' as u32);
    }

    #[test]
    fn build_one_ascii_tilde() {
        let uc = utf8_build_one(0x7e);
        assert_eq!(utf8_get_size(uc), 1);
        assert_eq!(utf8_get_width(uc), 1);
        assert_eq!(uc & 0xff, 0x7e);
    }

    #[test]
    fn build_one_space() {
        let uc = utf8_build_one(b' ');
        assert_eq!(utf8_get_size(uc), 1);
        assert_eq!(utf8_get_width(uc), 1);
        assert_eq!(uc & 0xff, 0x20);
    }

    // ---------------------------------------------------------------
    // utf8_set_size / utf8_get_size round-trip
    // ---------------------------------------------------------------

    #[test]
    fn set_get_size_roundtrip() {
        for s in 0..=21u8 {
            let packed = utf8_set_size(s);
            assert_eq!(utf8_get_size(packed), s, "size round-trip failed for {s}");
        }
    }

    // ---------------------------------------------------------------
    // utf8_set_width / utf8_get_width round-trip
    // ---------------------------------------------------------------

    #[test]
    fn set_get_width_roundtrip() {
        for w in 0..=2u8 {
            let packed = utf8_set_width(w);
            assert_eq!(
                utf8_get_width(packed),
                w,
                "width round-trip failed for {w}"
            );
        }
    }

    // ---------------------------------------------------------------
    // utf8_from_data / utf8_to_data round-trip for small (<=3 byte)
    // ---------------------------------------------------------------

    #[test]
    fn from_data_to_data_roundtrip_ascii() {
        // 1-byte ASCII 'Z'
        let ud = Utf8Data::new([b'Z'], 1, 1, 1);
        let mut uc: Utf8Char = 0;
        let state = unsafe { utf8_from_data(&ud, &mut uc) };
        assert_eq!(state, Utf8State::Done);
        assert_eq!(utf8_get_size(uc), 1);
        assert_eq!(utf8_get_width(uc), 1);

        let back = utf8_to_data(uc);
        assert_eq!(back.size, 1);
        assert_eq!(back.width, 1);
        assert_eq!(&back.data[..1], &[b'Z']);
    }

    #[test]
    fn from_data_to_data_roundtrip_2byte() {
        // U+00E9 (e-acute) = 0xC3 0xA9
        let ud = Utf8Data::new([0xC3, 0xA9], 2, 2, 1);
        let mut uc: Utf8Char = 0;
        let state = unsafe { utf8_from_data(&ud, &mut uc) };
        assert_eq!(state, Utf8State::Done);
        assert_eq!(utf8_get_size(uc), 2);

        let back = utf8_to_data(uc);
        assert_eq!(back.size, 2);
        assert_eq!(&back.data[..2], &[0xC3, 0xA9]);
    }

    #[test]
    fn from_data_to_data_roundtrip_3byte() {
        // U+4E16 (CJK "world") = 0xE4 0xB8 0x96
        let ud = Utf8Data::new([0xE4, 0xB8, 0x96], 3, 3, 2);
        let mut uc: Utf8Char = 0;
        let state = unsafe { utf8_from_data(&ud, &mut uc) };
        assert_eq!(state, Utf8State::Done);
        assert_eq!(utf8_get_size(uc), 3);

        let back = utf8_to_data(uc);
        assert_eq!(back.size, 3);
        assert_eq!(back.width, 2);
        assert_eq!(&back.data[..3], &[0xE4, 0xB8, 0x96]);
    }

    // utf8_to_string tests live in tmux-rs::utf8 (the function stays
    // there because it's a Rust-idiomatic convenience over
    // sentinel-terminated Utf8Data arrays).

    // ---------------------------------------------------------------
    // utf8_in_table
    // ---------------------------------------------------------------

    #[test]
    fn in_table_found() {
        let table: &[wchar_t] = &[10, 20, 30, 40, 50];
        assert!(utf8_in_table(30, table));
    }

    #[test]
    fn in_table_not_found() {
        let table: &[wchar_t] = &[10, 20, 30, 40, 50];
        assert!(!utf8_in_table(25, table));
    }

    #[test]
    fn in_table_empty() {
        let table: &[wchar_t] = &[];
        assert!(!utf8_in_table(1, table));
    }

    #[test]
    fn in_force_wide_table() {
        // 0x1F600 should NOT be in the force-wide table
        assert!(!utf8_in_table(0x1F600, &UTF8_FORCE_WIDE));
        // 0x1F385 should be in the force-wide table (Santa Claus)
        assert!(utf8_in_table(0x1F385, &UTF8_FORCE_WIDE));
        // First and last entries
        assert!(utf8_in_table(0x0261D, &UTF8_FORCE_WIDE));
        assert!(utf8_in_table(0x1FAF8, &UTF8_FORCE_WIDE));
    }

    // ---------------------------------------------------------------
    // Utf8Data::new and initialized_slice
    // ---------------------------------------------------------------

    #[test]
    fn utf8_data_new_and_slice() {
        let ud = Utf8Data::new([b'x', b'y', b'z'], 3, 3, 1);
        assert_eq!(ud.initialized_slice(), b"xyz");
    }

    #[test]
    fn utf8_data_new_pads_with_zeroes() {
        let ud = Utf8Data::new([b'a'], 1, 1, 1);
        // Bytes beyond size should be zero
        assert_eq!(ud.data[1], 0);
        assert_eq!(ud.data[UTF8_SIZE - 1], 0);
    }

    // ---------------------------------------------------------------
    // utf8_open / utf8_append — encoding/decoding UTF-8 sequences
    // ---------------------------------------------------------------

    #[test]
    fn open_rejects_ascii() {
        // ASCII bytes should not start a multi-byte sequence
        unsafe {
            let mut ud: Utf8Data = zeroed();
            let state = utf8_open(&mut ud, b'A');
            assert_eq!(state, Utf8State::Error);
        }
    }

    #[test]
    fn open_rejects_continuation_byte() {
        unsafe {
            let mut ud: Utf8Data = zeroed();
            // 0x80 is a continuation byte, not a valid starter
            let state = utf8_open(&mut ud, 0x80);
            assert_eq!(state, Utf8State::Error);
        }
    }

    #[test]
    fn open_2byte_sequence() {
        unsafe {
            let mut ud: Utf8Data = zeroed();
            // U+00E9 -> 0xC3 0xA9
            let state = utf8_open(&mut ud, 0xC3);
            assert_eq!(state, Utf8State::More);
            assert_eq!(ud.size, 2);
            assert_eq!(ud.have, 1);
        }
    }

    #[test]
    fn open_3byte_sequence() {
        unsafe {
            let mut ud: Utf8Data = zeroed();
            // U+4E16 -> 0xE4 ...
            let state = utf8_open(&mut ud, 0xE4);
            assert_eq!(state, Utf8State::More);
            assert_eq!(ud.size, 3);
            assert_eq!(ud.have, 1);
        }
    }

    #[test]
    fn open_4byte_sequence() {
        unsafe {
            let mut ud: Utf8Data = zeroed();
            // U+1F600 -> 0xF0 ...
            let state = utf8_open(&mut ud, 0xF0);
            assert_eq!(state, Utf8State::More);
            assert_eq!(ud.size, 4);
            assert_eq!(ud.have, 1);
        }
    }

    #[test]
    fn open_invalid_high_byte() {
        unsafe {
            let mut ud: Utf8Data = zeroed();
            // 0xF5 and above are not valid UTF-8 starters
            let state = utf8_open(&mut ud, 0xF5);
            assert_eq!(state, Utf8State::Error);
        }
    }

    // ---------------------------------------------------------------
    // utf8_item_data
    // ---------------------------------------------------------------

    #[test]
    fn utf8_item_data_display() {
        let item = utf8_item_data::new(b"hello");
        assert_eq!(format!("{item}"), "hello");
    }

    #[test]
    fn utf8_item_data_initialized_slice() {
        let item = utf8_item_data::new(&[0xC3, 0xA9]);
        assert_eq!(item.initialized_slice(), &[0xC3, 0xA9]);
    }

    // ---------------------------------------------------------------
    // Build-one then to_data round-trip
    // ---------------------------------------------------------------

    #[test]
    fn build_one_then_to_data() {
        for ch in [0u8, b'A', b' ', b'~', 127] {
            let uc = utf8_build_one(ch);
            let ud = utf8_to_data(uc);
            assert_eq!(ud.size, 1, "size for byte {ch}");
            assert_eq!(ud.width, 1, "width for byte {ch}");
            assert_eq!(ud.data[0], ch, "data[0] for byte {ch}");
        }
    }

    // ---------------------------------------------------------------
    // Utf8Data equality via from_data encoding stability
    // ---------------------------------------------------------------

    #[test]
    fn from_data_encoding_is_deterministic() {
        let ud = Utf8Data::new([0xC3, 0xA9], 2, 2, 1);
        let mut uc1: Utf8Char = 0;
        let mut uc2: Utf8Char = 0;
        unsafe {
            utf8_from_data(&ud, &mut uc1);
            utf8_from_data(&ud, &mut uc2);
        }
        assert_eq!(uc1, uc2, "same input should produce same compact encoding");
    }
}
