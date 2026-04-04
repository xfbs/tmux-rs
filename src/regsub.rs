// Copyright (c) 2019 Nicholas Marriott <nicholas.marriott@gmail.com>
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
//! Regex-based string substitution (like `sed s/pattern/replacement/`).
//!
//! Used by tmux's format system (`#{s/pat/repl/:value}`) and window-copy search.
//! Built on POSIX `regcomp`/`regexec` — supports extended regex syntax and
//! backreferences `\0`–`\9` in the replacement string.
//!
//! The main entry point is [`regsub`], which compiles a pattern, finds all
//! non-overlapping matches in the input text, and replaces each with the
//! expanded replacement string. Anchored patterns (`^...`) only replace the
//! first match.

use core::ffi::c_int;

use xmalloc::xrealloc_;

use crate::libc::{memcpy, regcomp, regex_t, regexec, regfree, regmatch_t, strlen};
use crate::*;

/// Appends a slice of `text[start..end]` to the growing output buffer.
unsafe fn regsub_copy(
    buf: *mut *mut u8,
    len: *mut isize,
    text: *const u8,
    start: usize,
    end: usize,
) {
    let add: usize = end - start;
    unsafe {
        *buf = xrealloc_(*buf, (*len) as usize + add + 1).as_ptr();
        memcpy((*buf).add(*len as usize) as _, text.add(start) as _, add);
        (*len) += add as isize;
    }
}

/// Expands backreferences (`\0`–`\9`) in the replacement string `with`,
/// substituting matched groups from `m`, and appends the result to `buf`.
pub unsafe fn regsub_expand(
    buf: *mut *mut u8,
    len: *mut isize,
    with: *const u8,
    text: *const u8,
    m: *mut regmatch_t,
    n: c_uint,
) {
    unsafe {
        let mut cp = with;
        while *cp != b'\0' {
            if *cp == b'\\' {
                cp = cp.add(1);
                // Trailing backslash at end of replacement string.
                if *cp == b'\0' {
                    break;
                }
                if *cp >= b'0' as _ && *cp <= b'9' as _ {
                    let i = (*cp - b'0') as u32;
                    if i < n && (*m.add(i as _)).rm_so != (*m.add(i as _)).rm_eo {
                        regsub_copy(
                            buf,
                            len,
                            text,
                            (*m.add(i as _)).rm_so as usize,
                            (*m.add(i as _)).rm_eo as usize,
                        );
                        // C used for(...; cp++) so continue still incremented;
                        // Rust while loop needs explicit advance past the digit.
                        cp = cp.add(1);
                        continue;
                    }
                }
            }
            *buf = xrealloc_(*buf, (*len) as usize + 2).as_ptr();
            *(*buf).add((*len) as usize) = *cp;
            (*len) += 1;

            cp = cp.add(1);
        }
    }
}

