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

use core::ffi::{CStr, c_int};
use core::mem;

use libc::{regcomp, regex_t, regexec, regfree, regmatch_t};

/// Compiles `pattern` with `flags`, replaces all non-overlapping matches in
/// `text` with the expanded `with` (backreferences `\0`–`\9`), and returns the
/// result as a `Vec<u8>`.
///
/// Returns `None` if the pattern fails to compile. Returns an empty `Vec` if
/// `text` is empty. Anchored patterns (`^...`) only replace the first match.
///
/// The output is not NUL-terminated; callers that need a C string should
/// append a terminator themselves.
pub fn regsub(
    pattern: &CStr,
    with: &CStr,
    text: &CStr,
    flags: c_int,
) -> Option<Vec<u8>> {
    let text_bytes = text.to_bytes();
    if text_bytes.is_empty() {
        return Some(Vec::new());
    }

    // SAFETY: we initialise `r` via `regcomp` before reading any field, and
    // call `regfree` unconditionally before returning.
    let mut r: regex_t = unsafe { mem::zeroed() };
    let mut m: [regmatch_t; 10] = unsafe { mem::zeroed() };

    // SAFETY: `pattern` is a NUL-terminated CStr; `regcomp` reads it up to the
    // NUL and writes a compiled automaton into `r`.
    if unsafe { regcomp(&raw mut r, pattern.as_ptr(), flags) } != 0 {
        return None;
    }

    let with_bytes = with.to_bytes();
    let anchored = pattern.to_bytes().first() == Some(&b'^');

    let mut buf: Vec<u8> = Vec::new();
    let mut start: usize = 0;
    let mut last: usize = 0;
    let mut empty = false;
    let end = text_bytes.len();

    while start <= end {
        // SAFETY: `text` is a NUL-terminated CStr, and `start <= end == strlen(text)`.
        // `text.as_ptr().add(start)` therefore points at a NUL-terminated
        // substring that `regexec` can read.
        let rc = unsafe {
            regexec(
                &raw mut r,
                text.as_ptr().add(start),
                m.len(),
                m.as_mut_ptr(),
                0,
            )
        };
        if rc != 0 {
            // No more matches — copy remainder.
            buf.extend_from_slice(&text_bytes[start..end]);
            break;
        }

        let match_start = m[0].rm_so as usize;
        let match_end = m[0].rm_eo as usize;

        // Append any text not part of this match (from the end of the last match).
        buf.extend_from_slice(&text_bytes[last..start + match_start]);

        // If the last match was empty and this one isn't (it is either later or
        // has matched text), expand this match. If it is empty, move on one
        // character and try again from there.
        if empty || start + match_start != last || m[0].rm_so != m[0].rm_eo {
            expand(&mut buf, with_bytes, &text_bytes[start..], &m);
            last = start + match_end;
            start += match_end;
            empty = false;
        } else {
            last = start + match_end;
            start += match_end + 1;
            empty = true;
        }

        // Stop now if anchored to start.
        if anchored {
            buf.extend_from_slice(&text_bytes[start..end]);
            break;
        }
    }

    // SAFETY: `r` was successfully initialised by `regcomp` above.
    unsafe { regfree(&raw mut r) };

    Some(buf)
}

/// Expands backreferences (`\0`–`\9`) in the replacement string `with`,
/// substituting matched groups from `m` (offsets relative to `text`), and
/// appends the result to `buf`. A literal backslash at the end of `with`
/// is dropped (matches libc sed behaviour). `\X` where `X` is not a digit
/// or is a non-matching group number expands to `X`.
fn expand(buf: &mut Vec<u8>, with: &[u8], text: &[u8], m: &[regmatch_t; 10]) {
    let mut i = 0;
    while i < with.len() {
        let c = with[i];
        if c == b'\\' {
            i += 1;
            if i >= with.len() {
                // Trailing backslash — drop it.
                break;
            }
            let d = with[i];
            if d.is_ascii_digit() {
                let idx = (d - b'0') as usize;
                if idx < m.len() && m[idx].rm_so != m[idx].rm_eo {
                    let so = m[idx].rm_so as usize;
                    let eo = m[idx].rm_eo as usize;
                    buf.extend_from_slice(&text[so..eo]);
                    i += 1;
                    continue;
                }
            }
            // Not a backreference — push the char following the backslash.
            buf.push(d);
            i += 1;
            continue;
        }
        buf.push(c);
        i += 1;
    }
}

