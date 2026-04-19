// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
//
// Permission u8, copy, modify, and distribute this software for any
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
//! UTF-8 helpers that retain tmux-rs-specific dependencies.
//!
//! The core codec (`utf8_from_data`, `utf8_to_data`, `utf8_set`,
//! `utf8_build_one`, `utf8_copy`, `utf8_cstrhas`, `utf8_width`,
//! `utf8_towc`, `utf8_fromwc`, `utf8_open`, `utf8_append`, `utf8_isvalid`),
//! the intern table, and the data types (`Utf8Data`, `Utf8Char`,
//! `UTF8_SIZE`) live in the `tmux-utf8` crate. They're re-exported here
//! so existing tmux-rs call sites continue to resolve via `crate::`.
//!
//! What stays in this file:
//!
//! - **vis-escape helpers** — `utf8_strvis`, `utf8_stravis`, `utf8_sanitize`:
//!   they depend on `compat::vis` (the tmux-rs internal escaper) and
//!   aren't part of the Unicode core.
//! - **C-string helpers** — `utf8_fromcstr`, `utf8_tocstr`, `utf8_cstrwidth`,
//!   `utf8_padcstr`, `utf8_rpadcstr`, `utf8_strlen`, `utf8_strwidth`,
//!   `utf8_to_string`: manipulate `*mut Utf8Data` arrays and C byte
//!   strings; still useful inside tmux-rs while the rest of the codebase
//!   migrates to safer idioms.

use crate::libc::memcpy;
use crate::*;

// Re-export the core utf8 surface from `tmux-utf8`. External call sites
// now use the safe method API (`Utf8Data::single`, `.encode()`, etc.);
// the freestanding underscore-prefix fns are kept only for use inside
// this file's own vis/cstring helpers.
pub(crate) use tmux_utf8::{
    UTF8_SIZE, Utf8Data, Utf8State, utf8_append, utf8_in_table, utf8_isvalid, utf8_open,
    utf8_set,
};

// Shorthand constants for the state values used inside the remaining
// sentinel-walking helpers below. Path-through-enum
// (`Utf8State::UTF8_MORE`) also works.
pub(crate) const UTF8_MORE: Utf8State = Utf8State::More;
pub(crate) const UTF8_DONE: Utf8State = Utf8State::Done;

// ---------------------------------------------------------------
// vis-escape helpers
//
// These format a byte string for safe display / clipboard / log use,
// escaping control chars according to vis(3) flags. They walk the
// input in UTF-8 steps (so multi-byte characters stay intact) and
// emit one vis-escaped output byte at a time via the local vis()
// wrapper.
// ---------------------------------------------------------------

// `utf8_strvis` (buf-writing variant) has been removed. It was the
// only caller writing into a pre-allocated C-style buffer and returning
// a length; the remaining vis helpers (`utf8_stravis` / `utf8_stravisx` /
// `utf8_stravis_`) cover every shape we actually use.

/// Walk `src` as UTF-8 and append each character to `dst`, escaping
/// non-printable bytes per the vis(3) flags. Complete multi-byte
/// sequences pass through unchanged; malformed bytes are individually
/// vis-escaped. The `VIS_DQ` flag additionally backslash-escapes `$`
/// before an identifier-start character so the output is safe inside
/// double-quoted shell strings.
pub fn utf8_strvis_(dst: &mut Vec<u8>, src: &[u8], flag: vis_flags) {
    let mut ud: Utf8Data = Utf8Data::empty();
    let mut i = 0;
    while i < src.len() {
        let byte = src[i];
        let mut more = ud.open(byte);
        if more == Utf8State::More {
            let mut j = i + 1;
            while j < src.len() && more == Utf8State::More {
                more = ud.append(src[j]);
                j += 1;
            }
            if more == Utf8State::Done {
                dst.extend(ud.initialized_slice());
                i = j;
                continue;
            }
            // Decode failed — fall through and vis-escape the lead byte.
        }
        let next = if i + 1 < src.len() { src[i + 1] as i32 } else { b'\0' as i32 };
        if flag.intersects(vis_flags::VIS_DQ) && byte == b'$' && i + 1 < src.len() {
            let nb = src[i + 1];
            if nb.is_ascii_alphabetic() || nb == b'_' || nb == b'{' {
                dst.push(b'\\');
            }
            dst.push(b'$');
        } else {
            vis__(dst, byte as i32, flag, next);
        }
        i += 1;
    }
}

/// Unsafe wrapper over [`utf8_stravis_`] for callers that hold a raw
/// NUL-terminated `*const u8`. Returns an owned `Vec<u8>`.
///
/// # Safety
/// `src` must be a valid NUL-terminated byte string.
pub unsafe fn utf8_stravis(src: *const u8, flag: vis_flags) -> Vec<u8> {
    let bytes = unsafe { CStr::from_ptr(src.cast()).to_bytes() };
    utf8_stravis_(bytes, flag)
}

