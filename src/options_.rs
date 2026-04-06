// Copyright (c) 2008 Nicholas Marriott <nicholas.marriott@gmail.com>
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
use crate::libc::{fnmatch};
use crate::options_table::OPTIONS_OTHER_NAMES_STR;
use crate::*;

// Option handling; each option has a name, type and value and is stored in a HashMap.

use std::collections::HashMap;

use std::collections::BTreeMap;

#[derive(Copy, Clone)]
pub struct options_array_item {
    pub index: u32,
    pub value: options_value,
}

pub struct options_entry {
    owner: *mut options,
    name: Cow<'static, str>,
    tableentry: *const options_table_entry,
    value: options_value,
    cached: i32,
    style: style,
}

pub struct options {
    tree: HashMap<String, Box<options_entry>>,
    parent: *mut options,
}

#[expect(non_snake_case)]
#[inline]
unsafe fn OPTIONS_IS_STRING(o: *const options_entry) -> bool {
    unsafe {
        (*o).tableentry.is_null()
            || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_STRING
    }
}

#[expect(non_snake_case)]
#[inline]
fn OPTIONS_IS_NUMBER(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null()
            && ((*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_NUMBER
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_KEY
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_FLAG
                || (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_CHOICE)
    }
}

#[expect(non_snake_case)]
#[inline]
unsafe fn OPTIONS_IS_COMMAND(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null()
            && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COMMAND
    }
}

#[expect(non_snake_case)]
#[inline]
unsafe fn OPTIONS_IS_ARRAY(o: *const options_entry) -> bool {
    unsafe {
        !(*o).tableentry.is_null() && ((*(*o).tableentry).flags & OPTIONS_TABLE_IS_ARRAY) != 0
    }
}


fn options_map_name(name: &str) -> Option<&'static str> {
    for &options_name_map { from, to} in &OPTIONS_OTHER_NAMES {
        if from == name {
            return Some(to);
        }
    }
    None
}

fn options_map_name_str(name: &str) -> &str {
    for map in &OPTIONS_OTHER_NAMES_STR {
        if map.from == name {
            return map.to;
        }
    }
    name
}

unsafe fn options_parent_table_entry(
    oo: *mut options,
    s: &str,
) -> *const options_table_entry {
    unsafe {
        if (*oo).parent.is_null() {
            fatalx_!("no parent options for {s}");
        }

        let o = options_get(&mut *(*oo).parent, s);
        if o.is_null() {
            fatalx_!("{s} not in parent options");
        }

        (*o).tableentry
    }
}

unsafe fn options_value_free(o: *const options_entry, ov: *mut options_value) {
    unsafe {
        if OPTIONS_IS_STRING(o) {
            free_((*ov).string);
        }
        if OPTIONS_IS_COMMAND(o) && !(*ov).cmdlist.is_null() {
            cmd_list_free((*ov).cmdlist);
        }
    }
}

unsafe fn options_value_to_string(
    o: *mut options_entry,
    ov: *mut options_value,
    numeric: i32,
) -> *mut u8 {
    unsafe {
        if OPTIONS_IS_COMMAND(o) {
            return cmd_list_print(&*(*ov).cmdlist, 0);
        }

        if OPTIONS_IS_NUMBER(o) {
            let s = match (*(*o).tableentry).type_ {
                options_table_type::OPTIONS_TABLE_NUMBER => {
                    format_nul!("{}", (*ov).number)
                }
                options_table_type::OPTIONS_TABLE_KEY => {
                    xstrdup(key_string_lookup_key((*ov).number as u64, 0)).as_ptr()
                }
                options_table_type::OPTIONS_TABLE_COLOUR => {
                    CString::new(colour_tostring((*ov).number as i32).into_owned())
                        .unwrap()
                        .into_raw()
                        .cast()
                }
                options_table_type::OPTIONS_TABLE_FLAG => {
                    if numeric != 0 {
                        format_nul!("{}", (*ov).number)
                    } else {
                        xstrdup(if (*ov).number != 0 {
                            c!("on")
                        } else {
                            c!("off")
                        })
                        .as_ptr()
                    }
                }
                options_table_type::OPTIONS_TABLE_CHOICE => {
                    xstrdup__((*(*o).tableentry).choices[(*ov).number as usize])
                }
                _ => {
                    fatalx("not a number option type");
                }
            };
            return s;
        }

        if OPTIONS_IS_STRING(o) {
            return xstrdup((*ov).string).as_ptr();
        }

        xstrdup(c!("")).as_ptr()
    }
}

pub unsafe fn options_create(parent: *mut options) -> *mut options {
    let oo = Box::new(options {
        tree: HashMap::new(),
        parent,
    });
    Box::into_raw(oo)
}

pub unsafe fn options_free(oo: *mut options) {
    unsafe {
        let keys: Vec<String> = (*oo).tree.keys().cloned().collect();
        for key in keys {
            if let Some(entry) = (*oo).tree.get_mut(&key) {
                options_remove(&mut **entry as *mut options_entry);
            }
        }
        drop(Box::from_raw(oo));
    }
}

pub unsafe fn options_get_parent(oo: *mut options) -> *mut options {
    unsafe { (*oo).parent }
}

pub fn options_set_parent(oo: &mut options, parent: *mut options) {
    oo.parent = parent;
}

/// Collect all entry pointers in sorted order (by name).
pub unsafe fn options_entries(oo: *mut options) -> Vec<*mut options_entry> {
    unsafe {
        let mut entries: Vec<_> = (*oo)
            .tree
            .values_mut()
            .map(|e| &mut **e as *mut options_entry)
            .collect();
        entries.sort_by(|a, b| (**a).name.cmp(&(**b).name));
        entries
    }
}


pub unsafe fn options_get_only(oo: *mut options, name: &str) -> *mut options_entry {
    unsafe {
        if let Some(entry) = (*oo).tree.get_mut(name) {
            return &mut **entry as *mut options_entry;
        }
        // Try mapped name
        let mapped = options_map_name_str(name);
        if mapped != name {
            if let Some(entry) = (*oo).tree.get_mut(mapped) {
                return &mut **entry as *mut options_entry;
            }
        }
        null_mut()
    }
}

pub unsafe fn options_get_only_const(oo: *const options, name: &str) -> *const options_entry {
    unsafe {
        if let Some(entry) = (*oo).tree.get(name) {
            return &**entry as *const options_entry;
        }
        let mapped = options_map_name_str(name);
        if mapped != name {
            if let Some(entry) = (*oo).tree.get(mapped) {
                return &**entry as *const options_entry;
            }
        }
        null_mut()
    }
}

