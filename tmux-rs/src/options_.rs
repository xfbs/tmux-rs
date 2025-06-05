use crate::*;

use libc::{fnmatch, isdigit, sscanf, strcasecmp, strchr, strcmp, strncmp, strstr};

use crate::compat::{
    RB_GENERATE_STATIC,
    queue::tailq_foreach,
    strtonum,
    tree::{rb_find, rb_foreach, rb_init, rb_insert, rb_min, rb_next, rb_remove},
};
use crate::log::fatalx_c;

//
// Option handling; each option has a name, type and value and is stored in
// a red-black tree.
//

#[repr(C)]
#[derive(Copy, Clone)]
pub struct options_array_item {
    pub index: u32,
    pub value: options_value,
    pub entry: rb_entry<options_array_item>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_cmp(a1: *const options_array_item, a2: *const options_array_item) -> c_int {
    unsafe {
        if (*a1).index < (*a2).index {
            return -1;
        }
        if (*a1).index > (*a2).index {
            return 1;
        }
        return 0;
    }
}
RB_GENERATE_STATIC!(options_array, options_array_item, entry, options_array_cmp);

#[repr(C)]
pub struct options_entry {
    pub owner: *mut options,
    pub name: *const c_char,
    pub tableentry: *const options_table_entry,
    pub value: options_value,
    pub cached: i32,
    pub style: style,
    pub entry: rb_entry<options_entry>,
}

#[repr(C)]
pub struct options {
    pub tree: rb_head<options_entry>,
    pub parent: *mut options,
}

#[inline]
#[unsafe(no_mangle)]
pub fn OPTIONS_IS_STRING(o: *const options_entry) -> bool { unsafe { (*o).tableentry.is_null() || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_STRING } }

#[inline]
#[unsafe(no_mangle)]
pub fn OPTIONS_IS_NUMBER(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null()
            && ((*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_NUMBER
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_KEY
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_FLAG
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_CHOICE)
    }
}

#[inline]
#[unsafe(no_mangle)]
pub fn OPTIONS_IS_COMMAND(o: *const options_entry) -> bool { unsafe { !(*o).tableentry.is_null() && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COMMAND } }

#[inline]
#[unsafe(no_mangle)]
pub fn OPTIONS_IS_ARRAY(o: *const options_entry) -> bool { unsafe { !(*o).tableentry.is_null() && ((*(*o).tableentry).flags & OPTIONS_TABLE_IS_ARRAY) != 0 } }

RB_GENERATE_STATIC!(options_tree, options_entry, entry, options_cmp);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_cmp(lhs: *const options_entry, rhs: *const options_entry) -> i32 { unsafe { libc::strcmp((*lhs).name, (*rhs).name) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_map_name(name: *const c_char) -> *const c_char {
    unsafe {
        let mut map = &raw const options_other_names as *const options_name_map;
        while !(*map).from.is_null() {
            if libc::strcmp((*map).from, name) == 0 {
                return (*map).to;
            }
            map = map.add(1);
        }
        name
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_parent_table_entry(oo: *mut options, s: *const c_char) -> *const options_table_entry {
    unsafe {
        if (*oo).parent.is_null() {
            fatalx_c(c"no parent options for %s".as_ptr(), s);
        }

        let o = options_get((*oo).parent, s);
        if o.is_null() {
            fatalx_c(c"%s not in parent options".as_ptr(), s);
        }

        (*o).tableentry
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_value_free(o: *const options_entry, ov: *mut options_value) {
    unsafe {
        if OPTIONS_IS_STRING(o) {
            free_((*ov).string);
        }
        if OPTIONS_IS_COMMAND(o) && !(*ov).cmdlist.is_null() {
            cmd_list_free((*ov).cmdlist);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_value_to_string(o: *mut options_entry, ov: *mut options_value, numeric: i32) -> *mut c_char {
    unsafe {
        let mut s: *mut c_char = null_mut();

        if OPTIONS_IS_COMMAND(o) {
            return cmd_list_print((*ov).cmdlist, 0);
        }

        if OPTIONS_IS_NUMBER(o) {
            match (*(*o).tableentry).type_ {
                options_table_type::OPTIONS_TABLE_NUMBER => {
                    xasprintf(&raw mut s, c"%lld".as_ptr(), (*ov).number);
                }
                options_table_type::OPTIONS_TABLE_KEY => {
                    s = xstrdup(key_string_lookup_key((*ov).number as u64, 0)).as_ptr();
                }
                options_table_type::OPTIONS_TABLE_COLOUR => {
                    s = xstrdup(colour_tostring((*ov).number as i32)).as_ptr();
                }
                options_table_type::OPTIONS_TABLE_FLAG => {
                    if numeric != 0 {
                        xasprintf(&mut s, c"%lld".as_ptr(), (*ov).number);
                    } else {
                        s = xstrdup(if (*ov).number != 0 { c"on".as_ptr() } else { c"off".as_ptr() }).as_ptr();
                    }
                }
                options_table_type::OPTIONS_TABLE_CHOICE => {
                    s = xstrdup(*(*(*o).tableentry).choices.add((*ov).number as usize)).as_ptr();
                }
                _ => {
                    fatalx(c"not a number option type");
                }
            }
            return s;
        }

        if OPTIONS_IS_STRING(o) {
            return xstrdup((*ov).string).as_ptr();
        }

        xstrdup(c"".as_ptr()).as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_create(parent: *mut options) -> *mut options {
    unsafe {
        let oo = xcalloc1::<options>() as *mut options;
        rb_init(&raw mut (*oo).tree);
        (*oo).parent = parent;
        oo
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_free(oo: *mut options) {
    unsafe {
        for o in rb_foreach(&raw mut (*oo).tree) {
            options_remove(o.as_ptr());
        }
        free_(oo);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_get_parent(oo: *mut options) -> *mut options { unsafe { (*oo).parent } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_set_parent(oo: *mut options, parent: *mut options) {
    unsafe {
        (*oo).parent = parent;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_first(oo: *mut options) -> *mut options_entry { unsafe { rb_min(&raw mut (*oo).tree) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_next(o: *mut options_entry) -> *mut options_entry { unsafe { rb_next(o) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_get_only(oo: *mut options, name: *const c_char) -> *mut options_entry {
    unsafe {
        let mut o = options_entry {
            name,
            ..unsafe { zeroed() } // TODO use uninit
        };

        let found = rb_find(&raw mut (*oo).tree, &raw const o);
        if found.is_null() {
            o.name = options_map_name(name);
            rb_find(&raw mut (*oo).tree, &o)
        } else {
            found
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_get(mut oo: *mut options, name: *const c_char) -> *mut options_entry {
    unsafe {
        let mut o = options_get_only(oo, name);
        while o.is_null() {
            oo = (*oo).parent;
            if oo.is_null() {
                break;
            }
            o = options_get_only(oo, name);
        }
        o
    }
}

pub unsafe fn options_get_(mut oo: *mut options, name: &CStr) -> *mut options_entry {
    unsafe {
        let mut o;
        while {
            o = options_get_only(oo, name.as_ptr());
            o.is_null()
        } {
            oo = (*oo).parent;
            if oo.is_null() {
                break;
            }
        }
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_empty(oo: *mut options, oe: *const options_table_entry) -> *mut options_entry {
    unsafe {
        let o = options_add(oo, (*oe).name);
        (*o).tableentry = oe;

        if (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            rb_init(&raw mut (*o).value.array);
        }
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_default(oo: *mut options, oe: *const options_table_entry) -> *mut options_entry {
    unsafe {
        let o = options_empty(oo, oe);
        let ov = &raw mut (*o).value;

        if (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            if (*oe).default_arr.is_null() {
                options_array_assign(o, (*oe).default_str, null_mut());
                return o;
            }
            let mut i = 0usize;
            while !(*(*oe).default_arr.add(i)).is_null() {
                options_array_set(o, i as u32, *(*oe).default_arr.add(i), 0, null_mut());
                i += 1;
            }
            return o;
        }

        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                (*ov).string = xstrdup((*oe).default_str).as_ptr();
            }
            _ => {
                (*ov).number = (*oe).default_num;
            }
        }
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_default_to_string(oe: *const options_table_entry) -> NonNull<c_char> {
    unsafe {
        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING | options_table_type::OPTIONS_TABLE_COMMAND => xstrdup((*oe).default_str),
            options_table_type::OPTIONS_TABLE_NUMBER => {
                let mut s = null_mut();
                xasprintf(&mut s, c"%lld".as_ptr(), (*oe).default_num);
                NonNull::new(s).unwrap()
            }
            options_table_type::OPTIONS_TABLE_KEY => xstrdup(key_string_lookup_key((*oe).default_num as u64, 0)),
            options_table_type::OPTIONS_TABLE_COLOUR => xstrdup(colour_tostring((*oe).default_num as i32)),
            options_table_type::OPTIONS_TABLE_FLAG => xstrdup(if (*oe).default_num != 0 { c"on".as_ptr() } else { c"off".as_ptr() } as *const c_char),
            options_table_type::OPTIONS_TABLE_CHOICE => xstrdup(*(*oe).choices.add((*oe).default_num as usize)),
        }
    }
}

#[unsafe(no_mangle)]
unsafe fn options_add(oo: *mut options, name: *const c_char) -> *mut options_entry {
    unsafe {
        let mut o = options_get_only(oo, name);
        if !o.is_null() {
            options_remove(o);
        }

        o = xcalloc1::<options_entry>() as *mut options_entry;
        (*o).owner = oo;
        (*o).name = xstrdup(name).as_ptr();

        rb_insert(&raw mut (*oo).tree, o);
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_remove(o: *mut options_entry) {
    unsafe {
        let oo = (*o).owner;

        if options_is_array(o) != 0 {
            options_array_clear(o);
        } else {
            options_value_free(o, &mut (*o).value);
        }
        rb_remove(&mut (*oo).tree, o);
        free_((*o).name.cast_mut()); // TODO cast away const
        free_(o);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_name(o: *mut options_entry) -> *const c_char { unsafe { (*o).name } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_owner(o: *mut options_entry) -> *mut options { unsafe { (*o).owner } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_table_entry(o: *mut options_entry) -> *const options_table_entry { unsafe { (*o).tableentry } }

unsafe fn options_array_item(o: *mut options_entry, idx: c_uint) -> *mut options_array_item {
    unsafe {
        let mut a = options_array_item {
            index: idx,
            ..unsafe { zeroed() } // TODO use uninit
        };
        rb_find(&raw mut (*o).value.array, &raw mut a)
    }
}

unsafe fn options_array_new(o: *mut options_entry, idx: c_uint) -> *mut options_array_item {
    unsafe {
        let a = xcalloc1::<options_array_item>() as *mut options_array_item;
        (*a).index = idx;
        rb_insert(&mut (*o).value.array, a);
        a
    }
}

unsafe fn options_array_free(o: *mut options_entry, a: *mut options_array_item) {
    unsafe {
        options_value_free(o, &mut (*a).value);
        rb_remove(&mut (*o).value.array, a);
        free_(a);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_clear(o: *mut options_entry) {
    unsafe {
        if options_is_array(o) == 0 {
            return;
        }

        let mut a = rb_min(&raw mut (*o).value.array);
        while !a.is_null() {
            let next = rb_next(a) as *mut options_array_item;
            options_array_free(o, a);
            a = next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_get(o: *mut options_entry, idx: u32) -> *mut options_value {
    unsafe {
        if options_is_array(o) == 0 {
            return null_mut();
        }
        let a = options_array_item(o, idx);
        if a.is_null() {
            return null_mut();
        }
        &raw mut (*a).value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_set(o: *mut options_entry, idx: u32, value: *const c_char, append: i32, cause: *mut *mut c_char) -> i32 {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            if !cause.is_null() {
                *cause = xstrdup(c"not an array".as_ptr()).as_ptr();
            }
            return -1;
        }

        if value.is_null() {
            let a = options_array_item(o, idx);
            if !a.is_null() {
                options_array_free(o, a);
            }
            return 0;
        }

        if OPTIONS_IS_COMMAND(o) {
            let pr = cmd_parse_from_string(value, null_mut());
            match (*pr).status {
                cmd_parse_status::CMD_PARSE_ERROR => {
                    if !cause.is_null() {
                        *cause = (*pr).error;
                    } else {
                        free_((*pr).error);
                    }
                    return -1;
                }
                cmd_parse_status::CMD_PARSE_SUCCESS => (),
            }

            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.cmdlist = (*pr).cmdlist;
            return 0;
        }

        if OPTIONS_IS_STRING(o) {
            let mut a = options_array_item(o, idx);
            let new = if !a.is_null() && append != 0 {
                let mut new = null_mut();
                xasprintf(&mut new, "%s%s\0".as_ptr() as *const c_char, (*a).value.string, value);
                new
            } else {
                xstrdup(value).as_ptr()
            };

            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &mut (*a).value);
            }
            (*a).value.string = new;
            return 0;
        }

        if !(*o).tableentry.is_null() && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR {
            let number = colour_fromstring(value);
            if number == -1 {
                xasprintf(cause, c"bad colour: %s".as_ptr(), value);
                return -1;
            }
            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.number = number as i64;
            return 0;
        }

        if !cause.is_null() {
            *cause = xstrdup(c"wrong array type".as_ptr()).as_ptr();
        }
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_assign(o: *mut options_entry, s: *const c_char, cause: *mut *mut c_char) -> i32 {
    unsafe {
        let mut separator = (*(*o).tableentry).separator;
        if separator.is_null() {
            separator = c" ,".as_ptr();
        }
        if *separator == 0 {
            if *s == 0 {
                return 0;
            }
            let mut i = 0;
            while i < u32::MAX {
                if options_array_item(o, i).is_null() {
                    break;
                }
                i += 1;
            }
            return options_array_set(o, i, s, 0, cause);
        }

        if *s == 0 {
            return 0;
        }
        let copy = xstrdup(s).as_ptr();
        let mut string = copy;
        while let Some(next) = NonNull::new(strsep(&raw mut string, separator)) {
            let next = next.as_ptr();
            if *next == 0 {
                continue;
            }
            let mut i = 0;
            while i < u32::MAX {
                if options_array_item(o, i).is_null() {
                    break;
                }
                i += 1;
            }
            if i == u32::MAX {
                break;
            }
            if options_array_set(o, i, next, 0, cause) != 0 {
                free_(copy);
                return -1;
            }
        }
        free_(copy);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_first(o: *mut options_entry) -> *mut options_array_item {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            return null_mut();
        }
        rb_min(&raw mut (*o).value.array)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_next(a: *mut options_array_item) -> *mut options_array_item { unsafe { rb_next(a) } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_item_index(a: *mut options_array_item) -> u32 { unsafe { (*a).index } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_array_item_value(a: *mut options_array_item) -> *mut options_value { unsafe { &raw mut (*a).value } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_is_array(o: *mut options_entry) -> i32 { unsafe { OPTIONS_IS_ARRAY(o) as i32 } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_is_string(o: *mut options_entry) -> i32 { unsafe { OPTIONS_IS_STRING(o) as i32 } }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_to_string(o: *mut options_entry, idx: i32, numeric: i32) -> *mut c_char {
    unsafe {
        if OPTIONS_IS_ARRAY(o) {
            if idx == -1 {
                let mut result = null_mut();
                let mut last: *mut i8 = null_mut();

                let mut a = rb_min(&raw mut (*o).value.array);
                while !a.is_null() {
                    let next = options_value_to_string(o, &raw mut (*a.cast::<options_array_item>()).value, numeric);

                    if last.is_null() {
                        result = next;
                    } else {
                        let mut new_result = null_mut();
                        xasprintf(&mut new_result, c"%s %s".as_ptr(), last, next);
                        free_(last);
                        free_(next);
                        result = new_result;
                    }
                    last = result;

                    a = rb_next(a);
                }

                if result.is_null() {
                    return xstrdup(c"".as_ptr()).as_ptr();
                }
                return result;
            }

            let a = options_array_item(o, idx as u32);
            if a.is_null() {
                return xstrdup(c"".as_ptr()).as_ptr();
            }
            return options_value_to_string(o, &raw mut (*a).value, numeric);
        }

        options_value_to_string(o, &raw mut (*o).value, numeric)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_parse(name: *const c_char, idx: *mut i32) -> *mut c_char {
    unsafe {
        if *name == 0 {
            return null_mut();
        }

        let copy = xstrdup(name).as_ptr();
        let cp = strchr(copy, b'[' as i32);

        if cp.is_null() {
            *idx = -1;
            return copy;
        }

        let end = strchr(cp.offset(1), b']' as i32);
        if end.is_null() || *end.offset(1) != 0 || isdigit(*end.offset(-1) as i32) == 0 {
            free_(copy);
            return null_mut();
        }

        let mut parsed_idx = 0;
        if sscanf(cp, c"[%d]".as_ptr(), &mut parsed_idx) != 1 || parsed_idx < 0 {
            free_(copy);
            return null_mut();
        }

        *idx = parsed_idx;
        *cp = 0;
        copy
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_parse_get(oo: *mut options, s: *const c_char, idx: *mut i32, only: i32) -> *mut options_entry {
    unsafe {
        let name = options_parse(s, idx);
        if name.is_null() {
            return null_mut();
        }

        let o = if only != 0 { options_get_only(oo, name) } else { options_get(oo, name) };

        free_(name);
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_match(s: *const c_char, idx: *mut i32, ambiguous: *mut i32) -> *mut c_char {
    unsafe {
        let parsed = options_parse(s, idx);
        if parsed.is_null() {
            return null_mut();
        }

        if *parsed == b'@' as i8 {
            *ambiguous = 0;
            return parsed;
        }

        let name = options_map_name(parsed);
        let namelen = strlen(name);

        let mut found: *const options_table_entry = null();
        let mut oe = &raw const options_table as *const options_table_entry;

        while !(*oe).name.is_null() {
            if strcmp((*oe).name, name) == 0 {
                found = oe;
                break;
            }
            if strncmp((*oe).name, name, namelen) == 0 {
                if !found.is_null() {
                    *ambiguous = 1;
                    free_(parsed);
                    return null_mut();
                }
                found = oe;
            }
            oe = oe.add(1);
        }

        free_(parsed);
        if found.is_null() {
            *ambiguous = 0;
            return null_mut();
        }

        xstrdup((*found).name).as_ptr()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_match_get(oo: *mut options, s: *const c_char, idx: *mut i32, only: i32, ambiguous: *mut i32) -> *mut options_entry {
    unsafe {
        let name = options_match(s, idx, ambiguous);
        if name.is_null() {
            return null_mut();
        }

        *ambiguous = 0;
        let o = if only != 0 { options_get_only(oo, name) } else { options_get(oo, name) };

        free_(name);
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_get_string(oo: *mut options, name: *const c_char) -> *const c_char {
    unsafe {
        let o = options_get(oo, name);
        if o.is_null() {
            fatalx_c(c"missing option %s".as_ptr(), name);
        }
        if !OPTIONS_IS_STRING(o) {
            fatalx_c(c"option %s is not a string".as_ptr(), name);
        }
        (*o).value.string
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_get_number(oo: *mut options, name: *const c_char) -> i64 {
    unsafe {
        let o = options_get(oo, name);
        if o.is_null() {
            fatalx_c(c"missing option %s".as_ptr(), name);
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_c(c"option %s is not a number".as_ptr(), name);
        }
        (*o).value.number
    }
}

pub unsafe fn options_get_number_(oo: *mut options, name: &CStr) -> i64 {
    unsafe {
        let o = options_get_(oo, name);
        if o.is_null() {
            fatalx_c(c"missing option %s".as_ptr(), name);
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_c(c"option %s is not a number".as_ptr(), name);
        }
        (*o).value.number
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_set_string(oo: *mut options, name: *const c_char, append: c_int, fmt: *const c_char, mut ap: ...) -> *mut options_entry {
    unsafe {
        let mut s: *mut c_char = null_mut();
        let mut separator = c"".as_ptr();
        let mut value: *mut c_char = null_mut();

        xvasprintf(&mut s, fmt, ap.as_va_list());

        let mut o = options_get_only(oo, name);
        if !o.is_null() && append != 0 && OPTIONS_IS_STRING(o) {
            if *name != b'@' as c_char {
                separator = (*(*o).tableentry).separator;
                if separator.is_null() {
                    separator = c"".as_ptr();
                }
            }
            xasprintf(&raw mut value, c"%s%s%s".as_ptr(), (*o).value.string, separator, s);
            free_(s);
        } else {
            value = s;
        }

        if o.is_null() && *name == b'@' as c_char {
            o = options_add(oo, name);
        } else if o.is_null() {
            o = options_default(oo, options_parent_table_entry(oo, name));
            if o.is_null() {
                return null_mut();
            }
        }

        if !OPTIONS_IS_STRING(o) {
            fatalx_c(c"option %s is not a string".as_ptr(), name);
        }
        free_((*o).value.string);
        (*o).value.string = value;
        (*o).cached = 0;
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_set_number(oo: *mut options, name: *const c_char, value: i64) -> *mut options_entry {
    unsafe {
        if *name == b'@' as c_char {
            fatalx_c(c"user option %s must be a string".as_ptr(), name);
        }

        let mut o = options_get_only(oo, name);
        if o.is_null() {
            o = options_default(oo, options_parent_table_entry(oo, name));
            if o.is_null() {
                return null_mut();
            }
        }

        if !OPTIONS_IS_NUMBER(o) {
            fatalx_c(c"option %s is not a number".as_ptr(), name);
        }
        (*o).value.number = value;
        o
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_scope_from_name(args: *mut args, window: i32, name: *const c_char, fs: *mut cmd_find_state, oo: *mut *mut options, cause: *mut *mut c_char) -> i32 {
    unsafe {
        let s = (*fs).s;
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');
        let mut scope = OPTIONS_TABLE_NONE;

        if *name == b'@' as c_char {
            return options_scope_from_flags(args, window, fs, oo, cause);
        }

        let mut oe = &raw const options_table as *const options_table_entry;
        while !(*oe).name.is_null() {
            if strcmp((*oe).name, name) == 0 {
                break;
            }
            oe = oe.add(1);
        }

        if (*oe).name.is_null() {
            xasprintf(cause, c"unknown option: %s".as_ptr(), name);
            return OPTIONS_TABLE_NONE;
        }

        const OPTIONS_TABLE_WINDOW_AND_PANE: i32 = OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE;
        match (*oe).scope {
            OPTIONS_TABLE_SERVER => {
                *oo = global_options;
                scope = OPTIONS_TABLE_SERVER;
            }
            OPTIONS_TABLE_SESSION => {
                if args_has_(args, 'g') {
                    *oo = global_s_options;
                    scope = OPTIONS_TABLE_SESSION;
                } else if s.is_null() && !target.is_null() {
                    xasprintf(cause, c"no such session: %s".as_ptr(), target);
                } else if s.is_null() {
                    xasprintf(cause, c"no current session".as_ptr());
                } else {
                    *oo = (*s).options;
                    scope = OPTIONS_TABLE_SESSION;
                }
            }
            OPTIONS_TABLE_WINDOW_AND_PANE => {
                if args_has_(args, 'p') {
                    if wp.is_null() && !target.is_null() {
                        xasprintf(cause, c"no such pane: %s".as_ptr(), target);
                    } else if wp.is_null() {
                        xasprintf(cause, c"no current pane".as_ptr());
                    } else {
                        *oo = (*wp).options;
                        scope = OPTIONS_TABLE_PANE;
                    }
                } else {
                    // FALLTHROUGH same as OPTIONS_TABLE_WINDOW case
                    if args_has_(args, 'g') {
                        *oo = global_w_options;
                        scope = OPTIONS_TABLE_WINDOW;
                    } else if wl.is_null() && !target.is_null() {
                        xasprintf(cause, c"no such window: %s".as_ptr(), target);
                    } else if wl.is_null() {
                        xasprintf(cause, c"no current window".as_ptr());
                    } else {
                        *oo = (*(*wl).window).options;
                        scope = OPTIONS_TABLE_WINDOW;
                    }
                }
            }
            OPTIONS_TABLE_WINDOW => {
                if args_has_(args, 'g') {
                    *oo = global_w_options;
                    scope = OPTIONS_TABLE_WINDOW;
                } else if wl.is_null() && !target.is_null() {
                    xasprintf(cause, c"no such window: %s".as_ptr(), target);
                } else if wl.is_null() {
                    xasprintf(cause, c"no current window".as_ptr());
                } else {
                    *oo = (*(*wl).window).options;
                    scope = OPTIONS_TABLE_WINDOW;
                }
            }
            _ => {}
        }
        scope
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_scope_from_flags(args: *mut args, window: i32, fs: *mut cmd_find_state, oo: *mut *mut options, cause: *mut *mut c_char) -> i32 {
    unsafe {
        let s = (*fs).s;
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');

        if args_has_(args, 's') {
            *oo = global_options;
            return OPTIONS_TABLE_SERVER;
        }

        if args_has_(args, 'p') {
            if wp.is_null() {
                if !target.is_null() {
                    xasprintf(cause, c"no such pane: %s".as_ptr(), target);
                } else {
                    xasprintf(cause, c"no current pane".as_ptr());
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*wp).options;
            return OPTIONS_TABLE_PANE;
        } else if window != 0 || args_has_(args, 'w') {
            if args_has_(args, 'g') {
                *oo = global_w_options;
                return OPTIONS_TABLE_WINDOW;
            }
            if wl.is_null() {
                if !target.is_null() {
                    xasprintf(cause, c"no such window: %s".as_ptr(), target);
                } else {
                    xasprintf(cause, c"no current window".as_ptr());
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*(*wl).window).options;
            return OPTIONS_TABLE_WINDOW;
        } else {
            if args_has_(args, 'g') {
                *oo = global_s_options;
                return OPTIONS_TABLE_SESSION;
            }
            if s.is_null() {
                if !target.is_null() {
                    xasprintf(cause, c"no such session: %s".as_ptr(), target);
                } else {
                    xasprintf(cause, c"no current session".as_ptr());
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*s).options;
            return OPTIONS_TABLE_SESSION;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_string_to_style(oo: *mut options, name: *const c_char, ft: *mut format_tree) -> *mut style {
    let __func__ = c"options_string_to_style".as_ptr();
    unsafe {
        let o = options_get(oo, name);
        if o.is_null() || !OPTIONS_IS_STRING(o) {
            return null_mut();
        }

        if (*o).cached != 0 {
            return &mut (*o).style;
        }
        let s = (*o).value.string;
        log_debug!("{}: {} is '{}'", _s(__func__), _s(name), _s(s));

        style_set(&mut (*o).style, &grid_default_cell);
        (*o).cached = if strstr(s, c"#{".as_ptr()).is_null() { 1 } else { 0 };

        if !ft.is_null() && (*o).cached == 0 {
            let expanded = format_expand(ft, s);
            if style_parse(&mut (*o).style, &grid_default_cell, expanded) != 0 {
                free_(expanded);
                return null_mut();
            }
            free_(expanded);
        } else {
            if style_parse(&mut (*o).style, &grid_default_cell, s) != 0 {
                return null_mut();
            }
        }
        &mut (*o).style
    }
}

unsafe fn options_from_string_check(oe: *const options_table_entry, value: *const c_char, cause: *mut *mut c_char) -> c_int {
    unsafe {
        let mut sy: style = std::mem::zeroed();

        if oe.is_null() {
            return 0;
        }
        if strcmp((*oe).name, c"default-shell".as_ptr()) == 0 && checkshell(value) == 0 {
            xasprintf(cause, c"not a suitable shell: %s".as_ptr(), value);
            return -1;
        }
        if !(*oe).pattern.is_null() && fnmatch((*oe).pattern, value, 0) != 0 {
            xasprintf(cause, c"value is invalid: %s".as_ptr(), value);
            return -1;
        }
        if ((*oe).flags & OPTIONS_TABLE_IS_STYLE) != 0 && strstr(value, c"#{".as_ptr()).is_null() && style_parse(&mut sy, &grid_default_cell, value) != 0 {
            xasprintf(cause, c"invalid style: %s".as_ptr(), value);
            return -1;
        }
        0
    }
}

unsafe fn options_from_string_flag(oo: *mut options, name: *const c_char, value: *const c_char, cause: *mut *mut c_char) -> c_int {
    unsafe {
        let flag = if value.is_null() || *value == 0 {
            !options_get_number(oo, name)
        } else if strcmp(value, c"1".as_ptr()) == 0 || strcasecmp(value, c"on".as_ptr()) == 0 || strcasecmp(value, c"yes".as_ptr()) == 0 {
            1
        } else if strcmp(value, c"0".as_ptr()) == 0 || strcasecmp(value, c"off".as_ptr()) == 0 || strcasecmp(value, c"no".as_ptr()) == 0 {
            0
        } else {
            xasprintf(cause, c"bad value: %s".as_ptr(), value);
            return -1;
        };
        options_set_number(oo, name, flag);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_find_choice(oe: *const options_table_entry, value: *const c_char, cause: *mut *mut c_char) -> c_int {
    unsafe {
        let mut n = 0;
        let mut choice = -1;
        let mut cp = (*oe).choices;

        while !(*cp).is_null() {
            if strcmp(*cp, value) == 0 {
                choice = n;
            }
            n += 1;
            cp = cp.add(1);
        }
        if choice == -1 {
            xasprintf(cause, c"unknown value: %s".as_ptr(), value);
            return -1;
        }
        choice
    }
}

unsafe fn options_from_string_choice(oe: *const options_table_entry, oo: *mut options, name: *const c_char, value: *const c_char, cause: *mut *mut c_char) -> c_int {
    unsafe {
        let choice = if value.is_null() {
            let mut choice = options_get_number(oo, name);
            if choice < 2 {
                choice = !choice;
            }
            choice
        } else {
            let choice = options_find_choice(oe, value, cause) as i64;
            if choice < 0 {
                return -1;
            }
            choice
        };
        options_set_number(oo, name, choice);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_from_string(oo: *mut options, oe: *const options_table_entry, name: *const c_char, value: *const c_char, append: c_int, cause: *mut *mut c_char) -> c_int {
    unsafe {
        let mut type_: options_table_type;
        let mut number: i64;
        let mut errstr: *const c_char;
        let mut new: *const c_char;
        let mut old: *mut c_char;
        let mut key: key_code;

        if !oe.is_null() {
            if value.is_null() && (*oe).type_ != options_table_type::OPTIONS_TABLE_FLAG && (*oe).type_ != options_table_type::OPTIONS_TABLE_CHOICE {
                xasprintf(cause, c"empty value".as_ptr());
                return -1;
            }
            type_ = (*oe).type_;
        } else {
            if *name != b'@' as c_char {
                xasprintf(cause, c"bad option name".as_ptr());
                return -1;
            }
            type_ = options_table_type::OPTIONS_TABLE_STRING;
        }

        match type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                old = xstrdup(options_get_string(oo, name)).as_ptr();
                options_set_string(oo, name, append, c"%s".as_ptr(), value);

                new = options_get_string(oo, name);
                if options_from_string_check(oe, new, cause) != 0 {
                    options_set_string(oo, name, 0, c"%s".as_ptr(), old);
                    free_(old);
                    return -1;
                }
                free_(old);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_NUMBER => {
                let mut errstr = null();
                number = strtonum(value, (*oe).minimum as i64, (*oe).maximum as i64, &raw mut errstr);
                if !errstr.is_null() {
                    xasprintf(cause, c"value is %s: %s".as_ptr(), errstr, value);
                    return -1;
                }
                options_set_number(oo, name, number);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_KEY => {
                key = key_string_lookup_string(value);
                if key == KEYC_UNKNOWN {
                    xasprintf(cause, c"bad key: %s".as_ptr(), value);
                    return -1;
                }
                options_set_number(oo, name, key as i64);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_COLOUR => {
                number = colour_fromstring(value) as i64;
                if number == -1 {
                    xasprintf(cause, c"bad colour: %s".as_ptr(), value);
                    return -1;
                }
                options_set_number(oo, name, number);
                return 0;
            }

            options_table_type::OPTIONS_TABLE_FLAG => {
                return options_from_string_flag(oo, name, value, cause);
            }

            options_table_type::OPTIONS_TABLE_CHOICE => {
                return options_from_string_choice(oe, oo, name, value, cause);
            }

            options_table_type::OPTIONS_TABLE_COMMAND => {}

            _ => {}
        }
        return -1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_push_changes(name: *const c_char) {
    let __func__ = c"options_push_changes".as_ptr();
    unsafe {
        let mut loop_: *mut client;
        let mut s: *mut session;
        let mut w: *mut window;
        let mut wp: *mut window_pane;

        log_debug!("{}: {}", _s(__func__), _s(name));

        if strcmp(name, c"automatic-rename".as_ptr()) == 0 {
            for w in rb_foreach(&raw mut windows).map(NonNull::as_ptr) {
                if (*w).active.is_null() {
                    continue;
                }
                if options_get_number((*w).options, name) != 0 {
                    (*(*w).active).flags |= window_pane_flags::PANE_CHANGED;
                }
            }
        }

        if strcmp(name, c"cursor-colour".as_ptr()) == 0 {
            for wp in rb_foreach(&raw mut all_window_panes) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if strcmp(name, c"cursor-style".as_ptr()) == 0 {
            for wp in rb_foreach(&raw mut all_window_panes) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if strcmp(name, c"fill-character".as_ptr()) == 0 {
            for w in rb_foreach(&raw mut windows) {
                window_set_fill_character(w);
            }
        }

        if strcmp(name, c"key-table".as_ptr()) == 0 {
            for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                server_client_set_key_table(loop_, null_mut());
            }
        }

        if strcmp(name, c"user-keys".as_ptr()) == 0 {
            for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
                if (*loop_).tty.flags.intersects(tty_flags::TTY_OPENED) {
                    tty_keys_build(&mut (*loop_).tty);
                }
            }
        }

        if strcmp(name, c"status".as_ptr()) == 0 || strcmp(name, c"status-interval".as_ptr()) == 0 {
            status_timer_start_all();
        }

        if strcmp(name, c"monitor-silence".as_ptr()) == 0 {
            alerts_reset_all();
        }

        if strcmp(name, c"window-style".as_ptr()) == 0 || strcmp(name, c"window-active-style".as_ptr()) == 0 {
            for wp in rb_foreach(&raw mut all_window_panes) {
                (*wp.as_ptr()).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
        }

        if strcmp(name, c"pane-colours".as_ptr()) == 0 {
            for wp in rb_foreach(&raw mut all_window_panes).map(NonNull::as_ptr) {
                colour_palette_from_option(&raw mut (*wp).palette, (*wp).options);
            }
        }

        if strcmp(name, c"pane-border-status".as_ptr()) == 0 {
            for w in rb_foreach(&raw mut windows) {
                layout_fix_panes(w.as_ptr(), null_mut());
            }
        }

        for s in rb_foreach(&raw mut sessions) {
            status_update_cache(s.as_ptr());
        }

        recalculate_sizes();

        for loop_ in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if !(*loop_).session.is_null() {
                server_redraw_client(loop_);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn options_remove_or_default(o: *mut options_entry, idx: i32, cause: *mut *mut c_char) -> i32 {
    unsafe {
        let oo = (*o).owner;

        if idx == -1 {
            if !(*o).tableentry.is_null() && (oo == global_options || oo == global_s_options || oo == global_w_options) {
                options_default(oo, (*o).tableentry);
            } else {
                options_remove(o);
            }
        } else if options_array_set(o, idx as u32, null(), 0, cause) != 0 {
            return -1;
        }
        0
    }
}