/// Owning counterpart of [`utf8_strvis_`]: allocate a `Vec<u8>`, run
/// the vis-escape pass on `src`, and return the result.
pub fn utf8_stravis_(src: &[u8], flag: vis_flags) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::with_capacity(4 * (src.len() + 1));
    utf8_strvis_(&mut buf, src, flag);
    buf.shrink_to_fit();
    buf
}

/// Explicit-length variant of [`utf8_stravis`] — useful when `src`
/// isn't NUL-terminated (e.g. lives inside an `evbuffer`). Returns an
/// owned `Vec<u8>`.
///
/// # Safety
/// `src` must be valid for `srclen` bytes.
pub unsafe fn utf8_stravisx(src: *const u8, srclen: usize, flag: vis_flags) -> Vec<u8> {
    let bytes = unsafe { std::slice::from_raw_parts(src, srclen) };
    utf8_stravis_(bytes, flag)
}

/// Replace every non-printable byte (and every non-ASCII UTF-8
/// character) in `src` with ASCII underscores. Multi-byte characters
/// collapse to `width` underscores so column layout is preserved.
/// Printable ASCII (0x20..=0x7e) passes through unchanged.
pub fn utf8_sanitize_(src: &[u8]) -> Vec<u8> {
    let mut dst: Vec<u8> = Vec::with_capacity(src.len());
    let mut ud: Utf8Data = Utf8Data::empty();
    let mut i = 0;
    while i < src.len() {
        let byte = src[i];
        let mut more = ud.open(byte);
        if more == Utf8State::More {
            let mut j = i + 1;
            while j < src.len() && more == Utf8State::More {
                more = ud.append(src[j]);
                j += 1;
            }
            if more == Utf8State::Done {
                for _ in 0..ud.width {
                    dst.push(b'_');
                }
                i = j;
                continue;
            }
            // Decode failed — fall through and handle `src[i]` byte-at-a-time.
        }
        if byte > 0x1f && byte < 0x7f {
            dst.push(byte);
        } else {
            dst.push(b'_');
        }
        i += 1;
    }
    dst
}

/// C-string-pointer wrapper around [`utf8_sanitize_`].
///
/// # Safety
/// `src` must be a valid NUL-terminated byte string.
pub unsafe fn utf8_sanitize(src: *const u8) -> Vec<u8> {
    let bytes = unsafe { CStr::from_ptr(src.cast()).to_bytes() };
    utf8_sanitize_(bytes)
}

// ---------------------------------------------------------------
// Sentinel-terminated Utf8Data array helpers
// ---------------------------------------------------------------

pub unsafe fn utf8_strlen(s: *const Utf8Data) -> usize {
    let mut i = 0;

    unsafe {
        while (*s.add(i)).size != 0 {
            i += 1;
        }
    }

    i
}

pub unsafe fn utf8_strwidth(s: *const Utf8Data, n: isize) -> u32 {
    unsafe {
        let mut width: u32 = 0;

        let mut i: isize = 0;
        while (*s.add(i as usize)).size != 0 {
            if n != -1 && n == i {
                break;
            }
            width += (*s.add(i as usize)).width as u32;
            i += 1;
        }

        width
    }
}