pub fn options_get(oo: &mut options, name: &str) -> *mut options_entry {
    #[expect(clippy::shadow_same)]
    let mut oo: *mut options = oo;

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

unsafe fn options_get_const(mut oo: *const options, name: &str) -> *const options_entry {
    unsafe {
        let mut o;
        while {
            o = options_get_only_const(oo, name);
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

pub unsafe fn options_empty(
    oo: *mut options,
    oe: *const options_table_entry,
) -> *mut options_entry {
    unsafe {
        let o = options_add(oo, (*oe).name);
        (*o).tableentry = oe;

        if (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            (*o).value.array = Box::into_raw(Box::new(BTreeMap::new()));
        }
        o
    }
}

pub unsafe fn options_default(
    oo: *mut options,
    oe: *const options_table_entry,
) -> *mut options_entry {
    unsafe {
        let o = options_empty(oo, oe);
        let ov = &raw mut (*o).value;

        if (*oe).flags & OPTIONS_TABLE_IS_ARRAY != 0 {
            if (*oe).default_arr.is_null() {
                _ = options_array_assign(o, (*oe).default_str.unwrap());
                return o;
            }
            let mut i = 0usize;
            while !(*(*oe).default_arr.add(i)).is_null() {
                _ = options_array_set(
                    o,
                    i as u32,
                    Some(cstr_to_str(*(*oe).default_arr.add(i))),
                    false,
                );
                i += 1;
            }
            return o;
        }

        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                (*ov).string = xstrdup___((*oe).default_str);
            }
            _ => {
                (*ov).number = (*oe).default_num;
            }
        }
        o
    }
}

pub unsafe fn options_default_to_string(oe: *const options_table_entry) -> NonNull<u8> {
    unsafe {
        match (*oe).type_ {
            options_table_type::OPTIONS_TABLE_STRING
            | options_table_type::OPTIONS_TABLE_COMMAND => {
                NonNull::new_unchecked(xstrdup___((*oe).default_str))
            }
            options_table_type::OPTIONS_TABLE_NUMBER => {
                NonNull::new(format_nul!("{}", (*oe).default_num)).unwrap()
            }
            options_table_type::OPTIONS_TABLE_KEY => {
                xstrdup(key_string_lookup_key((*oe).default_num as u64, 0))
            }
            options_table_type::OPTIONS_TABLE_COLOUR => NonNull::new(
                CString::new(colour_tostring((*oe).default_num as i32).into_owned())
                    .unwrap()
                    .into_raw()
                    .cast(),
            )
            .unwrap(),
            options_table_type::OPTIONS_TABLE_FLAG => xstrdup_(if (*oe).default_num != 0 {
                c"on"
            } else {
                c"off"
            }),
            options_table_type::OPTIONS_TABLE_CHOICE => {
                NonNull::new(xstrdup__((*oe).choices[(*oe).default_num as usize])).unwrap()
            }
        }
    }
}

unsafe fn options_add(oo: *mut options, name: &str) -> *mut options_entry {
    unsafe {
        // Remove existing entry if present
        if !options_get_only(oo, name).is_null() {
            options_remove_by_name(oo, name);
        }

        let entry = Box::new(options_entry {
            owner: oo,
            name: Cow::Owned(name.to_string()),
            tableentry: null(),
            value: options_value { number: 0 },
            cached: 0,
            style: zeroed(),
        });

        let key = name.to_string();
        (*oo).tree.insert(key.clone(), entry);
        &mut **(*oo).tree.get_mut(&key).unwrap() as *mut options_entry
    }
}

/// Remove an entry by name from the options tree.
unsafe fn options_remove_by_name(oo: *mut options, name: &str) {
    unsafe {
        // Try direct name first, then mapped name
        let key = if (*oo).tree.contains_key(name) {
            name.to_string()
        } else {
            let mapped = options_map_name_str(name);
            if (*oo).tree.contains_key(mapped) {
                mapped.to_string()
            } else {
                return;
            }
        };

        if let Some(mut entry) = (*oo).tree.remove(&key) {
            if options_is_array(&mut *entry as *mut options_entry) {
                options_array_clear(&mut *entry as *mut options_entry);
            } else {
                options_value_free(&*entry, &mut entry.value);
            }
        }
    }
}

unsafe fn options_remove(o: *mut options_entry) {
    unsafe {
        let oo = (*o).owner;
        let name = (*o).name.to_string();
        options_remove_by_name(oo, &name);
    }
}

pub unsafe fn options_name<'a>(o: *mut options_entry) -> &'a str {
    unsafe { &(*o).name }
}

pub unsafe fn options_owner(o: *mut options_entry) -> *mut options {
    unsafe { (*o).owner }
}

pub unsafe fn options_table_entry(o: *mut options_entry) -> *const options_table_entry {
    unsafe { (*o).tableentry }
}

/// Get the array map from an options_entry, creating it if null.
unsafe fn options_array_map(o: *mut options_entry) -> &'static mut options_array {
    unsafe {
        if (*o).value.array.is_null() {
            (*o).value.array = Box::into_raw(Box::new(BTreeMap::new()));
        }
        &mut *(*o).value.array
    }
}

unsafe fn options_array_item(o: *mut options_entry, idx: c_uint) -> *mut options_array_item {
    unsafe {
        let map = options_array_map(o);
        map.get_mut(&idx)
            .map_or(null_mut(), |a| a as *mut options_array_item)
    }
}

unsafe fn options_array_new(o: *mut options_entry, idx: c_uint) -> *mut options_array_item {
    unsafe {
        let map = options_array_map(o);
        map.entry(idx).or_insert(options_array_item {
            index: idx,
            value: options_value { number: 0 },
        });
        &mut *map.get_mut(&idx).unwrap() as *mut options_array_item
    }
}

unsafe fn options_array_free(o: *mut options_entry, a: *mut options_array_item) {
    unsafe {
        options_value_free(o, &mut (*a).value);
        let map = options_array_map(o);
        map.remove(&(*a).index);
    }
}

pub unsafe fn options_array_clear(o: *mut options_entry) {
    unsafe {
        if !options_is_array(o) {
            return;
        }
        let map = options_array_map(o);
        let indices: Vec<u32> = map.keys().copied().collect();
        for idx in indices {
            if let Some(mut item) = map.remove(&idx) {
                options_value_free(o, &mut item.value);
            }
        }
    }
}

pub unsafe fn options_array_get(o: *mut options_entry, idx: u32) -> *mut options_value {
    unsafe {
        if !options_is_array(o) {
            return null_mut();
        }
        let a = options_array_item(o, idx);
        if a.is_null() {
            return null_mut();
        }
        &raw mut (*a).value
    }
}

pub unsafe fn options_array_set(
    o: *mut options_entry,
    idx: u32,
    value: Option<&str>,
    append: bool,
) -> Result<(), CString> {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            return Err(CString::new("not an array").unwrap());
        }

        let Some(value) = value else {
            let a = options_array_item(o, idx);
            if !a.is_null() {
                options_array_free(o, a);
            }
            return Ok(());
        };

        if OPTIONS_IS_COMMAND(o) {
            let cmdlist = match cmd_parse_from_string(value, None) {
                Err(error) => {
                    return Err(error);
                }
                Ok(cmdlist) => cmdlist,
            };

            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.cmdlist = cmdlist;
            return Ok(());
        }

        if OPTIONS_IS_STRING(o) {
            let mut a = options_array_item(o, idx);
            let new = if !a.is_null() && append {
                format_nul!("{}{}", _s((*a).value.string), value)
            } else {
                xstrdup__(value)
            };

            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &mut (*a).value);
            }
            (*a).value.string = new;
            return Ok(());
        }

        if !(*o).tableentry.is_null()
            && (*(*o).tableentry).type_ == options_table_type::OPTIONS_TABLE_COLOUR
        {
            let number = colour_fromstring(value);
            if number == -1 {
                return Err(CString::new(format!("bad colour: {value}")).unwrap());
            }
            let mut a = options_array_item(o, idx);
            if a.is_null() {
                a = options_array_new(o, idx);
            } else {
                options_value_free(o, &raw mut (*a).value);
            }
            (*a).value.number = number as i64;
            return Ok(());
        }

        Err(CString::new("wrong array type").unwrap())
    }
}

