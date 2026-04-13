// Copyright (c) 2010 Nicholas Marriott <nicholas.marriott@gmail.com>
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
//! Command argument parsing and management.
//!
//! Implements getopt-style flag parsing for tmux commands. Each command defines
//! an [`args_parse`] template (e.g. `"bt:T:"`) specifying which flags it accepts
//! and whether they take arguments (`:` suffix, `::` for optional).
//!
//! An [`args`] set contains:
//! - **Flags**: stored in a `HashMap<u8, Box<args_entry>>`, keyed by flag character.
//!   Each entry tracks a count (for repeated flags like `-vvv`) and optional values.
//! - **Positional arguments**: a count + pointer to an array of [`args_value`].
//!
//! Values can be strings (`ARGS_STRING`), command blocks (`ARGS_COMMANDS`), or
//! none (`ARGS_NONE`). The `args_escape` function handles shell quoting for
//! display/serialization.

use std::collections::HashMap;
use std::ffi::CStr;

use crate::*;

const ARGS_ENTRY_OPTIONAL_VALUE: c_int = 1;
pub struct args_entry {
    pub flag: c_uchar,
    pub values: Vec<args_value>,
    pub count: c_uint,

    pub flags: c_int,
}

pub struct args {
    pub tree: HashMap<u8, Box<args_entry>>,
    pub values: Vec<args_value>,
}

pub struct args_command_state<'a> {
    pub cmdlist: *mut cmd_list,
    pub cmd: *mut u8,
    pub pi: cmd_parse_input<'a>,
}

pub unsafe fn args_find(args: *mut args, flag: c_uchar) -> *mut args_entry {
    unsafe {
        (*args)
            .tree
            .get_mut(&flag)
            .map_or(null_mut(), |e| &mut **e as *mut args_entry)
    }
}

pub fn args_find_ref(args: &args, flag: c_uchar) -> Option<&args_entry> {
    args.tree.get(&flag).map(|e| &**e)
}

pub fn args_type_to_string(value: &args_value) -> &'static str {
    match value {
        args_value::None => "NONE",
        args_value::String { .. } => "STRING",
        args_value::Commands { .. } => "COMMANDS",
    }
}

/// Return a value as a NUL-terminated C string pointer. For Commands variants
/// the printable form is computed lazily (on first access) and cached.
pub unsafe fn args_value_as_string(value: &args_value) -> *const u8 {
    unsafe {
        match value {
            args_value::None => c!(""),
            args_value::String { string } => string.as_ptr().cast(),
            args_value::Commands { cmdlist, cached } => cached
                .get_or_init(|| {
                    let p = cmd_list_print(&**cmdlist, 0);
                    let cs = CStr::from_ptr(p.cast()).to_owned();
                    free_(p);
                    cs
                })
                .as_ptr()
                .cast(),
        }
    }
}

impl args {
    fn create() -> Box<Self> {
        Box::new(Self {
            tree: HashMap::new(),
            values: Vec::new(),
        })
    }
}

pub fn args_create<'a>() -> &'a mut args {
    Box::leak(args::create())
}

pub unsafe fn args_parse_flag_argument(
    values: &[args_value],
    args: *mut args,
    i: &mut usize,
    string: *const u8,
    flag: i32,
    optional_argument: bool,
) -> Result<(), String> {
    unsafe {
        let value = if *string != b'\0' {
            args_value::new_string(xstrdup(string).as_ptr())
        } else {
            let argument = if *i >= values.len() {
                None
            } else {
                let arg = &values[*i];
                if !matches!(arg, args_value::String { .. }) {
                    return Err(format!("-{} argument must be a string", flag as u8 as char));
                }
                Some(arg)
            };

            let Some(argument) = argument else {
                if optional_argument {
                    log_debug!("{}: -{} (optional)", "args_parse_flag_argument", flag);
                    args_set(args, flag as c_uchar, None, ARGS_ENTRY_OPTIONAL_VALUE);
                    return Ok(());
                }
                return Err(format!("-{} expects an argument", flag as u8 as char));
            };

            *i += 1;
            argument.clone()
        };

        let s = args_value_as_string(&value);
        log_debug!("{}: -{} = {}", "args_parse_flag_argument", flag, _s(s));
        args_set(args, flag as c_uchar, Some(value), 0);
    }

    Ok(())
}