pub unsafe fn utf8_fromcstr(mut src: *const u8) -> *mut Utf8Data {
    unsafe {
        let mut dst: *mut Utf8Data = null_mut();
        let mut n = 0;

        while *src != b'\0' {
            dst = xreallocarray_(dst, n + 1).as_ptr();
            let mut more = utf8_open(dst.add(n), *src);
            if more == UTF8_MORE {
                while {
                    src = src.add(1);
                    *src != b'\0' && more == UTF8_MORE
                } {
                    more = utf8_append(dst.add(n), *src);
                }
                if more == UTF8_DONE {
                    n += 1;
                    continue;
                }
                src = src.sub((*dst.add(n)).have as usize);
            }
            utf8_set(dst.add(n), *src);
            n += 1;
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        (*dst.add(n)).size = 0;

        dst
    }
}

pub unsafe fn utf8_tocstr(mut src: *const Utf8Data) -> *mut u8 {
    unsafe {
        let mut dst = null_mut::<u8>();
        let mut n: usize = 0;

        while (*src).size != 0 {
            dst = xreallocarray_(dst, n + (*src).size as usize).as_ptr();
            memcpy(
                dst.add(n).cast(),
                (*src).data.as_ptr().cast(),
                (*src).size as usize,
            );
            n += (*src).size as usize;
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        *dst.add(n) = b'\0';
        dst
    }
}

/// Copy a sentinel-terminated `Utf8Data` array into an owned Rust
/// `String`. Halts at the first zero-size entry (sentinel). Bytes
/// from `initialized_slice()` must already be valid UTF-8 — callers
/// that pass un-validated byte data will panic.
pub fn utf8_to_string(src: &[Utf8Data]) -> String {
    let mut dst: Vec<u8> = Vec::new();

    for src in src {
        if src.size == 0 {
            break;
        }
        dst.extend(src.initialized_slice());
    }

    String::from_utf8(dst).unwrap()
}

/// Compute the display width (in terminal columns) of a byte slice,
/// treating complete UTF-8 sequences as their intrinsic width and
/// counting every other printable ASCII byte as 1. Malformed sequences
/// and control bytes (0x00..=0x1f, 0x7f) contribute 0.
pub fn utf8_cstrwidth_(s: &[u8]) -> u32 {
    let mut tmp: Utf8Data = Utf8Data::empty();
    let mut width: u32 = 0;
    let mut i = 0;
    while i < s.len() {
        let byte = s[i];
        let mut more = tmp.open(byte);
        if more == Utf8State::More {
            let mut j = i + 1;
            while j < s.len() && more == Utf8State::More {
                more = tmp.append(s[j]);
                j += 1;
            }
            if more == Utf8State::Done {
                width += tmp.width as u32;
                i = j;
                continue;
            }
            // Decode failed — fall through and treat `s[i]` singly.
        }
        if byte > 0x1f && byte != 0x7f {
            width += 1;
        }
        i += 1;
    }
    width
}

/// Unsafe C-string wrapper over [`utf8_cstrwidth_`].
///
/// # Safety
/// `s` must be a valid NUL-terminated byte string.
pub unsafe fn utf8_cstrwidth(s: *const u8) -> u32 {
    let bytes = unsafe { CStr::from_ptr(s.cast()).to_bytes() };
    utf8_cstrwidth_(bytes)
}

pub unsafe fn utf8_padcstr(s: *const u8, width: u32) -> *mut u8 {
    unsafe {
        let n = utf8_cstrwidth(s);
        if n >= width {
            return xstrdup(s).as_ptr();
        }

        let mut slen = strlen(s);
        let out: *mut u8 = xmalloc(slen + 1 + (width - n) as usize).as_ptr().cast();
        memcpy(out.cast(), s.cast(), slen);
        let mut i = n;
        while i < width {
            *out.add(slen) = b' ';
            slen += 1;
            i += 1;
        }
        *out.add(slen) = b'\0';
        out
    }
}

pub unsafe fn utf8_rpadcstr(s: *const u8, width: u32) -> *mut u8 {
    unsafe {
        let n = utf8_cstrwidth(s);
        if n >= width {
            return xstrdup(s).as_ptr();
        }

        let slen = strlen(s);
        let out: *mut u8 = xmalloc(slen + 1 + (width - n) as usize).as_ptr().cast();
        let mut i = 0;
        while i < width {
            *out.add(i as usize) = b' ';
            i += 1;
        }
        memcpy(out.add(i as usize).cast(), s.cast(), slen);
        *out.add(i as usize + slen) = b'\0';
        out
    }
}

/// Fuzz-friendly wrapper: feeds arbitrary bytes through the UTF-8 decoder
/// state machine (utf8_open/utf8_append). Pure computation, no side effects.
#[cfg(fuzzing)]
pub fn fuzz_utf8_decode(data: &[u8]) {
    unsafe {
        let mut ud: Utf8Data = std::mem::zeroed();
        let mut in_sequence = false;

        for &byte in data {
            if !in_sequence {
                match utf8_open(&raw mut ud, byte) {
                    Utf8State::More => in_sequence = true,
                    _ => {}
                }
            } else {
                match utf8_append(&raw mut ud, byte) {
                    Utf8State::More => {}
                    _ => in_sequence = false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // utf8_to_string — sentinel-terminated Utf8Data arrays → String
    // ---------------------------------------------------------------

    #[test]
    fn to_string_empty() {
        let data: Vec<Utf8Data> = vec![];
        assert_eq!(utf8_to_string(&data), "");
    }

    #[test]
    fn to_string_ascii() {
        let h = Utf8Data::new([b'H'], 1, 1, 1);
        let i = Utf8Data::new([b'i'], 1, 1, 1);
        assert_eq!(utf8_to_string(&[h, i]), "Hi");
    }

    #[test]
    fn to_string_sentinel_stops() {
        let a = Utf8Data::new([b'A'], 1, 1, 1);
        let sentinel = Utf8Data {
            data: [0; UTF8_SIZE],
            have: 0,
            size: 0,
            width: 0,
        };
        let b = Utf8Data::new([b'B'], 1, 1, 1);
        assert_eq!(utf8_to_string(&[a, sentinel, b]), "A");
    }

    #[test]
    fn to_string_multibyte() {
        // U+00E9 = 0xC3 0xA9 -> "e" with acute
        let ud = Utf8Data::new([0xC3, 0xA9], 2, 2, 1);
        assert_eq!(utf8_to_string(&[ud]), "\u{00E9}");
    }
}
