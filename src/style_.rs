// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
// Copyright (c) 2014 Tiago Cunha <tcunha@users.sourceforge.net>
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

use super::*;

use libc::{snprintf, strcasecmp, strchr, strcspn, strncasecmp, strspn};

use crate::compat::strlcpy;

// #define STYLE_ATTR_MASK (~0)

pub static mut style_default: style = style {
    gc: grid_cell::new(
        utf8_data::new([b' '], 0, 1, 1),
        grid_attr::empty(),
        grid_flag::empty(),
        8,
        8,
        0,
        0,
    ),
    ignore: 0,

    fill: 8,
    align: style_align::STYLE_ALIGN_DEFAULT,
    list: style_list::STYLE_LIST_OFF,

    range_type: style_range_type::STYLE_RANGE_NONE,
    range_argument: 0,
    range_string: [0; 16], // ""

    default_type: style_default_type::STYLE_DEFAULT_BASE,
};

pub unsafe extern "C" fn style_set_range_string(sy: *mut style, s: *const c_char) {
    unsafe {
        strlcpy(&raw mut (*sy).range_string as _, s, 16); // TODO use better sizeof
    }
}

pub unsafe extern "C" fn style_parse(
    sy: *mut style,
    base: *const grid_cell,
    mut in_: *const c_char,
) -> i32 {
    unsafe {
        let delimiters = c" ,\n".as_ptr();
        let mut errstr: *mut c_char = null_mut();

        type tmp_type = [c_char; 256];
        let mut tmp_bak: tmp_type = [0; 256];
        let tmp = tmp_bak.as_mut_ptr();

        let mut found: *mut c_char = null_mut();
        let mut end: usize = 0;
        let mut n: u32 = 0;

        if *in_ == b'\0' as _ {
            return 0;
        }

        let mut saved = MaybeUninit::<style>::uninit();
        style_copy(saved.as_mut_ptr(), sy);
        let saved = unsafe { saved.assume_init() };

        'error: {
            log_debug!("{}: {}", "style_parse", _s(in_));
            loop {
                while *in_ != b'\0' as _ && !strchr(delimiters, *in_ as _).is_null() {
                    in_ = in_.add(1);
                }
                if *in_ == b'\0' as _ {
                    break;
                }

                end = strcspn(in_, delimiters);
                if end > size_of::<tmp_type>() - 1 {
                    break 'error;
                }
                memcpy_(tmp, in_, end);
                *tmp.add(end) = b'\0' as _;

                log_debug!("{}: {}", "style_parse", _s(tmp));
                if strcasecmp(tmp, c"default".as_ptr()) == 0 {
                    (*sy).gc.fg = (*base).fg;
                    (*sy).gc.bg = (*base).bg;
                    (*sy).gc.us = (*base).us;
                    (*sy).gc.attr = (*base).attr;
                    (*sy).gc.flags = (*base).flags;
                } else if strcasecmp(tmp, c"ignore".as_ptr()) == 0 {
                    (*sy).ignore = 1;
                } else if strcasecmp(tmp, c"noignore".as_ptr()) == 0 {
                    (*sy).ignore = 0;
                } else if strcasecmp(tmp, c"push-default".as_ptr()) == 0 {
                    (*sy).default_type = style_default_type::STYLE_DEFAULT_PUSH;
                } else if strcasecmp(tmp, c"pop-default".as_ptr()) == 0 {
                    (*sy).default_type = style_default_type::STYLE_DEFAULT_POP;
                } else if strcasecmp(tmp, c"nolist".as_ptr()) == 0 {
                    (*sy).list = style_list::STYLE_LIST_OFF;
                } else if strncasecmp(tmp, c"list=".as_ptr(), 5) == 0 {
                    if strcasecmp(tmp.add(5), c"on".as_ptr()) == 0 {
                        (*sy).list = style_list::STYLE_LIST_ON;
                    } else if strcasecmp(tmp.add(5), c"focus".as_ptr()) == 0 {
                        (*sy).list = style_list::STYLE_LIST_FOCUS;
                    } else if strcasecmp(tmp.add(5), c"left-marker".as_ptr()) == 0 {
                        (*sy).list = style_list::STYLE_LIST_LEFT_MARKER;
                    } else if strcasecmp(tmp.add(5), c"right-marker".as_ptr()) == 0 {
                        (*sy).list = style_list::STYLE_LIST_RIGHT_MARKER;
                    } else {
                        break 'error;
                    }
                } else if strcasecmp(tmp, c"norange".as_ptr()) == 0 {
                    (*sy).range_type = style_default.range_type;
                    (*sy).range_argument = style_default.range_type as u32;
                    strlcpy(
                        &raw mut (*sy).range_string as *mut i8,
                        &raw const style_default.range_string as *const i8,
                        16,
                    );
                } else if end > 6 && strncasecmp(tmp, c"range=".as_ptr(), 6) == 0 {
                    found = strchr(tmp.add(6), b'|' as i32);
                    if !found.is_null() {
                        *found = b'\0' as _;
                        *found += 1;
                        if *found == b'\0' as _ {
                            break 'error;
                        }
                    }
                    if strcasecmp(tmp.add(6), c"left".as_ptr()) == 0 {
                        if !found.is_null() {
                            break 'error;
                        }
                        (*sy).range_type = style_range_type::STYLE_RANGE_LEFT;
                        (*sy).range_argument = 0;
                        style_set_range_string(sy, c"".as_ptr());
                    } else if strcasecmp(tmp.add(6), c"right".as_ptr()) == 0 {
                        if !found.is_null() {
                            break 'error;
                        }
                        (*sy).range_type = style_range_type::STYLE_RANGE_RIGHT;
                        (*sy).range_argument = 0;
                        style_set_range_string(sy, c"".as_ptr());
                    } else if strcasecmp(tmp.add(6), c"pane".as_ptr()) == 0 {
                        if found.is_null() {
                            break 'error;
                        }
                        if *found != b'%' as _ || *found.add(1) == b'\0' as _ {
                            break 'error;
                        }
                        let Ok(n) = strtonum(found.add(1), 0, u32::MAX) else {
                            break 'error;
                        };
                        (*sy).range_type = style_range_type::STYLE_RANGE_PANE;
                        (*sy).range_argument = n;
                        style_set_range_string(sy, c"".as_ptr());
                    } else if strcasecmp(tmp.add(6), c"window".as_ptr()) == 0 {
                        if found.is_null() {
                            break 'error;
                        }
                        let Ok(n) = strtonum(found, 0, u32::MAX) else {
                            break 'error;
                        };
                        (*sy).range_type = style_range_type::STYLE_RANGE_WINDOW;
                        (*sy).range_argument = n;
                        style_set_range_string(sy, c"".as_ptr());
                    } else if strcasecmp(tmp.add(6), c"session".as_ptr()) == 0 {
                        if found.is_null() {
                            break 'error;
                        }
                        if *found != b'$' as _ || *found.add(1) == b'\0' as _ {
                            break 'error;
                        }
                        let Ok(n) = strtonum(found.add(1), 0, u32::MAX) else {
                            break 'error;
                        };
                        (*sy).range_type = style_range_type::STYLE_RANGE_SESSION;
                        (*sy).range_argument = n;
                        style_set_range_string(sy, c"".as_ptr());
                    } else if strcasecmp(tmp.add(6), c"user".as_ptr()) == 0 {
                        if found.is_null() {
                            break 'error;
                        }
                        (*sy).range_type = style_range_type::STYLE_RANGE_USER;
                        (*sy).range_argument = 0;
                        style_set_range_string(sy, found);
                    }
                } else if strcasecmp(tmp, c"noalign".as_ptr()) == 0 {
                    (*sy).align = style_default.align;
                } else if end > 6 && strncasecmp(tmp, c"align=".as_ptr(), 6) == 0 {
                    if strcasecmp(tmp.add(6), c"left".as_ptr()) == 0 {
                        (*sy).align = style_align::STYLE_ALIGN_LEFT;
                    } else if strcasecmp(tmp.add(6), c"centre".as_ptr()) == 0 {
                        (*sy).align = style_align::STYLE_ALIGN_CENTRE;
                    } else if strcasecmp(tmp.add(6), c"right".as_ptr()) == 0 {
                        (*sy).align = style_align::STYLE_ALIGN_RIGHT;
                    } else if strcasecmp(tmp.add(6), c"absolute-centre".as_ptr()) == 0 {
                        (*sy).align = style_align::STYLE_ALIGN_ABSOLUTE_CENTRE;
                    } else {
                        break 'error;
                    }
                } else if end > 5 && strncasecmp(tmp, c"fill=".as_ptr(), 5) == 0 {
                    let value = colour_fromstring(tmp.add(5));
                    if value == -1 {
                        break 'error;
                    }
                    (*sy).fill = value;
                } else if end > 3 && strncasecmp(tmp.add(1), c"g=".as_ptr(), 2) == 0 {
                    let value = colour_fromstring(tmp.add(3));
                    if value == -1 {
                        break 'error;
                    }
                    if *in_ == b'f' as _ || *in_ == b'F' as _ {
                        if value != 8 {
                            (*sy).gc.fg = value;
                        } else {
                            (*sy).gc.fg = (*base).fg;
                        }
                    } else if *in_ == b'b' as _ || *in_ == b'B' as _ {
                        if value != 8 {
                            (*sy).gc.bg = value;
                        } else {
                            (*sy).gc.bg = (*base).bg;
                        }
                    } else {
                        break 'error;
                    }
                } else if end > 3 && strncasecmp(tmp, c"us=".as_ptr(), 3) == 0 {
                    let value = colour_fromstring(tmp.add(3));
                    if value == -1 {
                        break 'error;
                    }
                    if value != 8 {
                        (*sy).gc.us = value;
                    } else {
                        (*sy).gc.us = (*base).us;
                    }
                } else if strcasecmp(tmp, c"none".as_ptr()) == 0 {
                    (*sy).gc.attr = grid_attr::empty();
                } else if end > 2 && strncasecmp(tmp, c"no".as_ptr(), 2) == 0 {
                    let Ok(value) = attributes_fromstring(tmp.add(2)) else {
                        break 'error;
                    };
                    (*sy).gc.attr &= !value;
                } else {
                    let Ok(value) = attributes_fromstring(tmp) else {
                        break 'error;
                    };
                    (*sy).gc.attr |= value;
                }

                in_ = in_.add(end + strspn(in_.add(end), delimiters));
                if *in_ == b'\0' as _ {
                    break;
                }
            }

            return 0;
        }

        // error:
        style_copy(sy, &raw const saved);
        -1
    }
}