#[expect(clippy::needless_borrow, reason = "false positive")]
pub unsafe fn args_parse_flags(
    parse: *const args_parse,
    values: &[args_value],
    args: *mut args,
    i: &mut usize,
) -> Result<i32, Option<String>> {
    let __func__ = "args_parse_flags";
    unsafe {
        let value = &values[*i];
        let args_value::String { string: string_cs } = value else {
            return Ok(1);
        };

        let mut string = string_cs.as_ptr() as *const u8;
        log_debug!("{}: next {}", __func__, _s(string));
        if ({
            let tmp = *string != b'-';
            string = string.add(1);
            tmp
        }) || *string == b'\0'
        {
            return Ok(1);
        }
        *i += 1;
        if *string == b'-' && *string.add(1) == b'\0' {
            return Ok(1);
        }

        loop {
            let flag = *string as c_uchar;
            string = string.add(1);
            if flag == b'\0' {
                return Ok(0);
            }
            if flag == b'?' {
                return Err(None);
            }
            if !flag.is_ascii_alphanumeric() {
                return Err(Some(format!("invalid flag -{}", flag as char)));
            }

            let Some(found) = (*parse).template.bytes().position(|ch| ch == flag) else {
                return Err(Some(format!("unknown flag -{}", flag as char)));
            };
            if found + 1 >= (&(*parse).template).len() || (*parse).template.as_bytes()[found + 1] != b':' {
                log_debug!("{}: -{}", __func__, flag as char);
                args_set(args, flag, None, 0);
                continue;
            }
            let optional_argument = found + 2 < (&(*parse).template).len() && (*parse).template.as_bytes()[found + 2] == b':';
            return args_parse_flag_argument(
                values,
                args,
                i,
                string,
                flag as i32,
                optional_argument,
            )
            .map(|()| 0)
            .map_err(Some);
        }
    }
}

/// Parse arguments into a new argument set.
pub unsafe fn args_parse(
    parse: *const args_parse,
    values: &[args_value],
) -> Result<*mut args, Option<String>> {
    let __func__ = "args_parse";
    unsafe {
        let mut type_: args_parse_type;

        if values.is_empty() {
            return Ok(args_create());
        }

        let args = args_create();

        let mut i: usize = 1;
        while i < values.len() {
            match args_parse_flags(parse, values, args, &mut i) {
                Ok(1) => break,
                Ok(_) => {}
                Err(e) => {
                    args_free(args);
                    return Err(e);
                }
            }
        }
        log_debug!("{}: flags end at {} of {}", __func__, i, values.len());
        if i != values.len() {
            while i < values.len() {
                let value = &values[i];

                let s = args_value_as_string(value);
                log_debug!(
                    "{}: {} = {} (type {})",
                    __func__,
                    i,
                    _s(s),
                    args_type_to_string(value),
                );

                if let Some(cb) = (*parse).cb {
                    type_ = cb(args, args.values.len() as u32);
                    if type_ == args_parse_type::ARGS_PARSE_INVALID {
                        args_free(args);
                        return Err(None);
                    }
                } else {
                    type_ = args_parse_type::ARGS_PARSE_STRING;
                }

                match type_ {
                    args_parse_type::ARGS_PARSE_INVALID => fatalx("unexpected argument type"),
                    args_parse_type::ARGS_PARSE_STRING => {
                        if !matches!(value, args_value::String { .. }) {
                            let msg = format!(
                                "argument {} must be \"string\"",
                                args.values.len() + 1
                            );
                            args_free(args);
                            return Err(Some(msg));
                        }
                        args.values.push(value.clone());
                    }
                    args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING => {
                        args.values.push(value.clone());
                    }
                    args_parse_type::ARGS_PARSE_COMMANDS => {
                        if !matches!(value, args_value::Commands { .. }) {
                            let msg = format!(
                                "argument {} must be {{ commands }}",
                                args.values.len() + 1
                            );
                            args_free(args);
                            return Err(Some(msg));
                        }
                        args.values.push(value.clone());
                    }
                }
                i += 1;
            }
        }

        if (*parse).lower != -1 && (args.values.len() as i32) < (*parse).lower {
            let msg = format!("too few arguments (need at least {})", (*parse).lower);
            args_free(args);
            return Err(Some(msg));
        }
        if (*parse).upper != -1 && (args.values.len() as i32) > (*parse).upper {
            let msg = format!("too many arguments (need at most {})", (*parse).upper);
            args_free(args);
            return Err(Some(msg));
        }
        Ok(args)
    }
}

/// Copy an `args_value` while expanding `%N` template references.
unsafe fn args_copy_value_expanded(
    from: &args_value,
    argc: i32,
    argv: *mut *mut u8,
) -> args_value {
    unsafe {
        match from {
            args_value::None => args_value::None,
            args_value::String { string } => {
                let mut expanded = xstrdup(string.as_ptr().cast()).as_ptr();
                for i in 0..argc {
                    let s =
                        cmd_template_replace(expanded, cstr_to_str_(*argv.add(i as usize)), i + 1);
                    free_(expanded);
                    expanded = s;
                }
                args_value::new_string(expanded)
            }
            args_value::Commands { cmdlist, .. } => {
                args_value::new_commands(cmd_list_copy(&**cmdlist, argc, argv))
            }
        }
    }
}