// note one difference was that this function previously could avoid allocation on error
pub unsafe fn options_array_assign(o: *mut options_entry, s: &str) -> Result<(), CString> {
    unsafe {
        let mut separator = (*(*o).tableentry).separator;
        if separator.is_null() {
            separator = c!(" ,");
        }
        if *separator == 0 {
            if s.is_empty() {
                return Ok(());
            }
            let mut i = 0;
            while i < u32::MAX {
                if options_array_item(o, i).is_null() {
                    break;
                }
                i += 1;
            }
            return options_array_set(o, i, Some(s), false);
        }

        if s.is_empty() {
            return Ok(());
        }
        let copy = xstrdup__(s);
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
            if let Err(cause) = options_array_set(o, i, Some(cstr_to_str(next)), false) {
                free_(copy);
                return Err(cause);
            }
        }
        free_(copy);
        Ok(())
    }
}

/// Collect all array item pointers in index order.
pub unsafe fn options_array_items(o: *mut options_entry) -> Vec<*mut options_array_item> {
    unsafe {
        if !OPTIONS_IS_ARRAY(o) {
            return Vec::new();
        }
        let map = options_array_map(o);
        map.values_mut()
            .map(|a| a as *mut options_array_item)
            .collect()
    }
}


pub unsafe fn options_array_item_index(a: *mut options_array_item) -> u32 {
    unsafe { (*a).index }
}

pub unsafe fn options_array_item_value(a: *mut options_array_item) -> *mut options_value {
    unsafe { &raw mut (*a).value }
}

pub unsafe fn options_is_array(o: *mut options_entry) -> bool {
    unsafe { OPTIONS_IS_ARRAY(o) }
}

pub unsafe fn options_is_string(o: *mut options_entry) -> bool {
    unsafe { OPTIONS_IS_STRING(o) }
}

pub unsafe fn options_to_string(o: *mut options_entry, idx: i32, numeric: i32) -> *mut u8 {
    unsafe {
        if OPTIONS_IS_ARRAY(o) {
            if idx == -1 {
                let mut result = null_mut();
                let mut last: *mut u8 = null_mut();

                for a in options_array_items(o) {
                    let next = options_value_to_string(
                        o,
                        &raw mut (*a).value,
                        numeric,
                    );

                    if last.is_null() {
                        result = next;
                    } else {
                        let new_result = format_nul!("{} {}", _s(last), _s(next));
                        free_(last);
                        free_(next);
                        result = new_result;
                    }
                    last = result;
                }

                if result.is_null() {
                    return xstrdup(c!("")).as_ptr();
                }
                return result;
            }

            let a = options_array_item(o, idx as u32);
            if a.is_null() {
                return xstrdup(c!("")).as_ptr();
            }
            return options_value_to_string(o, &raw mut (*a).value, numeric);
        }

        options_value_to_string(o, &raw mut (*o).value, numeric)
    }
}

pub fn options_parse(name: &str) -> Option<(String, i32)> {
    if name.is_empty() {
        return None;
    }

    let mut copy = name.to_string();

    let Some(cp) = copy.find('[') else {
        return Some((copy, -1));
    };

    let end = copy[cp+1..].find(']').map(|end| end + cp + 1)?;

    if end != copy.len() - 1 || !copy.as_bytes()[end - 1].is_ascii_digit() {
        return None;
    }

    let Ok(parsed_idx) = copy[cp+1..end].parse::<i32>() else {
        return None;
    };

    copy.truncate(cp);
    Some((copy, parsed_idx))
}

pub unsafe fn options_parse_get(
    oo: *mut options,
    s: &str,
    idx: *mut i32,
    only: i32,
) -> *mut options_entry {
    unsafe {
        let Some((name, idx_value)) = options_parse(s) else {
            return null_mut();
        };
        *idx = idx_value;

        if only != 0 {
            options_get_only(oo, &name)
        } else {
            options_get(&mut *oo, &name)
        }
    }
}

pub unsafe fn options_match(s: &str, idx: *mut i32, ambiguous: *mut i32) -> Option<String> {
    unsafe {
        let (parsed, idx_value) = options_parse(s)?;
        *idx = idx_value;

        if parsed.starts_with('@') {
            *ambiguous = 0;
            return Some(parsed);
        }

        let name = options_map_name(&parsed).unwrap_or(&parsed);

        let mut found: *const options_table_entry = null();

        for oe in &OPTIONS_TABLE {
            if oe.name == name {
                found = oe;
                break;
            }
            if oe.name.starts_with(name) {
                if !found.is_null() {
                    *ambiguous = 1;
                    return None;
                }
                found = oe;
            }
        }

        if found.is_null() {
            *ambiguous = 0;
            return None;
        }

        Some((*found).name.to_string())
    }
}

#[expect(dead_code)]
unsafe fn options_match_get(
    oo: *mut options,
    s: &str,
    idx: *mut i32,
    only: i32,
    ambiguous: *mut i32,
) -> *mut options_entry {
    unsafe {
        let Some(name) = options_match(s, idx, ambiguous) else {
            return null_mut();
        };

        *ambiguous = 0;
        if only != 0 {
            options_get_only(oo, &name)
        } else {
            options_get(&mut *oo, &name)
        }
    }
}

pub unsafe fn options_get_string(oo: *mut options, name: &str) -> *const u8 {
    unsafe {
        let o = options_get(&mut *oo, name);
        if o.is_null() {
            fatalx_!("missing option {name}");
        }
        if !OPTIONS_IS_STRING(o) {
            fatalx_!("option {name} is not a string");
        }
        (*o).value.string
    }
}