/// Fuzz-friendly wrapper: runs regex substitution with three byte slices
/// (pattern, replacement, text). Pure computation, no side effects.
#[cfg(fuzzing)]
pub fn fuzz_regsub(pattern: &[u8], with: &[u8], text: &[u8]) {
    if pattern.contains(&0) || with.contains(&0) || text.contains(&0) {
        return;
    }
    if pattern.is_empty() {
        return;
    }
    let p = std::ffi::CString::new(pattern).unwrap();
    let w = std::ffi::CString::new(with).unwrap();
    let t = std::ffi::CString::new(text).unwrap();
    let _ = regsub(&p, &w, &t, libc::REG_EXTENDED);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn sub(pattern: &str, with: &str, text: &str) -> Option<String> {
        sub_flags(pattern, with, text, libc::REG_EXTENDED)
    }

    fn sub_flags(pattern: &str, with: &str, text: &str, flags: c_int) -> Option<String> {
        let p = CString::new(pattern).unwrap();
        let w = CString::new(with).unwrap();
        let t = CString::new(text).unwrap();
        regsub(&p, &w, &t, flags).map(|v| String::from_utf8(v).unwrap())
    }

    // -----------------------------------------------------------------
    // Basic substitution
    // -----------------------------------------------------------------

    #[test]
    fn simple_replacement() {
        assert_eq!(sub("foo", "bar", "foo"), Some("bar".into()));
    }

    #[test]
    fn no_match_returns_original() {
        assert_eq!(sub("xyz", "bar", "hello"), Some("hello".into()));
    }

    #[test]
    fn empty_text_returns_empty() {
        assert_eq!(sub("foo", "bar", ""), Some("".into()));
    }

    #[test]
    fn invalid_pattern_returns_none() {
        // Unmatched bracket is invalid in extended regex.
        assert_eq!(sub("[", "bar", "hello"), None);
    }

    // -----------------------------------------------------------------
    // Global replacement (all matches)
    // -----------------------------------------------------------------

    #[test]
    fn replaces_all_matches() {
        assert_eq!(sub("o", "0", "foobar"), Some("f00bar".into()));
    }

    #[test]
    fn replaces_all_non_overlapping() {
        assert_eq!(sub("ab", "X", "ababab"), Some("XXX".into()));
    }

    // -----------------------------------------------------------------
    // Anchored patterns (^)
    // -----------------------------------------------------------------

    #[test]
    fn anchored_replaces_first_only() {
        assert_eq!(sub("^foo", "bar", "foofoo"), Some("barfoo".into()));
    }

    #[test]
    fn anchored_no_match() {
        assert_eq!(sub("^bar", "X", "foobar"), Some("foobar".into()));
    }

    // -----------------------------------------------------------------
    // Backreferences
    // -----------------------------------------------------------------

    #[test]
    fn backreference_whole_match() {
        assert_eq!(sub("(foo)", "[\\0]", "foo"), Some("[foo]".into()));
    }

    #[test]
    fn backreference_group() {
        assert_eq!(
            sub("(hello) (world)", "\\2 \\1", "hello world"),
            Some("world hello".into())
        );
    }

    #[test]
    fn backreference_nonexistent_group() {
        // \9 doesn't match a group — the digit is kept as a literal.
        assert_eq!(sub("(foo)", "\\9", "foo"), Some("9".into()));
    }

    // -----------------------------------------------------------------
    // Extended regex features
    // -----------------------------------------------------------------

    #[test]
    fn character_class() {
        assert_eq!(sub("[0-9]+", "N", "abc123def456"), Some("abcNdefN".into()));
    }

    #[test]
    fn alternation() {
        assert_eq!(
            sub("cat|dog", "pet", "I have a cat and a dog"),
            Some("I have a pet and a pet".into())
        );
    }

    #[test]
    fn dot_star_greedy() {
        assert_eq!(sub("a.*b", "X", "aXXbYYb"), Some("X".into()));
    }

    // -----------------------------------------------------------------
    // Case-insensitive matching
    // -----------------------------------------------------------------

    #[test]
    fn case_insensitive() {
        assert_eq!(
            sub_flags("foo", "bar", "FOO", libc::REG_EXTENDED | libc::REG_ICASE),
            Some("bar".into())
        );
    }

    // -----------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------

    #[test]
    fn replacement_longer_than_match() {
        assert_eq!(sub("a", "XYZ", "aaa"), Some("XYZXYZXYZ".into()));
    }

    #[test]
    fn replacement_empty_deletes_matches() {
        assert_eq!(sub("[0-9]", "", "a1b2c3"), Some("abc".into()));
    }

    #[test]
    fn literal_backslash_in_replacement() {
        // \\n in the Rust string is a single backslash followed by `n`, not a
        // backreference, so it should be kept as `n`.
        assert_eq!(sub("x", "\\n", "x"), Some("n".into()));
    }

    #[test]
    fn match_at_end_of_string() {
        assert_eq!(sub("bar$", "X", "foobar"), Some("fooX".into()));
    }

    #[test]
    fn full_string_match() {
        assert_eq!(sub("^.*$", "replaced", "anything"), Some("replaced".into()));
    }
}