/// Copy an arguments set.
pub unsafe fn args_copy(args: *mut args, argc: i32, argv: *mut *mut u8) -> *mut args {
    let __func__ = "args_copy";
    unsafe {
        cmd_log_argv!(argc, argv, "{__func__}");

        let new_args = args_create();
        for entry in (*args).tree.values() {
            if entry.values.is_empty() {
                for _ in 0..entry.count {
                    args_set(new_args, entry.flag, None, 0);
                }
                continue;
            }
            for value in &entry.values {
                let new_value = args_copy_value_expanded(value, argc, argv);
                args_set(new_args, entry.flag, Some(new_value), 0);
            }
        }
        for value in &(*args).values {
            new_args
                .values
                .push(args_copy_value_expanded(value, argc, argv));
        }

        new_args
    }
}

pub unsafe fn args_free(args: *mut args) {
    unsafe {
        // Drop the Box; args_value Drop impls will free CStrings and cmd_lists.
        drop(Box::from_raw(args));
    }
}

pub unsafe fn args_to_vector(args: *const args, argc: *mut i32, argv: *mut *mut *mut u8) {
    unsafe {
        *argc = 0;
        *argv = null_mut();

        for value in &(*args).values {
            match value {
                args_value::None => (),
                args_value::String { string } => {
                    cmd_append_argv(argc, argv, string.as_ptr().cast());
                }
                args_value::Commands { cmdlist, .. } => {
                    let s = cmd_list_print(&**cmdlist, 0);
                    cmd_append_argv(argc, argv, s);
                    free_(s);
                }
            }
        }
    }
}

pub unsafe fn args_from_vector(argc: i32, argv: *const *mut u8) -> Vec<args_value> {
    unsafe {
        let mut values = Vec::with_capacity(argc as usize);
        for i in 0..argc {
            values.push(args_value::new_string(xstrdup(*argv.add(i as usize)).as_ptr()));
        }
        values
    }
}

// TODO change this to use &mut String
macro_rules! args_print_add {
   ($buf:expr, $len:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::arguments::args_print_add_($buf, $len, format_args!($fmt $(, $args)*))
    };
}
pub unsafe fn args_print_add_(buf: *mut *mut u8, len: *mut usize, fmt: std::fmt::Arguments) {
    unsafe {
        let s = CString::new(fmt.to_string()).unwrap();

        *len += s.as_bytes().len();
        *buf = xrealloc(*buf as *mut c_void, *len).cast().as_ptr();

        strlcat(*buf, s.as_ptr().cast(), *len);
    }
}

pub unsafe fn args_print_add_value(buf: *mut *mut u8, len: *mut usize, value: &args_value) {
    unsafe {
        if **buf != b'\0' {
            args_print_add!(buf, len, " ");
        }

        match value {
            args_value::None => (),
            args_value::Commands { cmdlist, .. } => {
                let expanded = cmd_list_print(&**cmdlist, 0);
                args_print_add!(buf, len, "{{ {} }}", _s(expanded));
                free_(expanded);
            }
            args_value::String { string } => {
                let expanded = args_escape(string.as_ptr().cast());
                args_print_add!(buf, len, "{}", _s(expanded));
                free_(expanded);
            }
        }
    }
}

pub unsafe fn args_print(args: *mut args) -> *mut u8 {
    unsafe {
        let mut last: *mut args_entry = null_mut();

        let mut len: usize = 1;
        let mut buf: *mut u8 = xcalloc(1, len).cast().as_ptr();

        // Process the flags first.
        for entry in (*args).tree.values() {
            if entry.flags & ARGS_ENTRY_OPTIONAL_VALUE != 0 {
                continue;
            }
            if !entry.values.is_empty() {
                continue;
            }

            if *buf == b'\0' {
                args_print_add!(&raw mut buf, &raw mut len, "-");
            }
            for _ in 0..entry.count {
                args_print_add!(&raw mut buf, &raw mut len, "{}", entry.flag as char);
            }
        }

        // Then the flags with arguments.
        for entry in (*args).tree.values() {
            let entry_ptr = &**entry as *const args_entry as *mut args_entry;
            if entry.flags & ARGS_ENTRY_OPTIONAL_VALUE != 0 {
                if *buf != b'\0' {
                    args_print_add!(&raw mut buf, &raw mut len, " -{}", entry.flag as char);
                } else {
                    args_print_add!(&raw mut buf, &raw mut len, "-{}", entry.flag as char,);
                }
                last = entry_ptr;
                continue;
            }
            if entry.values.is_empty() {
                continue;
            }
            for value in &entry.values {
                if *buf != b'\0' {
                    args_print_add!(&raw mut buf, &raw mut len, " -{}", entry.flag as char,);
                } else {
                    args_print_add!(&raw mut buf, &raw mut len, "-{}", entry.flag as char,);
                }
                args_print_add_value(&raw mut buf, &raw mut len, value);
            }
            last = entry_ptr;
        }
        if !last.is_null() && ((*last).flags & ARGS_ENTRY_OPTIONAL_VALUE != 0) {
            args_print_add!(&raw mut buf, &raw mut len, " --");
        }

        // And finally the argument vector.
        for value in &(*args).values {
            args_print_add_value(&raw mut buf, &raw mut len, value);
        }

        buf
    }
}

