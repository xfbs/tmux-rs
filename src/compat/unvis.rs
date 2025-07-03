// Copyright (c) 1989, 1993
// The Regents of the University of California.  All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
// 1. Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
// 2. Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in the
//    documentation and/or other materials provided with the distribution.
// 3. Neither the name of the University nor the names of its contributors
//    may be used to endorse or promote products derived from this software
//    without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
// ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
// OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
// HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
// LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
// OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
// SUCH DAMAGE.

// generated using c2rust on unvis.c
// TODO refactor

#[unsafe(no_mangle)]
pub unsafe fn unvis(
    mut cp: *mut libc::c_char,
    mut c: libc::c_char,
    mut astate: *mut libc::c_int,
    mut flag: libc::c_int,
) -> libc::c_int {
    unsafe {
        if flag & 1 != 0 {
            if *astate == 5 || *astate == 6 {
                *astate = 0;
                return 1;
            }
            return if *astate == 0 { 3 } else { -1 };
        }
        match *astate {
            0 => {
                *cp = 0;
                if c == b'\\' as i8 {
                    *astate = 1;
                    return 0;
                }
                *cp = c;
                1
            }
            1 => {
                match c as libc::c_int {
                    92 => {
                        *cp = c;
                        *astate = 0;
                        return 1;
                    }
                    48..=55 => {
                        *cp = (c as libc::c_int - '0' as i32) as libc::c_char;
                        *astate = 5;
                        return 0;
                    }
                    77 => {
                        *cp = 0o200i32 as i8;
                        *astate = 2;
                        return 0;
                    }
                    94 => {
                        *astate = 4;
                        return 0;
                    }
                    110 => {
                        *cp = b'\n' as i8;
                        *astate = 0;
                        return 1;
                    }
                    114 => {
                        *cp = b'\r' as i8;
                        *astate = 0;
                        return 1;
                    }
                    98 => {
                        *cp = '\u{8}' as i8;
                        *astate = 0;
                        return 1;
                    }
                    97 => {
                        *cp = '\u{7}' as i8;
                        *astate = 0;
                        return 1;
                    }
                    118 => {
                        *cp = '\u{b}' as i8;
                        *astate = 0;
                        return 1;
                    }
                    116 => {
                        *cp = '\t' as i8;
                        *astate = 0;
                        return 1;
                    }
                    102 => {
                        *cp = '\u{c}' as i8;
                        *astate = 0;
                        return 1;
                    }
                    115 => {
                        *cp = ' ' as i8;
                        *astate = 0;
                        return 1;
                    }
                    69 => {
                        *cp = '\u{1b}' as i8;
                        *astate = 0;
                        return 1;
                    }
                    10 => {
                        *astate = 0;
                        return 3;
                    }
                    36 => {
                        *astate = 0;
                        return 3;
                    }
                    _ => {}
                }
                *astate = 0;
                -1
            }
            2 => {
                if c as libc::c_int == '-' as i32 {
                    *astate = 3;
                } else if c as libc::c_int == '^' as i32 {
                    *astate = 4;
                } else {
                    *astate = 0;
                    return -1;
                }
                0
            }
            3 => {
                *astate = 0;
                *cp = (*cp as libc::c_int | c as libc::c_int) as libc::c_char;
                1
            }
            4 => {
                if c as libc::c_int == '?' as i32 {
                    *cp = (*cp as libc::c_int | 0o177 as libc::c_int) as libc::c_char;
                } else {
                    *cp = (*cp as libc::c_int | c as libc::c_int & 0o37 as libc::c_int)
                        as libc::c_char;
                }
                *astate = 0;
                1
            }
            5 => {
                if c as u8 as libc::c_int >= '0' as i32 && c as u8 as libc::c_int <= '7' as i32 {
                    *cp = (((*cp as libc::c_int) << 3 as libc::c_int)
                        + (c as libc::c_int - '0' as i32))
                        as libc::c_char;
                    *astate = 6;
                    return 0;
                }
                *astate = 0;
                2
            }
            6 => {
                *astate = 0 as libc::c_int;
                if c as u8 as libc::c_int >= '0' as i32 && c as u8 as libc::c_int <= '7' as i32 {
                    *cp = (((*cp as libc::c_int) << 3 as libc::c_int)
                        + (c as libc::c_int - '0' as i32))
                        as libc::c_char;
                    return 1;
                }
                2
            }
            _ => {
                *astate = 0;
                -1
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe fn strunvis(mut dst: *mut libc::c_char, mut src: *const libc::c_char) -> i32 {
    unsafe {
        let mut c: libc::c_char = 0;
        let mut start: *mut libc::c_char = dst;
        let mut state: libc::c_int = 0 as libc::c_int;
        loop {
            let fresh0 = src;
            src = src.offset(1);
            c = *fresh0;
            if c == 0 {
                break;
            }
            loop {
                match unvis(dst, c, &mut state, 0 as libc::c_int) {
                    1 => {
                        dst = dst.offset(1);
                        dst;
                        break;
                    }
                    2 => {
                        dst = dst.offset(1);
                        dst;
                    }
                    0 | 3 => {
                        break;
                    }
                    _ => {
                        *dst = '\0' as i32 as libc::c_char;
                        return -(1 as libc::c_int);
                    }
                }
            }
        }
        if unvis(dst, c, &mut state, 1 as libc::c_int) == 1 as libc::c_int {
            dst = dst.offset(1);
            dst;
        }
        *dst = '\0' as i32 as libc::c_char;
        dst.offset_from(start) as i32
    }
}
#[unsafe(no_mangle)]
pub unsafe fn strnunvis(
    mut dst: *mut libc::c_char,
    mut src: *const libc::c_char,
    mut sz: usize,
) -> isize {
    unsafe {
        let mut c: libc::c_char = 0;
        let mut p: libc::c_char = 0;
        let mut start: *mut libc::c_char = dst;
        let mut end: *mut libc::c_char = dst.add(sz).offset(-1);
        let mut state: libc::c_int = 0 as libc::c_int;
        if sz > 0 {
            *end = '\0' as i32 as libc::c_char;
        }
        loop {
            let fresh1 = src;
            src = src.offset(1);
            c = *fresh1;
            if c == 0 {
                break;
            }
            loop {
                match unvis(&mut p, c, &mut state, 0 as libc::c_int) {
                    1 => {
                        if dst < end {
                            *dst = p;
                        }
                        dst = dst.offset(1);
                        dst;
                        break;
                    }
                    2 => {
                        if dst < end {
                            *dst = p;
                        }
                        dst = dst.offset(1);
                        dst;
                    }
                    0 | 3 => {
                        break;
                    }
                    _ => {
                        if dst <= end {
                            *dst = '\0' as i32 as libc::c_char;
                        }
                        return -(1 as libc::c_int) as isize;
                    }
                }
            }
        }
        if unvis(&mut p, c, &mut state, 1 as libc::c_int) == 1 as libc::c_int {
            if dst < end {
                *dst = p;
            }
            dst = dst.offset(1);
            dst;
        }
        if dst <= end {
            *dst = '\0' as i32 as libc::c_char;
        }
        dst.offset_from(start)
    }
}