/// Performs regex substitution on `text`: compiles `pattern` with `flags`,
/// replaces all non-overlapping matches with `with` (expanding backreferences),
/// and returns a newly allocated result string.
///
/// Returns null if the pattern fails to compile. Returns an empty string
/// if the input text is empty. Anchored patterns (`^...`) only replace the
/// first match.
pub unsafe fn regsub(
    pattern: *const u8,
    with: *const u8,
    text: *const u8,
    flags: c_int,
) -> *mut u8 {
    unsafe {
        let mut r: regex_t = zeroed();
        let mut m: [regmatch_t; 10] = zeroed(); // TODO can use uninit
        let mut len: isize = 0;
        let mut empty = 0;
        let mut buf = null_mut();

        if *text == b'\0' {
            return xstrdup(c!("")).cast().as_ptr();
        }
        if regcomp(&raw mut r, pattern, flags) != 0 {
            return null_mut();
        }

        let mut start: isize = 0;
        let mut last: isize = 0;
        let end: isize = strlen(text) as _;

        while start <= end {
            if regexec(
                &raw mut r,
                text.add(start as _) as _,
                m.len(),
                m.as_mut_ptr(),
                0,
            ) != 0
            {
                regsub_copy(
                    &raw mut buf,
                    &raw mut len,
                    text,
                    start as usize,
                    end as usize,
                );
                break;
            }

            // Append any text not part of this match (from the end of the
            // last match).
            regsub_copy(
                &raw mut buf,
                &raw mut len,
                text,
                last as usize,
                (m[0].rm_so as isize + start) as usize,
            );

            // If the last match was empty and this one isn't (it is either
            // later or has matched text), expand this match. If it is
            // empty, move on one character and try again from there.
            if empty != 0 || start + m[0].rm_so as isize != last || m[0].rm_so != m[0].rm_eo {
                regsub_expand(
                    &raw mut buf,
                    &raw mut len,
                    with,
                    text.offset(start),
                    m.as_mut_ptr(),
                    m.len() as u32,
                );

                last = start + m[0].rm_eo as isize;
                start += m[0].rm_eo as isize;
                empty = 0;
            } else {
                last = start + m[0].rm_eo as isize;
                start += (m[0].rm_eo + 1) as isize;
                empty = 1;
            }

            // Stop now if anchored to start.
            if *pattern == b'^' {
                regsub_copy(
                    &raw mut buf,
                    &raw mut len,
                    text,
                    start as usize,
                    end as usize,
                );
                break;
            }
        }
        *buf.offset(len) = b'\0' as _;

        regfree(&raw mut r);
        buf
    }
}