/// Escape an argument.
pub unsafe fn args_escape(s: *const u8) -> *mut u8 {
    unsafe {
        let dquoted: *const u8 = c!(" #';${}%");
        let squoted: *const u8 = c!(" \"");

        let mut escaped: *mut u8 = null_mut();

        if *s == b'\0' {
            return format_nul!("''");
        }
        let quotes = if *s.add(libc::strcspn(s, dquoted)) != b'\0' {
            Some('"')
        } else if *s.add(libc::strcspn(s, squoted)) != b'\0' {
            Some('\'')
        } else {
            None
        };

        if *s != b' ' && *s.add(1) == b'\0' && (quotes.is_some() || *s == b'~') {
            escaped = format_nul!("\\{}", *s as char);
            return escaped;
        }

        let mut flags =
            vis_flags::VIS_OCTAL | vis_flags::VIS_CSTYLE | vis_flags::VIS_TAB | vis_flags::VIS_NL;
        if quotes == Some('"') {
            flags |= vis_flags::VIS_DQ;
        }
        utf8_stravis(&raw mut escaped, s, flags);

        let result = if quotes == Some('\'') {
            format_nul!("'{}'", _s(escaped))
        } else if quotes == Some('"') {
            if *escaped == b'~' {
                format_nul!("\"\\{}\"", _s(escaped))
            } else {
                format_nul!("\"{}\"", _s(escaped))
            }
        } else if *escaped == b'~' {
            format_nul!("\\{}", _s(escaped))
        } else {
            xstrdup(escaped).as_ptr()
        };
        free_(escaped);

        result
    }
}

// a better name for this might be args_count, but that name already exists
// so it would be confusing to use
pub unsafe fn args_has_count(args: *mut args, flag: u8) -> i32 {
    unsafe {
        match args_find_ref(&*args, flag) {
            Some(entry) => entry.count as i32,
            None => 0,
        }
    }
}

pub unsafe fn args_has(args: *mut args, flag: char) -> bool {
    debug_assert!(flag.is_ascii());
    unsafe {
        match args_find_ref(&*args, flag as u8) {
            Some(entry) => entry.count != 0,
            None => false,
        }
    }
}

pub fn args_set(args: *mut args, flag: c_uchar, value: Option<args_value>, flags: i32) {
    unsafe {
        let entry = (*args).tree.entry(flag).or_insert_with(|| {
            Box::new(args_entry {
                flag,
                values: Vec::new(),
                count: 0,
                flags,
            })
        });
        entry.count += 1;
        if let Some(value) = value
            && !matches!(value, args_value::None)
        {
            entry.values.push(value);
        }
    }
}

pub unsafe fn args_get(args: *mut args, flag: u8) -> *const u8 {
    unsafe {
        let entry = args_find(args, flag);

        if entry.is_null() {
            return null_mut();
        }
        match (*entry).values.last() {
            Some(args_value::String { string }) => string.as_ptr().cast(),
            _ => null_mut(),
        }
    }
}

/// Collect all entry pointers sorted by flag.
pub unsafe fn args_entry_list(args: *mut args) -> Vec<*mut args_entry> {
    unsafe {
        let mut entries: Vec<_> = (*args)
            .tree
            .values_mut()
            .map(|e| &mut **e as *mut args_entry)
            .collect();
        entries.sort_by_key(|e| (**e).flag);
        entries
    }
}

/// Get argument count.
pub unsafe fn args_count(args: *const args) -> u32 {
    unsafe { (*args).values.len() as u32 }
}

/// Get argument values as a slice.
pub unsafe fn args_values<'a>(args: *mut args) -> &'a [args_value] {
    unsafe { &(*args).values }
}

/// Get a single argument value by index, or `None` if out of range.
pub unsafe fn args_value<'a>(args: *mut args, idx: u32) -> Option<&'a args_value> {
    unsafe { (&(*args).values).get(idx as usize) }
}

