// Copyright (c) 2009 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use crate::*;

pub static CMD_SHOW_ENVIRONMENT_ENTRY: cmd_entry = cmd_entry {
    name: "show-environment",
    alias: Some("showenv"),

    args: args_parse::new("hgst:", 0, 1, None),
    usage: "[-hgs] [-t target-session] [name]",

    target: cmd_entry_flag::new(
        b't',
        cmd_find_type::CMD_FIND_SESSION,
        cmd_find_flags::CMD_FIND_CANFAIL,
    ),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: cmd_show_environment_exec,
    source: cmd_entry_flag::zeroed(),
};

/// Escape a value for shell double-quote context. Characters that POSIX
/// interprets inside double quotes (`$`, `` ` ``, `"`, `\`) are backslash-escaped.
fn cmd_show_environment_escape(value: &[u8]) -> String {
    let mut out = String::with_capacity(value.len() * 2);
    for &c in value {
        if c == b'$' || c == b'`' || c == b'"' || c == b'\\' {
            out.push('\\');
        }
        out.push(c as char);
    }
    out
}

unsafe fn cmd_show_environment_print(
    self_: *mut cmd,
    item: *mut cmdq_item,
    envent: &EnvironEntry,
) {
    unsafe {
        let args = cmd_get_args(self_);

        if !args_has(args, 'h') && envent.flags.intersects(ENVIRON_HIDDEN) {
            return;
        }
        if args_has(args, 'h') && !envent.flags.intersects(ENVIRON_HIDDEN) {
            return;
        }

        if !args_has(args, 's') {
            if let Some(ref value) = envent.value {
                cmdq_print!(
                    item,
                    "{}={}",
                    envent.name,
                    String::from_utf8_lossy(value),
                );
            } else {
                cmdq_print!(item, "-{}", envent.name);
            }
            return;
        }

        if let Some(ref value) = envent.value {
            let escaped = cmd_show_environment_escape(value);
            cmdq_print!(
                item,
                "{}=\"{}\"; export {};",
                envent.name,
                escaped,
                envent.name,
            );
        } else {
            cmdq_print!(item, "unset {};", envent.name);
        }
    }
}

unsafe fn cmd_show_environment_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let target = cmdq_get_target(item);
        let env: *mut Environ;
        let name = args_string(args, 0);

        let mut tflag = args_get_(args, 't');
        if !tflag.is_null() && (*target).s.is_null() {
            cmdq_error!(item, "no such session: {}", _s(tflag));
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, 'g') {
            env = GLOBAL_ENVIRON;
        } else {
            if (*target).s.is_null() {
                tflag = args_get_(args, 't');
                if !tflag.is_null() {
                    cmdq_error!(item, "no such session: {}", _s(tflag));
                } else {
                    cmdq_error!(item, "no current session");
                }
                return cmd_retval::CMD_RETURN_ERROR;
            }
            env = (*(*target).s).environ;
        }

        if !name.is_null() {
            let Some(envent) = environ_find_raw(&*env, name) else {
                cmdq_error!(item, "unknown variable: {}", _s(name));
                return cmd_retval::CMD_RETURN_ERROR;
            };
            cmd_show_environment_print(self_, item, envent);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        for entry in environ_entries(&*env) {
            cmd_show_environment_print(self_, item, entry);
        }
        cmd_retval::CMD_RETURN_NORMAL
    }
}
