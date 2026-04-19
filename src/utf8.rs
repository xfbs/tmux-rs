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

use crate::compat::vis;
use crate::libc::memcpy;
use crate::*;

// Re-export the core utf8 surface from `tmux-utf8`. Callers that `use
// crate::{Utf8Data, utf8_from_data, ...}` keep working unchanged.
pub(crate) use tmux_utf8::{
    UTF8_SIZE, Utf8Char, Utf8Data, Utf8State, utf8_append, utf8_copy, utf8_cstrhas,
    utf8_from_data, utf8_fromwc, utf8_in_table, utf8_isvalid, utf8_open, utf8_set, utf8_to_data,
    utf8_towc,
};

// Legacy alias — a handful of existing call sites say `utf8_state::UTF8_DONE`
// instead of `Utf8State::Done`. Keep the alias so the CamelCase crate
// transition doesn't force a mass rewrite. The `UTF8_*` variants also
// exist as associated consts on `Utf8State` itself (see `tmux-utf8`).
#[allow(non_camel_case_types)]
pub(crate) type utf8_state = Utf8State;
// Shorthand constants for the state values used locally in vis/cstring
// helpers. Path-through-enum (`Utf8State::UTF8_MORE`) also works.
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

pub unsafe fn utf8_strvis(
    mut dst: *mut u8,
    mut src: *const u8,
    len: usize,
    flag: vis_flags,
) -> i32 {
    unsafe {
        let mut ud: Utf8Data = zeroed();
        let start = dst;
        let end = src.add(len);
        let mut more: utf8_state;

        while src < end {
            more = utf8_open(&raw mut ud, *src);
            if more == utf8_state::Done.into() || more == UTF8_MORE {
                // placeholder
            }
            if more == UTF8_MORE {
                src = src.add(1);
                while src < end && more == UTF8_MORE {
                    more = utf8_append(&raw mut ud, *src);
                    src = src.add(1);
                }
                if more == UTF8_DONE {
                    for i in 0..ud.size {
                        *dst = ud.data[i as usize];
                        dst = dst.add(1);
                    }
                    continue;
                }
                src = src.sub(ud.have as usize);
            }
            if flag.intersects(vis_flags::VIS_DQ) && *src == b'$' && src < end.sub(1) {
                if (*src.add(1)).is_ascii_alphabetic() || *src.add(1) == b'_' || *src.add(1) == b'{'
                {
                    *dst = b'\\';
                    dst = dst.add(1);
                }
                *dst = b'$';
                dst = dst.add(1);
            } else if src < end.sub(1) {
                dst = vis(dst, *src as i32, flag, *src.add(1) as i32);
            } else if src < end {
                dst = vis(dst, *src as i32, flag, b'\0' as i32);
            }
            src = src.add(1);
        }
        *dst = b'\0';
        (dst.addr() - start.addr()) as i32
    }
}

pub unsafe fn utf8_strvis_(dst: &mut Vec<u8>, mut src: *const u8, len: usize, flag: vis_flags) {
    unsafe {
        let mut ud: Utf8Data = zeroed();
        let end = src.add(len);
        let mut more: utf8_state;

        while src < end {
            more = utf8_open(&raw mut ud, *src);
            if more == UTF8_MORE {
                src = src.add(1);
                while src < end && more == UTF8_MORE {
                    more = utf8_append(&raw mut ud, *src);
                    src = src.add(1);
                }
                if more == UTF8_DONE {
                    dst.extend(ud.initialized_slice());
                    continue;
                }
                src = src.sub(ud.have as usize);
            }
            if flag.intersects(vis_flags::VIS_DQ) && *src == b'$' && src < end.sub(1) {
                if (*src.add(1)).is_ascii_alphabetic() || *src.add(1) == b'_' || *src.add(1) == b'{'
                {
                    dst.push(b'\\');
                }
                dst.push(b'$');
            } else if src < end.sub(1) {
                vis__(dst, *src as i32, flag, *src.add(1) as i32);
            } else if src < end {
                vis__(dst, *src as i32, flag, b'\0' as i32);
            }
            src = src.add(1);
        }
    }
}

pub unsafe fn utf8_stravis(dst: *mut *mut u8, src: *const u8, flag: vis_flags) -> i32 {
    unsafe {
        let buf = xreallocarray(null_mut(), 4, strlen(src) + 1);
        let len = utf8_strvis(buf.as_ptr().cast(), src, strlen(src), flag);

        *dst = xrealloc(buf.as_ptr(), len as usize + 1).as_ptr().cast();
        len
    }
}

pub unsafe fn utf8_stravis_(src: *const u8, flag: vis_flags) -> Vec<u8> {
    unsafe {
        let mut buf: Vec<u8> = Vec::with_capacity(4 * (strlen(src) + 1));
        utf8_strvis_(&mut buf, src, strlen(src), flag);
        buf.shrink_to_fit();
        buf
    }
}

pub unsafe fn utf8_stravisx(
    dst: *mut *mut u8,
    src: *const u8,
    srclen: usize,
    flag: vis_flags,
) -> i32 {
    unsafe {
        let buf = xreallocarray(null_mut(), 4, srclen + 1);
        let len = utf8_strvis(buf.as_ptr().cast(), src, srclen, flag);

        *dst = xrealloc(buf.as_ptr(), len as usize + 1).as_ptr().cast();
        len
    }
}

pub unsafe fn utf8_sanitize(mut src: *const u8) -> *mut u8 {
    unsafe {
        let mut dst: *mut u8 = null_mut();
        let mut n: usize = 0;
        let mut ud: Utf8Data = zeroed();

        while *src != b'\0' {
            dst = xreallocarray_(dst, n + 1).as_ptr();
            let mut more = utf8_open(&raw mut ud, *src);
            if more == UTF8_MORE {
                while {
                    src = src.add(1);
                    *src != b'\0' && more == UTF8_MORE
                } {
                    more = utf8_append(&raw mut ud, *src);
                }
                if more == UTF8_DONE {
                    dst = xreallocarray_(dst, n + ud.width as usize).as_ptr();
                    for _ in 0..ud.width {
                        *dst.add(n) = b'_';
                        n += 1;
                    }
                    continue;
                }
                src = src.sub(ud.have as usize);
            }
            if *src > 0x1f && *src < 0x7f {
                *dst.add(n) = *src;
                n += 1;
            } else {
                *dst.add(n) = b'_';
                n += 1;
            }
            src = src.add(1);
        }
        dst = xreallocarray_(dst, n + 1).as_ptr();
        *dst.add(n) = b'\0';
        dst
    }
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

pub unsafe fn utf8_cstrwidth(mut s: *const u8) -> u32 {
    unsafe {
        let mut tmp: Utf8Data = zeroed();

        let mut width: u32 = 0;
        while *s != b'\0' {
            let mut more = utf8_open(&raw mut tmp, *s);
            if more == UTF8_MORE {
                while {
                    s = s.add(1);
                    *s != b'\0' && more == UTF8_MORE
                } {
                    more = utf8_append(&raw mut tmp, *s);
                }
                if more == UTF8_DONE {
                    width += tmp.width as u32;
                    continue;
                }
                s = s.sub(tmp.have as usize);
            }
            if *s > 0x1f && *s != 0x7f {
                width += 1;
            }
            s = s.add(1);
        }
        width
    }
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