/// Return argument as string. Returns null if `idx` is out of range.
pub unsafe fn args_string(args: *mut args, idx: u32) -> *const u8 {
    unsafe {
        match (&(*args).values).get(idx as usize) {
            Some(v) => args_value_as_string(v),
            None => null(),
        }
    }
}

/// Make a command now.
pub unsafe fn args_make_commands_now(
    self_: *mut cmd,
    item: *mut cmdq_item,
    idx: u32,
    expand: bool,
) -> *mut cmd_list {
    unsafe {
        let mut error = null_mut();
        let state = args_make_commands_prepare(self_, item, idx, null_mut(), false, expand);
        let cmdlist = args_make_commands(state, 0, null_mut(), &raw mut error);
        if cmdlist.is_null() {
            cmdq_error!(item, "{}", _s(error));
            free_(error);
        } else {
            (*cmdlist).references += 1;
        }
        args_make_commands_free(state);
        cmdlist
    }
}

/// Save bits to make a command later.
pub unsafe fn args_make_commands_prepare<'a>(
    self_: *mut cmd,
    item: *mut cmdq_item,
    idx: u32,
    default_command: *const u8,
    wait: bool,
    expand: bool,
) -> *mut args_command_state<'a> {
    let __func__ = "args_make_commands_prepare";
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let tc = cmdq_get_target_client(item);

        let state = xcalloc1::<args_command_state>() as *mut args_command_state;

        let cmd = if (idx as usize) < (*args).values.len() {
            let value = &(&(*args).values)[idx as usize];
            match value {
                args_value::Commands { cmdlist, .. } => {
                    (*state).cmdlist = *cmdlist;
                    (*(*state).cmdlist).references += 1;
                    return state;
                }
                args_value::String { string } => string.as_ptr() as *const u8,
                args_value::None => {
                    if default_command.is_null() {
                        fatalx("argument out of range");
                    }
                    default_command
                }
            }
        } else {
            if default_command.is_null() {
                fatalx("argument out of range");
            }
            default_command
        };

        if expand {
            (*state).cmd = format_single_from_target(item, cmd);
        } else {
            (*state).cmd = xstrdup(cmd).as_ptr();
        }
        log_debug!("{}: {}", __func__, _s((*state).cmd));

        if wait {
            (*state).pi.item = item;
        }
        let mut file = null();
        cmd_get_source(self_, &raw mut file, &(*state).pi.line);
        if !file.is_null() {
            (*state).pi.file = Some(cstr_to_str(xstrdup(file).as_ptr()));
        }
        (*state).pi.c = tc;
        if !(*state).pi.c.is_null() {
            (*(*state).pi.c).references += 1;
        }
        cmd_find_copy_state(&raw mut (*state).pi.fs, target);

        state
    }
}

/// Return argument as command.
pub unsafe fn args_make_commands(
    state: *mut args_command_state,
    argc: i32,
    argv: *mut *mut u8,
    error: *mut *mut u8,
) -> *mut cmd_list {
    let __func__ = "args_make_commands";
    unsafe {
        if !(*state).cmdlist.is_null() {
            if argc == 0 {
                return (*state).cmdlist;
            }
            return cmd_list_copy(&*(*state).cmdlist, argc, argv);
        }

        let mut cmd = xstrdup((*state).cmd).as_ptr();
        log_debug!("{}: {}", __func__, _s(cmd));
        cmd_log_argv!(argc, argv, "args_make_commands");
        for i in 0..argc {
            let new_cmd = cmd_template_replace(cmd, cstr_to_str_(*argv.add(i as usize)), i + 1);
            log_debug!(
                "{}: %%{} {}: {}",
                __func__,
                i + 1,
                _s(*argv.add(i as usize)),
                _s(new_cmd)
            );
            free_(cmd);
            cmd = new_cmd;
        }
        log_debug!("{}: {}", __func__, _s(cmd));

        let pr = cmd_parse_from_string(cstr_to_str(cmd), Some(&(*state).pi));
        free_(cmd);

        match pr {
            Err(err) => {
                *error = err.into_raw().cast();
                null_mut()
            }
            Ok(cmdlist) => cmdlist,
        }
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "this usage is okay, getting pointer to call free"
)]
/// Free commands state.
pub unsafe fn args_make_commands_free(state: *mut args_command_state) {
    unsafe {
        if !(*state).cmdlist.is_null() {
            cmd_list_free((*state).cmdlist);
        }
        if !(*state).pi.c.is_null() {
            server_client_unref((*state).pi.c);
        }
        free_(
            (*state)
                .pi
                .file
                .map(str::as_ptr)
                .unwrap_or_default()
                .cast_mut(),
        ); // TODO casting away const
        free_((*state).cmd);
        free_(state);
    }
}