pub unsafe fn options_get_string_(oo: *const options, name: &str) -> *const u8 {
    unsafe {
        let o = options_get_const(oo, name);
        if o.is_null() {
            fatalx_!("missing option {name}");
        }
        if !OPTIONS_IS_STRING(o) {
            fatalx_!("option {name} is not a string");
        }
        (*o).value.string
    }
}

unsafe fn options_get_number(oo: *mut options, name: &str) -> i64 {
    unsafe {
        let o = options_get(&mut *oo, name);
        if o.is_null() {
            fatalx_!("missing option {name}");
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_!("option {name} is not a number");
        }
        (*o).value.number
    }
}

pub unsafe fn options_get_number_(oo: *const options, name: &str) -> i64 {
    unsafe {
        let o = options_get_const(oo, name);
        if o.is_null() {
            fatalx_!("missing option {name}");
        }
        if !OPTIONS_IS_NUMBER(o) {
            fatalx_!("option {name} is not a number");
        }
        (*o).value.number
    }
}

/// panics if internally stored value is out of range of returned type
#[track_caller]
pub fn options_get_number___<T: TryFrom<i64>>(oo: &options, name: &str) -> T {
    unsafe {
        let o = options_get_const(oo, name);
        if o.is_null() {
            panic!("missing option {name}");
        }
        if !OPTIONS_IS_NUMBER(o) {
            panic!("option {name} is not a number");
        }

        match T::try_from((*o).value.number) {
            Ok(value) => value,
            Err(_) => panic!("options_get_number out of range"),
        }
    }
}