pub unsafe extern "C" fn style_tostring(sy: *const style) -> *const c_char {
    type s_type = [i8; 256];
    static mut s_buf: MaybeUninit<s_type> = MaybeUninit::<s_type>::uninit();

    unsafe {
        let gc = &raw const (*sy).gc;
        let mut off: i32 = 0;
        let mut comma = c"".as_ptr();
        let mut tmp = c"".as_ptr();
        type b_type = [i8; 21];
        let mut b: b_type = [0; 21];

        let s = &raw mut s_buf as *mut c_char;
        *s = b'\0' as c_char;

        if (*sy).list != style_list::STYLE_LIST_OFF {
            if (*sy).list == style_list::STYLE_LIST_ON {
                tmp = c"on".as_ptr();
            } else if (*sy).list == style_list::STYLE_LIST_FOCUS {
                tmp = c"focus".as_ptr();
            } else if (*sy).list == style_list::STYLE_LIST_LEFT_MARKER {
                tmp = c"left-marker".as_ptr();
            } else if (*sy).list == style_list::STYLE_LIST_RIGHT_MARKER {
                tmp = c"right-marker".as_ptr();
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}list={}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*sy).range_type != style_range_type::STYLE_RANGE_NONE {
            if (*sy).range_type == style_range_type::STYLE_RANGE_LEFT {
                tmp = c"left".as_ptr();
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_RIGHT {
                tmp = c"right".as_ptr();
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_PANE {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"pane|%%%u".as_ptr(),
                    (*sy).range_argument,
                );
                tmp = &raw const b as _;
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_WINDOW {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"window|%u".as_ptr(),
                    (*sy).range_argument,
                );
                tmp = &raw const b as _;
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_SESSION {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"session|$%u".as_ptr(),
                    (*sy).range_argument,
                );
                tmp = &raw const b as _;
            } else if (*sy).range_type == style_range_type::STYLE_RANGE_USER {
                snprintf(
                    &raw mut b as _,
                    size_of::<b_type>(),
                    c"user|%s".as_ptr(),
                    (*sy).range_string,
                );
                tmp = &raw const b as _;
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}range={}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*sy).align != style_align::STYLE_ALIGN_DEFAULT {
            if (*sy).align == style_align::STYLE_ALIGN_LEFT {
                tmp = c"left".as_ptr();
            } else if (*sy).align == style_align::STYLE_ALIGN_CENTRE {
                tmp = c"centre".as_ptr();
            } else if (*sy).align == style_align::STYLE_ALIGN_RIGHT {
                tmp = c"right".as_ptr();
            } else if (*sy).align == style_align::STYLE_ALIGN_ABSOLUTE_CENTRE {
                tmp = c"absolute-centre".as_ptr();
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}align={}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*sy).default_type != style_default_type::STYLE_DEFAULT_BASE {
            if (*sy).default_type == style_default_type::STYLE_DEFAULT_PUSH {
                tmp = c"push-default".as_ptr();
            } else if (*sy).default_type == style_default_type::STYLE_DEFAULT_POP {
                tmp = c"pop-default".as_ptr();
            }
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}{}",
                _s(comma),
                _s(tmp),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*sy).fill != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}fill={}",
                _s(comma),
                _s(colour_tostring((*sy).fill)),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*gc).fg != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}fg={}",
                _s(comma),
                _s(colour_tostring((*gc).fg)),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*gc).bg != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}bg={}",
                _s(comma),
                _s(colour_tostring((*gc).bg)),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if (*gc).us != 8 {
            off += xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}us={}",
                _s(comma),
                _s(colour_tostring((*gc).us)),
            )
            .unwrap() as i32;
            comma = c",".as_ptr();
        }
        if !(*gc).attr.is_empty() {
            xsnprintf_!(
                s.add(off as usize),
                size_of::<s_type>() - off as usize,
                "{}{}",
                _s(comma),
                _s(attributes_tostring((*gc).attr)),
            );
            comma = c",".as_ptr();
        }

        if *s == b'\0' as c_char {
            return c"default".as_ptr();
        }
        s
    }
}