/// Get prepared command.
pub unsafe fn args_make_commands_get_command(state: *mut args_command_state) -> *mut u8 {
    unsafe {
        if !(*state).cmdlist.is_null() {
            let first = cmd_list_commands((*state).cmdlist).first().copied().unwrap_or(null_mut());
            if first.is_null() {
                return xstrdup_(c"").as_ptr();
            }
            return xstrdup__(cmd_get_entry(first).name);
        }
        let n = libc::strcspn((*state).cmd, c!(" ,"));
        format_nul!("{1:0$}", n, _s((*state).cmd))
    }
}

/// Get the values for a flag as a slice.
pub unsafe fn args_flag_values<'a>(args: *mut args, flag: u8) -> &'a [args_value] {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return &[];
        }
        &(*entry).values
    }
}

/// Convert an argument value to a number.
pub unsafe fn args_strtonum(
    args: *mut args,
    flag: u8,
    minval: i64,
    maxval: i64,
) -> Result<i64, String> {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return Err("missing".into());
        }
        let Some(args_value::String { string }) = (*entry).values.last() else {
            return Err("missing".into());
        };
        strtonum(string.as_ptr().cast(), minval, maxval)
            .map_err(|errstr| errstr.to_string_lossy().into_owned())
    }
}

/// Convert an argument value to a number, and expand formats.
pub unsafe fn args_strtonum_and_expand(
    args: *mut args,
    flag: u8,
    minval: c_longlong,
    maxval: c_longlong,
    item: *mut cmdq_item,
) -> Result<c_longlong, String> {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return Err("missing".into());
        }
        let Some(args_value::String { string }) = (*entry).values.last() else {
            return Err("missing".into());
        };
        let formatted = format_single_from_target(item, string.as_ptr().cast());
        let tmp = strtonum(formatted, minval, maxval);
        free_(formatted);
        tmp.map_err(|errstr| errstr.to_string_lossy().into_owned())
    }
}

/// Convert an argument to a number which may be a percentage.
pub unsafe fn args_percentage(
    args: *mut args,
    flag: u8,
    minval: i64,
    maxval: i64,
    curval: i64,
) -> Result<i64, String> {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return Err("missing".into());
        }
        let Some(args_value::String { string }) = (*entry).values.last() else {
            return Err("empty".into());
        };
        let value = string.as_ptr().cast();
        args_string_percentage(value, minval, maxval, curval)
    }
}

/// Convert a string to a number which may be a percentage.
pub unsafe fn args_string_percentage(
    value: *const u8,
    minval: i64,
    maxval: i64,
    curval: i64,
) -> Result<i64, String> {
    unsafe {
        let valuelen: usize = strlen(value);

        if valuelen == 0 {
            return Err("empty".into());
        }
        let ll;
        if *value.add(valuelen - 1) == b'%' {
            let copy = xstrdup(value).as_ptr();
            *copy.add(valuelen - 1) = b'\0';

            let tmp = strtonum(copy, 0, 100);
            free_(copy);
            let pct = tmp.map_err(|errstr| errstr.to_string_lossy().into_owned())?;
            ll = (curval * pct) / 100;
            if ll < minval {
                return Err("too small".into());
            }
            if ll > maxval {
                return Err("too large".into());
            }
        } else {
            ll = strtonum(value, minval, maxval)
                .map_err(|errstr| errstr.to_string_lossy().into_owned())?;
        }

        Ok(ll)
    }
}

/// Convert an argument to a number which may be a percentage, and expand formats.
pub unsafe fn args_percentage_and_expand(
    args: *mut args,
    flag: u8,
    minval: i64,
    maxval: i64,
    curval: i64,
    item: *mut cmdq_item,
) -> Result<i64, String> {
    unsafe {
        let entry = args_find(args, flag);
        if entry.is_null() {
            return Err("missing".into());
        }
        let Some(args_value::String { string }) = (*entry).values.last() else {
            return Err("empty".into());
        };
        let value = string.as_ptr().cast();
        args_string_percentage_and_expand(value, minval, maxval, curval, item)
    }
}

