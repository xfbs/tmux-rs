use core::ffi::{c_char, c_int};
use libc::{memcpy, regcomp, regex_t, regexec, regfree, regmatch_t, strlen};
use xmalloc::xrealloc_;

use super::*;

unsafe fn regsub_copy(buf: *mut *mut c_char, len: *mut isize, text: *const c_char, start: usize, end: usize) {
    let add: usize = end - start;
    unsafe {
        *buf = xrealloc_((*buf), (*len) as usize + add + 1).as_ptr();
        memcpy((*buf).add(*len as usize) as _, text.add(start) as _, add);
        (*len) += add as isize;
    }
}

pub unsafe fn regsub_expand(
    buf: *mut *mut c_char,
    len: *mut isize,
    with: *mut c_char,
    text: *const c_char,
    m: *mut regmatch_t,
    n: c_uint,
) {
    unsafe {
        let mut cp: *mut c_char = null_mut();
        let mut i: u32 = 0;

        cp = with;
        while *cp != b'\0' as c_char {
            if *cp == b'\\' as c_char {
                cp = cp.add(1);
                if *cp >= b'0' as _ && *cp <= b'9' as _ {
                    i = (*cp - b'0' as c_char) as u32;
                    if i < n && (*m.add(i as _)).rm_so != (*m.add(i as _)).rm_eo {
                        regsub_copy(
                            buf,
                            len,
                            text,
                            (*m.add(i as _)).rm_so as usize,
                            (*m.add(i as _)).rm_eo as usize,
                        );
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

#[unsafe(no_mangle)]
pub unsafe fn regsub(pattern: *const c_char, with: *mut c_char, text: *const c_char, flags: c_int) -> *mut c_char {
    unsafe {
        let mut r: regex_t = zeroed();
        let mut m: [regmatch_t; 10] = zeroed(); // TODO can use uninit
        let mut len: isize = 0;
        let mut empty = 0;
        let mut buf = null_mut();

        if *text == b'\0' as c_char {
            return xstrdup(c"".as_ptr()).cast().as_ptr();
        }
        if regcomp(&raw mut r, pattern, flags) != 0 {
            return null_mut();
        }

        let mut start: isize = 0;
        let mut last: isize = 0;
        let mut end: isize = strlen(text) as _;

        while start <= end {
            if regexec(&raw mut r, text.add(start as _) as _, m.len(), m.as_mut_ptr(), 0) != 0 {
                regsub_copy(&raw mut buf, &raw mut len, text, start as usize, end as usize);
                break;
            }

            /*
             * Append any text not part of this match (from the end of the
             * last match).
             */
            regsub_copy(
                &raw mut buf,
                &raw mut len,
                text,
                last as usize,
                (m[0].rm_so as isize + start) as usize,
            );

            /*
             * If the last match was empty and this one isn't (it is either
             * later or has matched text), expand this match. If it is
             * empty, move on one character and try again from there.
             */
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
            if (*pattern == b'^' as _) {
                regsub_copy(&raw mut buf, &raw mut len, text, start as usize, end as usize);
                break;
            }
        }
        *buf.offset(len) = b'\0' as _;

        regfree(&raw mut r);
        buf
    }
}