/// Fuzz-friendly wrapper: runs regex substitution with three NUL-terminated
/// byte slices (pattern, replacement, text). Pure computation, no side effects.
#[cfg(fuzzing)]
pub fn fuzz_regsub(pattern: &[u8], with: &[u8], text: &[u8]) {
    // All three must be NUL-terminated C strings with no interior NULs.
    if pattern.contains(&0) || with.contains(&0) || text.contains(&0) {
        return;
    }
    if pattern.is_empty() {
        return;
    }

    let mut pat = Vec::with_capacity(pattern.len() + 1);
    pat.extend_from_slice(pattern);
    pat.push(0);

    let mut w = Vec::with_capacity(with.len() + 1);
    w.extend_from_slice(with);
    w.push(0);

    let mut t = Vec::with_capacity(text.len() + 1);
    t.extend_from_slice(text);
    t.push(0);

    unsafe {
        let result = regsub(pat.as_ptr(), w.as_ptr(), t.as_ptr(), crate::libc::REG_EXTENDED);
        if !result.is_null() {
            crate::free_(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: call regsub with Rust strings, return the result as a String.
    /// Uses REG_EXTENDED by default.
    unsafe fn sub(pattern: &str, with: &str, text: &str) -> Option<String> {
        unsafe { sub_flags(pattern, with, text, libc::REG_EXTENDED) }
    }

    unsafe fn sub_flags(
        pattern: &str,
        with: &str,
        text: &str,
        flags: c_int,
    ) -> Option<String> {
        unsafe {
            let p = CString::new(pattern).unwrap();
            let w = CString::new(with).unwrap();
            let t = CString::new(text).unwrap();
            let result = regsub(
                p.as_ptr().cast(),
                w.as_ptr().cast(),
                t.as_ptr().cast(),
                flags,
            );
            if result.is_null() {
                return None;
            }
            let s = CStr::from_ptr(result.cast()).to_str().unwrap().to_string();
            free_(result);
            Some(s)
        }
    }

    // ---------------------------------------------------------------
    // Basic substitution
    // ---------------------------------------------------------------

    #[test]
    fn simple_replacement() {
        unsafe {
            assert_eq!(sub("foo", "bar", "foo"), Some("bar".into()));
        }
    }

    #[test]
    fn no_match_returns_original() {
        unsafe {
            assert_eq!(sub("xyz", "bar", "hello"), Some("hello".into()));
        }
    }

    #[test]
    fn empty_text_returns_empty() {
        unsafe {
            assert_eq!(sub("foo", "bar", ""), Some("".into()));
        }
    }

    #[test]
    fn invalid_pattern_returns_none() {
        unsafe {
            // Unmatched bracket is invalid in extended regex.
            assert_eq!(sub("[", "bar", "hello"), None);
        }
    }

    // ---------------------------------------------------------------
    // Global replacement (all matches)
    // ---------------------------------------------------------------

    #[test]
    fn replaces_all_matches() {
        unsafe {
            assert_eq!(sub("o", "0", "foobar"), Some("f00bar".into()));
        }
    }

    #[test]
    fn replaces_all_non_overlapping() {
        unsafe {
            assert_eq!(sub("ab", "X", "ababab"), Some("XXX".into()));
        }
    }

    // ---------------------------------------------------------------
    // Anchored patterns (^)
    // ---------------------------------------------------------------

    #[test]
    fn anchored_replaces_first_only() {
        unsafe {
            assert_eq!(sub("^foo", "bar", "foofoo"), Some("barfoo".into()));
        }
    }

    #[test]
    fn anchored_no_match() {
        unsafe {
            assert_eq!(sub("^bar", "X", "foobar"), Some("foobar".into()));
        }
    }

    // ---------------------------------------------------------------
    // Backreferences
    // ---------------------------------------------------------------

    #[test]
    fn backreference_whole_match() {
        unsafe {
            assert_eq!(sub("(foo)", "[\\0]", "foo"), Some("[foo]".into()));
        }
    }

    #[test]
    fn backreference_group() {
        unsafe {
            assert_eq!(
                sub("(hello) (world)", "\\2 \\1", "hello world"),
                Some("world hello".into())
            );
        }
    }

    #[test]
    fn backreference_nonexistent_group() {
        unsafe {
            // \9 doesn't match a group — the digit is kept as a literal.
            assert_eq!(sub("(foo)", "\\9", "foo"), Some("9".into()));
        }
    }

    // ---------------------------------------------------------------
    // Extended regex features
    // ---------------------------------------------------------------

    #[test]
    fn character_class() {
        unsafe {
            assert_eq!(sub("[0-9]+", "N", "abc123def456"), Some("abcNdefN".into()));
        }
    }

    #[test]
    fn alternation() {
        unsafe {
            assert_eq!(sub("cat|dog", "pet", "I have a cat and a dog"), Some("I have a pet and a pet".into()));
        }
    }

    #[test]
    fn dot_star_greedy() {
        unsafe {
            assert_eq!(sub("a.*b", "X", "aXXbYYb"), Some("X".into()));
        }
    }

    // ---------------------------------------------------------------
    // Case-insensitive matching
    // ---------------------------------------------------------------

    #[test]
    fn case_insensitive() {
        unsafe {
            assert_eq!(
                sub_flags("foo", "bar", "FOO", libc::REG_EXTENDED | libc::REG_ICASE),
                Some("bar".into())
            );
        }
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn replacement_longer_than_match() {
        unsafe {
            assert_eq!(sub("a", "XYZ", "aaa"), Some("XYZXYZXYZ".into()));
        }
    }

    #[test]
    fn replacement_empty_deletes_matches() {
        unsafe {
            assert_eq!(sub("[0-9]", "", "a1b2c3"), Some("abc".into()));
        }
    }

    #[test]
    fn literal_backslash_in_replacement() {
        unsafe {
            // \\ in C string is a single backslash followed by a non-digit,
            // so it should be kept as-is.
            assert_eq!(sub("x", "\\n", "x"), Some("n".into()));
        }
    }

    #[test]
    fn match_at_end_of_string() {
        unsafe {
            assert_eq!(sub("bar$", "X", "foobar"), Some("fooX".into()));
        }
    }

    #[test]
    fn full_string_match() {
        unsafe {
            assert_eq!(sub("^.*$", "replaced", "anything"), Some("replaced".into()));
        }
    }
}