/// Convert a string to a number which may be a percentage, and expand formats.
pub unsafe fn args_string_percentage_and_expand(
    value: *const u8,
    minval: i64,
    maxval: i64,
    curval: i64,
    item: *mut cmdq_item,
) -> Result<i64, String> {
    unsafe {
        let valuelen = strlen(value);
        let ll;

        if *value.add(valuelen - 1) == b'%' {
            let copy = xstrdup(value).as_ptr();
            *copy.add(valuelen - 1) = b'\0';

            let f = format_single_from_target(item, copy);
            let tmp = strtonum(f, 0, 100);
            free_(f);
            free_(copy);
            let pct = tmp.map_err(|errstr| errstr.to_string_lossy().into_owned())?;
            ll = (curval * pct) / 100;
            if ll < minval {
                return Err("too small".into());
            }
            if ll > maxval {
                return Err("too large".into());
            }
        } else {
            let f = format_single_from_target(item, value);
            let tmp = strtonum(f, minval, maxval);
            free_(f);
            ll = tmp.map_err(|errstr| errstr.to_string_lossy().into_owned())?;
        }

        Ok(ll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_to_string() {
        assert_eq!(args_type_to_string(&args_value::None), "NONE");
        unsafe {
            let s = args_value::new_string(xstrdup_(c"x").as_ptr());
            assert_eq!(args_type_to_string(&s), "STRING");
        }
    }

    #[test]
    fn create_is_empty() {
        unsafe {
            let args = args_create();
            assert_eq!(args_count(args), 0);
            assert!(!args_has(args, 'v'));
            args_free(args);
        }
    }

    #[test]
    fn set_and_has_flag() {
        unsafe {
            let args = args_create();
            args_set(args, b'v', None, 0);
            assert!(args_has(args, 'v'));
            assert!(!args_has(args, 'x'));
            args_free(args);
        }
    }

    #[test]
    fn set_flag_increments_count() {
        unsafe {
            let args = args_create();
            args_set(args, b'v', None, 0);
            args_set(args, b'v', None, 0);
            args_set(args, b'v', None, 0);
            assert_eq!(args_has_count(args, b'v'), 3);
            args_free(args);
        }
    }

    #[test]
    fn set_and_get_flag_value() {
        unsafe {
            let args = args_create();
            let value = args_value::new_string(xstrdup_(c"hello").as_ptr());
            args_set(args, b't', Some(value), 0);

            let got = args_get(args, b't');
            assert!(!got.is_null());
            assert_eq!(CStr::from_ptr(got.cast()).to_str().unwrap(), "hello");
            args_free(args);
        }
    }

    #[test]
    fn get_missing_flag_returns_null() {
        unsafe {
            let args = args_create();
            assert!(args_get(args, b'z').is_null());
            args_free(args);
        }
    }

    #[test]
    fn positional_args() {
        unsafe {
            let args = args_create();
            (*args).values.push(args_value::new_string(xstrdup_(c"target").as_ptr()));

            assert_eq!(args_count(args), 1);
            assert!(args_value(args, 0).is_some());
            assert_eq!(CStr::from_ptr(args_string(args, 0).cast()).to_str().unwrap(), "target");
            assert!(args_value(args, 1).is_none());
            assert!(args_string(args, 1).is_null());
            args_free(args);
        }
    }

    #[test]
    fn entry_list_sorted() {
        unsafe {
            let args = args_create();
            args_set(args, b'z', None, 0);
            args_set(args, b'a', None, 0);
            args_set(args, b'm', None, 0);

            let entries = args_entry_list(args);
            assert_eq!(entries.len(), 3);
            assert_eq!((*entries[0]).flag, b'a');
            assert_eq!((*entries[1]).flag, b'm');
            assert_eq!((*entries[2]).flag, b'z');
            args_free(args);
        }
    }

    #[test]
    fn percentage_plain_number() {
        unsafe {
            let v = CString::new("42").unwrap();
            let result = args_string_percentage(v.as_ptr().cast(), 0, 100, 200);
            assert_eq!(result, Ok(42));
        }
    }

    #[test]
    fn percentage_with_percent() {
        unsafe {
            let v = CString::new("50%").unwrap();
            let result = args_string_percentage(v.as_ptr().cast(), 0, 200, 200);
            assert_eq!(result, Ok(100));
        }
    }

    #[test]
    fn percentage_too_small() {
        unsafe {
            let v = CString::new("5").unwrap();
            let result = args_string_percentage(v.as_ptr().cast(), 10, 100, 200);
            assert!(result.is_err());
        }
    }

    #[test]
    fn percentage_too_large() {
        unsafe {
            let v = CString::new("200").unwrap();
            let result = args_string_percentage(v.as_ptr().cast(), 0, 100, 200);
            assert!(result.is_err());
        }
    }

    #[test]
    fn percentage_empty_string() {
        unsafe {
            let v = CString::new("").unwrap();
            let result = args_string_percentage(v.as_ptr().cast(), 0, 100, 200);
            assert!(result.is_err());
        }
    }

    #[test]
    fn escape_empty_string() {
        unsafe {
            let s = CString::new("").unwrap();
            let result = args_escape(s.as_ptr().cast());
            let escaped = CStr::from_ptr(result.cast()).to_str().unwrap().to_string();
            assert_eq!(escaped, "''");
            free_(result);
        }
    }

    #[test]
    fn escape_simple_word() {
        unsafe {
            let s = CString::new("hello").unwrap();
            let result = args_escape(s.as_ptr().cast());
            let escaped = CStr::from_ptr(result.cast()).to_str().unwrap().to_string();
            assert_eq!(escaped, "hello");
            free_(result);
        }
    }

    #[test]
    fn escape_word_with_space() {
        unsafe {
            let s = CString::new("hello world").unwrap();
            let result = args_escape(s.as_ptr().cast());
            let escaped = CStr::from_ptr(result.cast()).to_str().unwrap().to_string();
            assert!(escaped.starts_with('\'') || escaped.starts_with('"'), "expected quoted: {escaped}");
            free_(result);
        }
    }

    #[test]
    fn escape_single_special_char() {
        unsafe {
            let s = CString::new("#").unwrap();
            let result = args_escape(s.as_ptr().cast());
            let escaped = CStr::from_ptr(result.cast()).to_str().unwrap().to_string();
            assert_eq!(escaped, "\\#");
            free_(result);
        }
    }

    #[test]
    fn escape_tilde() {
        unsafe {
            let s = CString::new("~").unwrap();
            let result = args_escape(s.as_ptr().cast());
            let escaped = CStr::from_ptr(result.cast()).to_str().unwrap().to_string();
            assert_eq!(escaped, "\\~");
            free_(result);
        }
    }

    /// Helper to create an `args_value` Vec from `&CStr` slices.
    unsafe fn make_values(strs: &[&CStr]) -> Vec<args_value> {
        unsafe {
            strs.iter()
                .map(|s| args_value::new_string(xstrdup(s.as_ptr().cast()).cast().as_ptr()))
                .collect()
        }
    }

    #[test]
    fn parse_no_flags() {
        unsafe {
            let parse = args_parse::new("", 0, 1, None);
            let values = make_values(&[c"cmd", c"arg1"]);

            let args = args_parse(&raw const parse, &values).unwrap();
            assert!(!args.is_null());
            assert_eq!(args_count(args), 1);
            assert_eq!(CStr::from_ptr(args_string(args, 0).cast()).to_str().unwrap(), "arg1");

            args_free(args);
        }
    }

    #[test]
    fn parse_simple_flags() {
        unsafe {
            let parse = args_parse::new("ab", 0, 0, None);
            let values = make_values(&[c"cmd", c"-ab"]);

            let args = args_parse(&raw const parse, &values).unwrap();
            assert!(!args.is_null());
            assert!(args_has(args, 'a'));
            assert!(args_has(args, 'b'));
            assert!(!args_has(args, 'c'));

            args_free(args);
        }
    }

    #[test]
    fn parse_flag_with_argument() {
        unsafe {
            let parse = args_parse::new("t:", 0, 0, None);
            let values = make_values(&[c"cmd", c"-t", c"mysession"]);

            let args = args_parse(&raw const parse, &values).unwrap();
            assert!(!args.is_null());
            assert!(args_has(args, 't'));
            let t_val = args_get(args, b't');
            assert!(!t_val.is_null());
            assert_eq!(CStr::from_ptr(t_val.cast()).to_str().unwrap(), "mysession");

            args_free(args);
        }
    }

    #[test]
    fn parse_unknown_flag_is_error() {
        unsafe {
            let parse = args_parse::new("ab", 0, 0, None);
            let values = make_values(&[c"cmd", c"-z"]);

            let result = args_parse(&raw const parse, &values);
            assert!(matches!(result, Err(Some(_))));
        }
    }

    #[test]
    fn parse_too_few_args() {
        unsafe {
            let parse = args_parse::new("", 2, 3, None);
            let values = make_values(&[c"cmd", c"one"]);

            let result = args_parse(&raw const parse, &values);
            assert!(matches!(result, Err(Some(_))));
        }
    }

    #[test]
    fn parse_double_dash_stops_flags() {
        unsafe {
            let parse = args_parse::new("v", 0, 1, None);
            let values = make_values(&[c"cmd", c"--", c"-v"]);

            let args = args_parse(&raw const parse, &values).unwrap();
            assert!(!args.is_null());
            assert!(!args_has(args, 'v'));
            assert_eq!(args_count(args), 1);

            args_free(args);
        }
    }

    #[test]
    fn parse_empty_returns_empty() {
        unsafe {
            let parse = args_parse::new("v", -1, -1, None);

            let args = args_parse(&raw const parse, &[]).unwrap();
            assert!(!args.is_null());
            assert_eq!(args_count(args), 0);
            assert!(!args_has(args, 'v'));
            args_free(args);
        }
    }
}