macro_rules! options_set_string {
   ($oo:expr, $name:expr, $append:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::options_::options_set_string_($oo, $name, $append, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use options_set_string;

pub unsafe fn options_set_string_(
    oo: *mut options,
    name: &str,
    append: bool,
    args: std::fmt::Arguments,
) -> *mut options_entry {
    unsafe {
        let mut separator = c!("");
        let value: *mut u8;

        let mut s = args.to_string();
        s.push('\0');
        let s = s.leak().as_mut_ptr().cast();

        let mut o = options_get_only(oo, name);
        if !o.is_null() && append && OPTIONS_IS_STRING(o) {
            if !name.starts_with('@') {
                separator = (*(*o).tableentry).separator;
                if separator.is_null() {
                    separator = c!("");
                }
            }
            value = format_nul!("{}{}{}", _s((*o).value.string), _s(separator), _s(s),);
            free_(s);
        } else {
            value = s;
        }

        if o.is_null() && name.starts_with('@') {
            o = options_add(oo, name);
        } else if o.is_null() {
            o = options_default(oo, options_parent_table_entry(oo, name));
            if o.is_null() {
                return null_mut();
            }
        }

        if !OPTIONS_IS_STRING(o) {
            panic!("option {name} is not a string");
        }
        free_((*o).value.string);
        (*o).value.string = value;
        (*o).cached = 0;
        o
    }
}

pub unsafe fn options_set_number(
    oo: *mut options,
    name: &str,
    value: i64,
) -> *mut options_entry {
    unsafe {
        if name.starts_with('@') {
            panic!("user option {name} must be a string");
        }

        let mut o = options_get_only(oo, name);
        if o.is_null() {
            o = options_default(oo, options_parent_table_entry(oo, name));
            if o.is_null() {
                return null_mut();
            }
        }

        if !OPTIONS_IS_NUMBER(o) {
            panic!("option {name} is not a number");
        }
        (*o).value.number = value;
        o
    }
}

pub unsafe fn options_scope_from_name(
    args: *mut args,
    window: i32,
    name: &str,
    fs: *mut cmd_find_state,
    oo: *mut *mut options,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let s = (*fs).s.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');
        let mut scope = OPTIONS_TABLE_NONE;

        if name.starts_with('@') {
            return options_scope_from_flags(args, window, fs, oo, cause);
        }

        let Some(oe) = OPTIONS_TABLE.iter().find(|oe| oe.name == name) else {
            *cause = format_nul!("unknown option: {name}");
            return OPTIONS_TABLE_NONE;
        };

        const OPTIONS_TABLE_WINDOW_AND_PANE: i32 = OPTIONS_TABLE_WINDOW | OPTIONS_TABLE_PANE;
        match oe.scope {
            OPTIONS_TABLE_SERVER => {
                *oo = GLOBAL_OPTIONS;
                scope = OPTIONS_TABLE_SERVER;
            }
            OPTIONS_TABLE_SESSION => {
                if args_has(args, 'g') {
                    *oo = GLOBAL_S_OPTIONS;
                    scope = OPTIONS_TABLE_SESSION;
                } else if s.is_null() && !target.is_null() {
                    *cause = format_nul!("no such session: {}", _s(target));
                } else if s.is_null() {
                    *cause = format_nul!("no current session");
                } else {
                    *oo = (*s).options;
                    scope = OPTIONS_TABLE_SESSION;
                }
            }
            OPTIONS_TABLE_WINDOW_AND_PANE => {
                if args_has(args, 'p') {
                    if wp.is_null() && !target.is_null() {
                        *cause = format_nul!("no such pane: {}", _s(target));
                    } else if wp.is_null() {
                        *cause = format_nul!("no current pane");
                    } else {
                        *oo = (*wp).options;
                        scope = OPTIONS_TABLE_PANE;
                    }
                } else {
                    // FALLTHROUGH same as OPTIONS_TABLE_WINDOW case
                    if args_has(args, 'g') {
                        *oo = GLOBAL_W_OPTIONS;
                        scope = OPTIONS_TABLE_WINDOW;
                    } else if wl.is_null() && !target.is_null() {
                        *cause = format_nul!("no such window: {}", _s(target));
                    } else if wl.is_null() {
                        *cause = format_nul!("no current window");
                    } else {
                        *oo = (*(*wl).window).options;
                        scope = OPTIONS_TABLE_WINDOW;
                    }
                }
            }
            OPTIONS_TABLE_WINDOW => {
                if args_has(args, 'g') {
                    *oo = GLOBAL_W_OPTIONS;
                    scope = OPTIONS_TABLE_WINDOW;
                } else if wl.is_null() && !target.is_null() {
                    *cause = format_nul!("no such window: {}", _s(target));
                } else if wl.is_null() {
                    *cause = format_nul!("no current window");
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

pub unsafe fn options_scope_from_flags(
    args: *mut args,
    window: i32,
    fs: *mut cmd_find_state,
    oo: *mut *mut options,
    cause: *mut *mut u8,
) -> i32 {
    unsafe {
        let s = (*fs).s.and_then(|id| session_from_id(id)).unwrap_or(null_mut());
        let wl = (*fs).wl;
        let wp = (*fs).wp;
        let target = args_get_(args, 't');

        if args_has(args, 's') {
            *oo = GLOBAL_OPTIONS;
            return OPTIONS_TABLE_SERVER;
        }

        if args_has(args, 'p') {
            if wp.is_null() {
                if !target.is_null() {
                    *cause = format_nul!("no such pane: {}", _s(target));
                } else {
                    *cause = format_nul!("no current pane");
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*wp).options;
            OPTIONS_TABLE_PANE
        } else if window != 0 || args_has(args, 'w') {
            if args_has(args, 'g') {
                *oo = GLOBAL_W_OPTIONS;
                return OPTIONS_TABLE_WINDOW;
            }
            if wl.is_null() {
                if !target.is_null() {
                    *cause = format_nul!("no such window: {}", _s(target));
                } else {
                    *cause = format_nul!("no current window");
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*(*wl).window).options;
            OPTIONS_TABLE_WINDOW
        } else {
            if args_has(args, 'g') {
                *oo = GLOBAL_S_OPTIONS;
                return OPTIONS_TABLE_SESSION;
            }
            if s.is_null() {
                if !target.is_null() {
                    *cause = format_nul!("no such session: {}", _s(target));
                } else {
                    *cause = format_nul!("no current session");
                }
                return OPTIONS_TABLE_NONE;
            }
            *oo = (*s).options;
            OPTIONS_TABLE_SESSION
        }
    }
}

pub unsafe fn options_string_to_style(
    oo: *mut options,
    name: &str,
    ft: *mut format_tree,
) -> *mut style {
    let __func__ = c!("options_string_to_style");
    unsafe {
        let o = options_get(&mut *oo, name);
        if o.is_null() || !OPTIONS_IS_STRING(o) {
            return null_mut();
        }

        if (*o).cached != 0 {
            return &mut (*o).style;
        }
        let s = (*o).value.string;
        log_debug!("{}: {} is '{}'", _s(__func__), name, _s(s));

        style_set(&mut (*o).style, &GRID_DEFAULT_CELL);
        (*o).cached = cstr_to_str(s).contains("#{") as i32;

        if !ft.is_null() && (*o).cached == 0 {
            let expanded = format_expand(ft, s);
            if style_parse(&mut (*o).style, &GRID_DEFAULT_CELL, expanded) != 0 {
                free_(expanded);
                return null_mut();
            }
            free_(expanded);
        } else if style_parse(&mut (*o).style, &GRID_DEFAULT_CELL, s) != 0 {
            return null_mut();
        }
        &mut (*o).style
    }
}

unsafe fn options_from_string_check(
    oe: *const options_table_entry,
    value: *const u8,
) -> Result<(), CString> {
    unsafe {
        let mut sy: style = std::mem::zeroed();

        if oe.is_null() {
            return Ok(());
        }
        if (*oe).name == "default-shell" && !checkshell_(value) {
            return Err(CString::new(format!("not a suitable shell: {}", _s(value))).unwrap());
        }
        if !(*oe).pattern.is_null() && fnmatch((*oe).pattern, value, 0) != 0 {
            return Err(CString::new(format!("value is invalid: {}", _s(value))).unwrap());
        }
        if ((*oe).flags & OPTIONS_TABLE_IS_STYLE) != 0
            && !cstr_to_str(value).contains("#{")
            && style_parse(&mut sy, &GRID_DEFAULT_CELL, value) != 0
        {
            return Err(CString::new(format!("invalid style: {}", _s(value))).unwrap());
        }
        Ok(())
    }
}

unsafe fn options_from_string_flag(
    oo: *mut options,
    name: &str,
    value: *const u8,
) -> Result<(), CString> {
    unsafe {
        let flag = if value.is_null() || *value == 0 {
            options_get_number(oo, name) == 0
        } else if streq_(value, "1") || strcaseeq_(value, "on") || strcaseeq_(value, "yes") {
            true
        } else if streq_(value, "0") || strcaseeq_(value, "off") || strcaseeq_(value, "no") {
            false
        } else {
            return Err(CString::new(format!("bad value: {}", _s(value))).unwrap());
        };
        options_set_number(oo, name, flag as i64);
        Ok(())
    }
}

pub unsafe fn options_find_choice(
    oe: *const options_table_entry,
    value: *const u8,
) -> Result<i32, CString> {
    unsafe {
        let Some(choice) = (*oe).choices.iter().position(|&cp| streq_(value, cp)) else {
            return Err(CString::new(format!("unknown value: {}", _s(value))).unwrap());
        };
        Ok(choice as i32)
    }
}

unsafe fn options_from_string_choice(
    oe: *const options_table_entry,
    oo: *mut options,
    name: &str,
    value: *const u8,
) -> Result<(), CString> {
    unsafe {
        let choice = if value.is_null() {
            let mut choice = options_get_number(oo, name);
            #[expect(clippy::bool_to_int_with_if, reason = "more readable this way")]
            if choice < 2 {
                choice = if choice == 0 { 1 } else { 0 };
            }
            choice
        } else {
            options_find_choice(oe, value)? as i64
        };
        options_set_number(oo, name, choice);
        Ok(())
    }
}

pub unsafe fn options_from_string(
    oo: *mut options,
    oe: *const options_table_entry,
    name: &str,
    value: *const u8,
    append: bool,
) -> Result<(), CString> {
    unsafe {
        let new: *const u8;
        let old: *mut u8;
        let key: key_code;

        let type_: options_table_type = if !oe.is_null() {
            if value.is_null()
                && (*oe).type_ != options_table_type::OPTIONS_TABLE_FLAG
                && (*oe).type_ != options_table_type::OPTIONS_TABLE_CHOICE
            {
                return Err(CString::new("empty value").unwrap());
            }
            (*oe).type_
        } else {
            if !name.starts_with('@') {
                return Err(CString::new("bad option name").unwrap());
            }
            options_table_type::OPTIONS_TABLE_STRING
        };

        match type_ {
            options_table_type::OPTIONS_TABLE_STRING => {
                old = xstrdup(options_get_string(oo, name)).as_ptr();
                options_set_string!(oo, name, append, "{}", _s(value));

                new = options_get_string(oo, name);
                if let Err(err) = options_from_string_check(oe, new) {
                    options_set_string!(oo, name, false, "{}", _s(old));
                    free_(old);
                    return Err(err);
                }
                free_(old);
                return Ok(());
            }

            options_table_type::OPTIONS_TABLE_NUMBER => {
                match strtonum(value, (*oe).minimum as i64, (*oe).maximum as i64) {
                    Ok(number) => {
                        options_set_number(oo, name, number);
                        return Ok(());
                    }
                    Err(errstr) => {
                        return Err(CString::new(format!(
                            "value is {}: {}",
                            _s(errstr.as_ptr()),
                            _s(value)
                        ))
                        .unwrap());
                    }
                }
            }

            options_table_type::OPTIONS_TABLE_KEY => {
                key = key_string_lookup_string(value);
                if key == KEYC_UNKNOWN {
                    return Err(CString::new(format!("bad key: {}", _s(value))).unwrap());
                }
                options_set_number(oo, name, key as i64);
                return Ok(());
            }

            options_table_type::OPTIONS_TABLE_COLOUR => {
                let number = colour_fromstring(cstr_to_str(value)) as i64;
                if number == -1 {
                    return Err(CString::new(format!("bad colour: {}", _s(value))).unwrap());
                }
                options_set_number(oo, name, number);
                return Ok(());
            }

            options_table_type::OPTIONS_TABLE_FLAG => {
                return options_from_string_flag(oo, name, value);
            }

            options_table_type::OPTIONS_TABLE_CHOICE => {
                return options_from_string_choice(oe, oo, name, value);
            }

            options_table_type::OPTIONS_TABLE_COMMAND => {}
        }

        Err(CString::new("").unwrap())
    }
}

pub unsafe fn options_push_changes(name: &str) {
    let __func__ = c!("options_push_changes");
    unsafe {
        log_debug!("{}: {}", _s(__func__), name);

        if name == "automatic-rename" {
            for w in (*(&raw mut WINDOWS)).values().copied() {
                if (*w).active.is_null() {
                    continue;
                }
                if options_get_number((*w).options, name) != 0 {
                    (*(*w).active).flags |= window_pane_flags::PANE_CHANGED;
                }
            }
        }

        if name == "cursor-colour" {
            for wp in (*(&raw mut ALL_WINDOW_PANES)).values().map(|wp| NonNull::new(*wp).unwrap()) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if name == "cursor-style" {
            for wp in (*(&raw mut ALL_WINDOW_PANES)).values().map(|wp| NonNull::new(*wp).unwrap()) {
                window_pane_default_cursor(wp.as_ptr());
            }
        }

        if name == "fill-character" {
            for w in (*(&raw mut WINDOWS)).values().map(|w| NonNull::new(*w).unwrap()) {
                window_set_fill_character(w);
            }
        }

        if name == "key-table" {
            for loop_ in clients_iter() {
                server_client_set_key_table(loop_, null_mut());
            }
        }

        if name == "user-keys" {
            for loop_ in clients_iter() {
                if (*loop_).tty.flags.intersects(tty_flags::TTY_OPENED) {
                    tty_keys_build(&mut (*loop_).tty);
                }
            }
        }

        if name == "status" || name == "status-interval" {
            status_timer_start_all();
        }

        if name == "monitor-silence" {
            alerts_reset_all();
        }

        if name == "window-style" || name == "window-active-style" {
            for wp in (*(&raw mut ALL_WINDOW_PANES)).values().map(|wp| NonNull::new(*wp).unwrap()) {
                (*wp.as_ptr()).flags |= window_pane_flags::PANE_STYLECHANGED;
            }
        }

        if name == "pane-colours" {
            for wp in (*(&raw mut ALL_WINDOW_PANES)).values().copied() {
                colour_palette_from_option(Some(&mut (*wp).palette), (*wp).options);
            }
        }

        if name == "pane-border-status" {
            for w in (*(&raw mut WINDOWS)).values().map(|w| NonNull::new(*w).unwrap()) {
                layout_fix_panes(w.as_ptr(), null_mut());
            }
        }

        for s in sessions_iter() {
            status_update_cache(s);
        }

        recalculate_sizes();

        for loop_ in clients_iter() {
            if !client_get_session(loop_).is_null() {
                server_redraw_client(loop_);
            }
        }
    }
}

#[cfg(test)]
#[allow(dangerous_implicit_autorefs, unsafe_op_in_unsafe_fn)]
mod tests {
    use super::*;
    use crate::options_table::OPTIONS_TABLE;
    use std::sync::Mutex;

    /// Mutex to serialize tests that access global options state.
    static OPTIONS_LOCK: Mutex<()> = Mutex::new(());

    /// Create a standalone options tree with global options as parent.
    /// Caller must hold OPTIONS_LOCK.
    unsafe fn init_globals() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| unsafe {
            use crate::tmux::{GLOBAL_OPTIONS, GLOBAL_S_OPTIONS, GLOBAL_W_OPTIONS};
            GLOBAL_OPTIONS = options_create(null_mut());
            GLOBAL_S_OPTIONS = options_create(null_mut());
            GLOBAL_W_OPTIONS = options_create(null_mut());
            for oe in &OPTIONS_TABLE {
                if oe.scope & OPTIONS_TABLE_SERVER != 0 {
                    options_default(GLOBAL_OPTIONS, oe);
                }
                if oe.scope & OPTIONS_TABLE_SESSION != 0 {
                    options_default(GLOBAL_S_OPTIONS, oe);
                }
                if oe.scope & OPTIONS_TABLE_WINDOW != 0 {
                    options_default(GLOBAL_W_OPTIONS, oe);
                }
            }
        });
    }

    // ---------------------------------------------------------------
    // options_parse — pure function, no global state needed
    // ---------------------------------------------------------------

    #[test]
    fn parse_simple_name() {
        assert_eq!(options_parse("status"), Some(("status".to_string(), -1)));
    }

    #[test]
    fn parse_name_with_index() {
        assert_eq!(
            options_parse("command-alias[0]"),
            Some(("command-alias".to_string(), 0))
        );
    }

    #[test]
    fn parse_name_with_large_index() {
        assert_eq!(
            options_parse("command-alias[42]"),
            Some(("command-alias".to_string(), 42))
        );
    }

    #[test]
    fn parse_empty_name() {
        assert_eq!(options_parse(""), None);
    }

    #[test]
    fn parse_no_closing_bracket() {
        assert_eq!(options_parse("foo[3"), None);
    }

    #[test]
    fn parse_bracket_not_at_end() {
        assert_eq!(options_parse("foo[3]bar"), None);
    }

    #[test]
    fn parse_user_option() {
        assert_eq!(
            options_parse("@my-option"),
            Some(("@my-option".to_string(), -1))
        );
    }

    #[test]
    fn parse_user_option_with_index() {
        assert_eq!(
            options_parse("@my-option[5]"),
            Some(("@my-option".to_string(), 5))
        );
    }

    // ---------------------------------------------------------------
    // options_create / options_free — lifecycle
    // ---------------------------------------------------------------

    #[test]
    fn create_and_free() {
        unsafe {
            let oo = options_create(null_mut());
            assert!(!oo.is_null());
            assert!((*oo).parent.is_null());
            assert!((*oo).tree.is_empty());
            options_free(oo);
        }
    }

    #[test]
    fn create_with_parent() {
        unsafe {
            let parent = options_create(null_mut());
            let child = options_create(parent);
            assert_eq!((*child).parent, parent);
            options_free(child);
            options_free(parent);
        }
    }

    // ---------------------------------------------------------------
    // options_default — create option with default value from table
    // ---------------------------------------------------------------

    #[test]
    fn default_string_option() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());

            // Find a string option from the table (e.g., "default-shell")
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "default-terminal")
                .unwrap();
            assert_eq!(oe.type_, options_table_type::OPTIONS_TABLE_STRING);

            let o = options_default(oo, oe);
            assert!(!o.is_null());
            assert!(OPTIONS_IS_STRING(o));

            options_free(oo);
        }
    }

    #[test]
    fn default_number_option() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());

            // "base-index" is a number option, default 0
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "base-index")
                .unwrap();
            assert_eq!(oe.type_, options_table_type::OPTIONS_TABLE_NUMBER);

            let o = options_default(oo, oe);
            assert!(!o.is_null());
            assert!(OPTIONS_IS_NUMBER(o));
            assert_eq!((*o).value.number, 0);

            options_free(oo);
        }
    }

    #[test]
    fn default_flag_option() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());

            // "mouse" is a flag option, default off (0)
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "mouse")
                .unwrap();
            assert_eq!(oe.type_, options_table_type::OPTIONS_TABLE_FLAG);

            let o = options_default(oo, oe);
            assert!(!o.is_null());
            assert_eq!((*o).value.number, 0);

            options_free(oo);
        }
    }

    #[test]
    fn default_choice_option() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());

            // "status" is a choice option (off, on, 2, 3, 4, 5), default "on" (index 1)
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "status")
                .unwrap();
            assert_eq!(oe.type_, options_table_type::OPTIONS_TABLE_CHOICE);

            let o = options_default(oo, oe);
            assert!(!o.is_null());
            assert_eq!((*o).value.number, oe.default_num);

            options_free(oo);
        }
    }

    #[test]
    fn default_colour_option() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());

            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "display-panes-colour")
                .unwrap();
            assert_eq!(oe.type_, options_table_type::OPTIONS_TABLE_COLOUR);

            let o = options_default(oo, oe);
            assert!(!o.is_null());
            assert_eq!((*o).value.number, oe.default_num);

            options_free(oo);
        }
    }

    // ---------------------------------------------------------------
    // options_get / options_set — get/set with parent lookup
    // ---------------------------------------------------------------

    #[test]
    fn set_and_get_number() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let parent = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "base-index")
                .unwrap();
            options_default(parent, oe);

            // Parent has default (0), set child to 1
            let child = options_create(parent);
            options_default(child, oe);
            options_set_number(child, "base-index", 1);

            assert_eq!(options_get_number_(child, "base-index"), 1);
            assert_eq!(options_get_number_(parent, "base-index"), 0);

            options_free(child);
            options_free(parent);
        }
    }

    #[test]
    fn get_falls_through_to_parent() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let parent = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "base-index")
                .unwrap();
            options_default(parent, oe);
            options_set_number(parent, "base-index", 7);

            // Child has no "base-index" — should fall through to parent
            let child = options_create(parent);
            assert_eq!(options_get_number_(child, "base-index"), 7);

            options_free(child);
            options_free(parent);
        }
    }

    #[test]
    fn set_and_get_user_string_option() {
        unsafe {
            let oo = options_create(null_mut());
            options_set_string!(oo, "@my-var", false, "hello");

            let val = options_get_string(oo, "@my-var");
            assert!(!val.is_null());
            assert_eq!(cstr_to_str(val), "hello");

            options_free(oo);
        }
    }

    #[test]
    fn set_string_append() {
        unsafe {
            let oo = options_create(null_mut());
            options_set_string!(oo, "@test", false, "hello");
            options_set_string!(oo, "@test", true, " world");

            let val = options_get_string(oo, "@test");
            assert_eq!(cstr_to_str(val), "hello world");

            options_free(oo);
        }
    }

    // ---------------------------------------------------------------
    // options_to_string — value formatting
    // ---------------------------------------------------------------

    #[test]
    fn to_string_number() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "base-index")
                .unwrap();
            options_default(oo, oe);
            options_set_number(oo, "base-index", 42);

            let o = options_get_only(oo, "base-index");
            let s = options_to_string(o, -1, 0);
            assert_eq!(cstr_to_str(s), "42");
            free_(s);

            options_free(oo);
        }
    }

    #[test]
    fn to_string_flag() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "mouse")
                .unwrap();
            options_default(oo, oe);

            let o = options_get_only(oo, "mouse");

            // Default is off
            let s = options_to_string(o, -1, 0);
            assert_eq!(cstr_to_str(s), "off");
            free_(s);

            // Set to on
            options_set_number(oo, "mouse", 1);
            let s = options_to_string(o, -1, 0);
            assert_eq!(cstr_to_str(s), "on");
            free_(s);

            // Numeric mode
            let s = options_to_string(o, -1, 1);
            assert_eq!(cstr_to_str(s), "1");
            free_(s);

            options_free(oo);
        }
    }

    #[test]
    fn to_string_choice() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "status")
                .unwrap();
            options_default(oo, oe);

            let o = options_get_only(oo, "status");
            let s = options_to_string(o, -1, 0);
            // Default "status" is "on" (index 1)
            assert_eq!(cstr_to_str(s), "on");
            free_(s);

            options_free(oo);
        }
    }

    #[test]
    fn to_string_user_string() {
        unsafe {
            let oo = options_create(null_mut());
            options_set_string!(oo, "@foo", false, "bar baz");

            let o = options_get_only(oo, "@foo");
            let s = options_to_string(o, -1, 0);
            assert_eq!(cstr_to_str(s), "bar baz");
            free_(s);

            options_free(oo);
        }
    }

    // ---------------------------------------------------------------
    // options_from_string — parse typed values
    // ---------------------------------------------------------------

    #[test]
    fn from_string_flag_on() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "mouse")
                .unwrap();
            options_default(oo, oe);

            assert!(options_from_string(oo, oe, "mouse", c!("on"), false).is_ok());
            assert_eq!(options_get_number_(oo, "mouse"), 1);

            assert!(options_from_string(oo, oe, "mouse", c!("off"), false).is_ok());
            assert_eq!(options_get_number_(oo, "mouse"), 0);

            options_free(oo);
        }
    }

    #[test]
    fn from_string_flag_toggle() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "mouse")
                .unwrap();
            options_default(oo, oe);

            // Toggle: null value flips the flag
            assert_eq!(options_get_number_(oo, "mouse"), 0);
            assert!(options_from_string(oo, oe, "mouse", null(), false).is_ok());
            assert_eq!(options_get_number_(oo, "mouse"), 1);
            assert!(options_from_string(oo, oe, "mouse", null(), false).is_ok());
            assert_eq!(options_get_number_(oo, "mouse"), 0);

            options_free(oo);
        }
    }

    #[test]
    fn from_string_flag_bad_value() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "mouse")
                .unwrap();
            options_default(oo, oe);

            let result = options_from_string(oo, oe, "mouse", c!("maybe"), false);
            assert!(result.is_err());

            options_free(oo);
        }
    }

    #[test]
    fn from_string_number() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "base-index")
                .unwrap();
            options_default(oo, oe);

            assert!(options_from_string(oo, oe, "base-index", c!("5"), false).is_ok());
            assert_eq!(options_get_number_(oo, "base-index"), 5);

            options_free(oo);
        }
    }

    #[test]
    fn from_string_number_out_of_range() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "base-index")
                .unwrap();
            options_default(oo, oe);

            // base-index max is i32::MAX — test exceeding it
            let result = options_from_string(oo, oe, "base-index", c!("-1"), false);
            assert!(result.is_err(), "negative value should be rejected (min=0)");

            options_free(oo);
        }
    }

    #[test]
    fn from_string_choice() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "status")
                .unwrap();
            options_default(oo, oe);

            assert!(options_from_string(oo, oe, "status", c!("off"), false).is_ok());
            assert_eq!(options_get_number_(oo, "status"), 0);

            assert!(options_from_string(oo, oe, "status", c!("on"), false).is_ok());
            assert_eq!(options_get_number_(oo, "status"), 1);

            let result = options_from_string(oo, oe, "status", c!("invalid"), false);
            assert!(result.is_err());

            options_free(oo);
        }
    }

    #[test]
    fn from_string_colour() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let oo = options_create(null_mut());
            let oe = OPTIONS_TABLE
                .iter()
                .find(|oe| oe.name == "display-panes-colour")
                .unwrap();
            options_default(oo, oe);

            assert!(options_from_string(oo, oe, "display-panes-colour", c!("red"), false).is_ok());

            let result =
                options_from_string(oo, oe, "display-panes-colour", c!("notacolour"), false);
            assert!(result.is_err());

            options_free(oo);
        }
    }

    // ---------------------------------------------------------------
    // options_match — prefix matching
    // ---------------------------------------------------------------

    #[test]
    fn match_exact() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let mut idx = 0i32;
            let mut ambiguous = 0i32;
            let result = options_match("status", &raw mut idx, &raw mut ambiguous);
            assert_eq!(result.as_deref(), Some("status"));
            assert_eq!(idx, -1);
            assert_eq!(ambiguous, 0);
        }
    }

    #[test]
    fn match_prefix() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let mut idx = 0i32;
            let mut ambiguous = 0i32;
            // "base-i" should match "base-index" uniquely
            let result = options_match("base-i", &raw mut idx, &raw mut ambiguous);
            assert_eq!(result.as_deref(), Some("base-index"));
        }
    }

    #[test]
    fn match_ambiguous() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let mut idx = 0i32;
            let mut ambiguous = 0i32;
            // "status-" matches multiple options (status-style, status-position, etc.)
            let result = options_match("status-", &raw mut idx, &raw mut ambiguous);
            assert!(result.is_none());
            assert_eq!(ambiguous, 1);
        }
    }

    #[test]
    fn match_unknown() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let mut idx = 0i32;
            let mut ambiguous = 0i32;
            let result =
                options_match("nonexistent-option", &raw mut idx, &raw mut ambiguous);
            assert!(result.is_none());
            assert_eq!(ambiguous, 0);
        }
    }

    #[test]
    fn match_user_option() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let mut idx = 0i32;
            let mut ambiguous = 0i32;
            // User options (@-prefixed) are returned as-is, no table lookup
            let result = options_match("@my-thing", &raw mut idx, &raw mut ambiguous);
            assert_eq!(result.as_deref(), Some("@my-thing"));
            assert_eq!(ambiguous, 0);
        }
    }

    #[test]
    fn match_with_index() {
        let _lock = OPTIONS_LOCK.lock().unwrap();
        unsafe {
            init_globals();
            let mut idx = 0i32;
            let mut ambiguous = 0i32;
            let result = options_match("command-alias[3]", &raw mut idx, &raw mut ambiguous);
            assert_eq!(result.as_deref(), Some("command-alias"));
            assert_eq!(idx, 3);
        }
    }

    // ---------------------------------------------------------------
    // options_map_name — name aliasing (color → colour)
    // ---------------------------------------------------------------

    #[test]
    fn map_name_color_to_colour() {
        assert_eq!(
            options_map_name("display-panes-color"),
            Some("display-panes-colour")
        );
    }

    #[test]
    fn map_name_no_mapping() {
        assert_eq!(options_map_name("status"), None);
    }

    #[test]
    fn map_name_cursor_color() {
        assert_eq!(options_map_name("cursor-color"), Some("cursor-colour"));
    }

    // ---------------------------------------------------------------
    // options_entries — iteration
    // ---------------------------------------------------------------

    #[test]
    fn entries_returns_sorted() {
        unsafe {
            let oo = options_create(null_mut());
            options_set_string!(oo, "@zebra", false, "z");
            options_set_string!(oo, "@alpha", false, "a");
            options_set_string!(oo, "@middle", false, "m");

            let entries = options_entries(oo);
            assert_eq!(entries.len(), 3);
            assert_eq!(options_name(entries[0]), "@alpha");
            assert_eq!(options_name(entries[1]), "@middle");
            assert_eq!(options_name(entries[2]), "@zebra");

            options_free(oo);
        }
    }

    // ---------------------------------------------------------------
    // options_get_only vs options_get — scope behavior
    // ---------------------------------------------------------------

    #[test]
    fn get_only_does_not_check_parent() {
        unsafe {
            let parent = options_create(null_mut());
            options_set_string!(parent, "@foo", false, "parent-value");

            let child = options_create(parent);
            // get_only should not find @foo in child
            let o = options_get_only(child, "@foo");
            assert!(o.is_null());

            // But get should find it via parent
            let o = options_get(&mut *child, "@foo");
            assert!(!o.is_null());
            assert_eq!(cstr_to_str((*o).value.string), "parent-value");

            options_free(child);
            options_free(parent);
        }
    }
}

// note one difference was that this function previously could avoid allocation on error
pub unsafe fn options_remove_or_default(o: *mut options_entry, idx: i32) -> Result<(), CString> {
    unsafe {
        let oo = (*o).owner;

        if idx == -1 {
            if !(*o).tableentry.is_null()
                && (oo == GLOBAL_OPTIONS || oo == GLOBAL_S_OPTIONS || oo == GLOBAL_W_OPTIONS)
            {
                options_default(oo, (*o).tableentry);
            } else {
                options_remove(o);
            }
        } else {
            options_array_set(o, idx as u32, None, false)?;
        }
        Ok(())
    }
}