pub unsafe extern "C" fn style_add(
    gc: *mut grid_cell,
    oo: *mut options,
    name: *const c_char,
    mut ft: *mut format_tree,
) {
    unsafe {
        let mut sy: *mut style = null_mut();
        let mut ft0: *mut format_tree = null_mut();

        if ft.is_null() {
            ft0 = format_create(null_mut(), null_mut(), 0, format_flags::FORMAT_NOJOBS);
            ft = ft0;
        }

        sy = options_string_to_style(oo, name, ft);
        if sy.is_null() {
            sy = &raw mut style_default;
        }
        if (*sy).gc.fg != 8 {
            (*gc).fg = (*sy).gc.fg;
        }
        if (*sy).gc.bg != 8 {
            (*gc).bg = (*sy).gc.bg;
        }
        if (*sy).gc.us != 8 {
            (*gc).us = (*sy).gc.us;
        }
        (*gc).attr |= (*sy).gc.attr;

        if !ft0.is_null() {
            format_free(ft0);
        }
    }
}

pub unsafe extern "C" fn style_apply(
    gc: *mut grid_cell,
    oo: *mut options,
    name: *const c_char,
    ft: *mut format_tree,
) {
    unsafe {
        memcpy__(gc, &raw const grid_default_cell);
        style_add(gc, oo, name, ft);
    }
}

pub unsafe extern "C" fn style_set(sy: *mut style, gc: *const grid_cell) {
    unsafe {
        memcpy__(sy, &raw const style_default);
        memcpy__(&raw mut (*sy).gc, gc);
    }
}

pub unsafe extern "C" fn style_copy(dst: *mut style, src: *const style) {
    unsafe {
        memcpy__(dst, src);
    }
}
