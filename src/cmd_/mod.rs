// Copyright (c) 2007 Nicholas Marriott <nicholas.marriott@gmail.com>
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

use libc::{strchr, strcmp, strlen, strncmp};

use crate::compat::{
    queue::{
        tailq_concat, tailq_first, tailq_foreach, tailq_init, tailq_insert_tail, tailq_next,
        tailq_remove,
    },
    strlcat, strlcpy,
};
use crate::xmalloc::{xrealloc_, xreallocarray_};

pub mod cmd_attach_session;
pub mod cmd_bind_key;
pub mod cmd_break_pane;
pub mod cmd_capture_pane;
pub mod cmd_choose_tree;
pub mod cmd_command_prompt;
pub mod cmd_confirm_before;
pub mod cmd_copy_mode;
pub mod cmd_detach_client;
pub mod cmd_display_menu;
pub mod cmd_display_message;
pub mod cmd_display_panes;
pub mod cmd_find;
pub mod cmd_find_window;
pub mod cmd_if_shell;
pub mod cmd_join_pane;
pub mod cmd_kill_pane;
pub mod cmd_kill_server;
pub mod cmd_kill_session;
pub mod cmd_kill_window;
pub mod cmd_list_buffers;
pub mod cmd_list_clients;
pub mod cmd_list_keys;
pub mod cmd_list_panes;
pub mod cmd_list_sessions;
pub mod cmd_list_windows;
pub mod cmd_load_buffer;
pub mod cmd_lock_server;
pub mod cmd_move_window;
pub mod cmd_new_session;
pub mod cmd_new_window;
pub mod cmd_paste_buffer;
pub mod cmd_pipe_pane;
pub mod cmd_queue;
pub mod cmd_refresh_client;
pub mod cmd_rename_session;
pub mod cmd_rename_window;
pub mod cmd_resize_pane;
pub mod cmd_resize_window;
pub mod cmd_respawn_pane;
pub mod cmd_respawn_window;
pub mod cmd_rotate_window;
pub mod cmd_run_shell;
pub mod cmd_save_buffer;
pub mod cmd_select_layout;
pub mod cmd_select_pane;
pub mod cmd_select_window;
pub mod cmd_send_keys;
pub mod cmd_server_access;
pub mod cmd_set_buffer;
pub mod cmd_set_environment;
pub mod cmd_set_option;
pub mod cmd_show_environment;
pub mod cmd_show_messages;
pub mod cmd_show_options;
pub mod cmd_show_prompt_history;
pub mod cmd_source_file;
pub mod cmd_split_window;
pub mod cmd_swap_pane;
pub mod cmd_swap_window;
pub mod cmd_switch_client;
pub mod cmd_unbind_key;
pub mod cmd_wait_for;

unsafe extern "C" {
    // pub static mut cmd_table: [*const cmd_entry; 0usize];
    // pub fn cmd_log_argv(_: c_int, _: *mut *mut c_char, _: *const c_char, ...);
    // pub fn cmd_prepend_argv(_: *mut c_int, _: *mut *mut *mut c_char, _: *const c_char);
    // pub fn cmd_append_argv(_: *mut c_int, _: *mut *mut *mut c_char, _: *const c_char);
    // pub fn cmd_pack_argv(_: c_int, _: *mut *mut c_char, _: *mut c_char, _: usize) -> c_int;
    // pub fn cmd_unpack_argv(_: *mut c_char, _: usize, _: c_int, _: *mut *mut *mut c_char) -> c_int;
    // pub fn cmd_copy_argv(_: c_int, _: *mut *mut c_char) -> *mut *mut c_char;
    // pub fn cmd_free_argv(_: c_int, _: *mut *mut c_char);
    // pub fn cmd_stringify_argv(_: c_int, _: *mut *mut c_char) -> *mut c_char;
    // pub fn cmd_get_alias(_: *const c_char) -> *mut c_char;
    // pub fn cmd_get_entry(_: *mut cmd) -> *const cmd_entry;
    // pub fn cmd_get_args(_: *mut cmd) -> *mut args;
    // pub fn cmd_get_group(_: *mut cmd) -> c_uint;
    // pub fn cmd_get_source(_: *mut cmd, _: *mut *const c_char, _: *mut c_uint);
    // pub fn cmd_parse(_: *mut args_value, _: c_uint, _: *const c_char, _: c_uint, _: *mut *mut c_char) -> *mut cmd;
    // pub fn cmd_copy(_: *mut cmd, _: c_int, _: *mut *mut c_char) -> *mut cmd;
    // pub fn cmd_free(_: *mut cmd);
    // pub fn cmd_print(_: *mut cmd) -> *mut c_char;
    // pub fn cmd_list_new() -> *mut cmd_list;
    // pub fn cmd_list_copy(_: *mut cmd_list, _: c_int, _: *mut *mut c_char) -> *mut cmd_list;
    // pub fn cmd_list_append(_: *mut cmd_list, _: *mut cmd);
    // pub fn cmd_list_append_all(_: *mut cmd_list, _: *mut cmd_list);
    // pub fn cmd_list_move(_: *mut cmd_list, _: *mut cmd_list);
    // pub fn cmd_list_free(_: *mut cmd_list);
    // pub fn cmd_list_print(_: *mut cmd_list, _: c_int) -> *mut c_char;
    // pub fn cmd_list_first(_: *mut cmd_list) -> *mut cmd;
    // pub fn cmd_list_next(_: *mut cmd) -> *mut cmd;
    // pub fn cmd_list_all_have(_: *mut cmd_list, _: c_int) -> c_int;
    // pub fn cmd_list_any_have(_: *mut cmd_list, _: c_int) -> c_int;
    // pub fn cmd_mouse_at(_: *mut window_pane, _: *mut mouse_event, _: *mut c_uint, _: *mut c_uint, _: c_int) -> c_int;
    // pub fn cmd_mouse_window(_: *mut mouse_event, _: *mut *mut session) -> *mut winlink;
    // pub fn cmd_mouse_pane(_: *mut mouse_event, _: *mut *mut session, _: *mut *mut winlink) -> *mut window_pane;
    // pub fn cmd_template_replace(_: *const c_char, _: *const c_char, _: c_int) -> *mut c_char;
}

unsafe extern "C" {
    static cmd_attach_session_entry: cmd_entry;
    static cmd_bind_key_entry: cmd_entry;
    static cmd_break_pane_entry: cmd_entry;
    static cmd_capture_pane_entry: cmd_entry;
    static cmd_choose_buffer_entry: cmd_entry;
    static cmd_choose_client_entry: cmd_entry;
    static cmd_choose_tree_entry: cmd_entry;
    static cmd_clear_history_entry: cmd_entry;
    static cmd_clear_prompt_history_entry: cmd_entry;
    static cmd_clock_mode_entry: cmd_entry;
    static cmd_command_prompt_entry: cmd_entry;
    static cmd_confirm_before_entry: cmd_entry;
    static cmd_copy_mode_entry: cmd_entry;
    static cmd_customize_mode_entry: cmd_entry;
    static cmd_delete_buffer_entry: cmd_entry;
    static cmd_detach_client_entry: cmd_entry;
    static cmd_display_menu_entry: cmd_entry;
    static cmd_display_message_entry: cmd_entry;
    static cmd_display_popup_entry: cmd_entry;
    static cmd_display_panes_entry: cmd_entry;
    static cmd_find_window_entry: cmd_entry;
    static cmd_has_session_entry: cmd_entry;
    static cmd_if_shell_entry: cmd_entry;
    static cmd_join_pane_entry: cmd_entry;
    static cmd_kill_pane_entry: cmd_entry;
    static cmd_kill_server_entry: cmd_entry;
    static cmd_kill_session_entry: cmd_entry;
    static cmd_kill_window_entry: cmd_entry;
    static cmd_last_pane_entry: cmd_entry;
    static cmd_last_window_entry: cmd_entry;
    static cmd_link_window_entry: cmd_entry;
    static cmd_list_buffers_entry: cmd_entry;
    static cmd_list_clients_entry: cmd_entry;
    static cmd_list_commands_entry: cmd_entry;
    static cmd_list_keys_entry: cmd_entry;
    static cmd_list_panes_entry: cmd_entry;
    static cmd_list_sessions_entry: cmd_entry;
    static cmd_list_windows_entry: cmd_entry;
    static cmd_load_buffer_entry: cmd_entry;
    static cmd_lock_client_entry: cmd_entry;
    static cmd_lock_server_entry: cmd_entry;
    static cmd_lock_session_entry: cmd_entry;
    static cmd_move_pane_entry: cmd_entry;
    static cmd_move_window_entry: cmd_entry;
    static cmd_new_session_entry: cmd_entry;
    static cmd_new_window_entry: cmd_entry;
    static cmd_next_layout_entry: cmd_entry;
    static cmd_next_window_entry: cmd_entry;
    static cmd_paste_buffer_entry: cmd_entry;
    static cmd_pipe_pane_entry: cmd_entry;
    static cmd_previous_layout_entry: cmd_entry;
    static cmd_previous_window_entry: cmd_entry;
    static cmd_refresh_client_entry: cmd_entry;
    static cmd_rename_session_entry: cmd_entry;
    static cmd_rename_window_entry: cmd_entry;
    static cmd_resize_pane_entry: cmd_entry;
    static cmd_resize_window_entry: cmd_entry;
    static cmd_respawn_pane_entry: cmd_entry;
    static cmd_respawn_window_entry: cmd_entry;
    static cmd_rotate_window_entry: cmd_entry;
    static cmd_run_shell_entry: cmd_entry;
    static cmd_save_buffer_entry: cmd_entry;
    static cmd_select_layout_entry: cmd_entry;
    static cmd_select_pane_entry: cmd_entry;
    static cmd_select_window_entry: cmd_entry;
    static cmd_send_keys_entry: cmd_entry;
    static cmd_send_prefix_entry: cmd_entry;
    static cmd_server_access_entry: cmd_entry;
    static cmd_set_buffer_entry: cmd_entry;
    static cmd_set_environment_entry: cmd_entry;
    static cmd_set_hook_entry: cmd_entry;
    static cmd_set_option_entry: cmd_entry;
    static cmd_set_window_option_entry: cmd_entry;
    static cmd_show_buffer_entry: cmd_entry;
    static cmd_show_environment_entry: cmd_entry;
    static cmd_show_hooks_entry: cmd_entry;
    static cmd_show_messages_entry: cmd_entry;
    static cmd_show_options_entry: cmd_entry;
    static cmd_show_prompt_history_entry: cmd_entry;
    static cmd_show_window_options_entry: cmd_entry;
    static cmd_source_file_entry: cmd_entry;
    static cmd_split_window_entry: cmd_entry;
    static cmd_start_server_entry: cmd_entry;
    static cmd_suspend_client_entry: cmd_entry;
    static cmd_swap_pane_entry: cmd_entry;
    static cmd_swap_window_entry: cmd_entry;
    static cmd_switch_client_entry: cmd_entry;
    static cmd_unbind_key_entry: cmd_entry;
    static cmd_unlink_window_entry: cmd_entry;
    static cmd_wait_for_entry: cmd_entry;
}

#[unsafe(no_mangle)]
pub static mut cmd_table: [*const cmd_entry; 91] = unsafe {
    [
        &raw const cmd_attach_session_entry,
        &raw const cmd_bind_key_entry,
        &raw const cmd_break_pane_entry,
        &raw const cmd_capture_pane_entry,
        &raw const cmd_choose_buffer_entry,
        &raw const cmd_choose_client_entry,
        &raw const cmd_choose_tree_entry,
        &raw const cmd_clear_history_entry,
        &raw const cmd_clear_prompt_history_entry,
        &raw const cmd_clock_mode_entry,
        &raw const cmd_command_prompt_entry,
        &raw const cmd_confirm_before_entry,
        &raw const cmd_copy_mode_entry,
        &raw const cmd_customize_mode_entry,
        &raw const cmd_delete_buffer_entry,
        &raw const cmd_detach_client_entry,
        &raw const cmd_display_menu_entry,
        &raw const cmd_display_message_entry,
        &raw const cmd_display_popup_entry,
        &raw const cmd_display_panes_entry,
        &raw const cmd_find_window_entry,
        &raw const cmd_has_session_entry,
        &raw const cmd_if_shell_entry,
        &raw const cmd_join_pane_entry,
        &raw const cmd_kill_pane_entry,
        &raw const cmd_kill_server_entry,
        &raw const cmd_kill_session_entry,
        &raw const cmd_kill_window_entry,
        &raw const cmd_last_pane_entry,
        &raw const cmd_last_window_entry,
        &raw const cmd_link_window_entry,
        &raw const cmd_list_buffers_entry,
        &raw const cmd_list_clients_entry,
        &raw const cmd_list_commands_entry,
        &raw const cmd_list_keys_entry,
        &raw const cmd_list_panes_entry,
        &raw const cmd_list_sessions_entry,
        &raw const cmd_list_windows_entry,
        &raw const cmd_load_buffer_entry,
        &raw const cmd_lock_client_entry,
        &raw const cmd_lock_server_entry,
        &raw const cmd_lock_session_entry,
        &raw const cmd_move_pane_entry,
        &raw const cmd_move_window_entry,
        &raw const cmd_new_session_entry,
        &raw const cmd_new_window_entry,
        &raw const cmd_next_layout_entry,
        &raw const cmd_next_window_entry,
        &raw const cmd_paste_buffer_entry,
        &raw const cmd_pipe_pane_entry,
        &raw const cmd_previous_layout_entry,
        &raw const cmd_previous_window_entry,
        &raw const cmd_refresh_client_entry,
        &raw const cmd_rename_session_entry,
        &raw const cmd_rename_window_entry,
        &raw const cmd_resize_pane_entry,
        &raw const cmd_resize_window_entry,
        &raw const cmd_respawn_pane_entry,
        &raw const cmd_respawn_window_entry,
        &raw const cmd_rotate_window_entry,
        &raw const cmd_run_shell_entry,
        &raw const cmd_save_buffer_entry,
        &raw const cmd_select_layout_entry,
        &raw const cmd_select_pane_entry,
        &raw const cmd_select_window_entry,
        &raw const cmd_send_keys_entry,
        &raw const cmd_send_prefix_entry,
        &raw const cmd_server_access_entry,
        &raw const cmd_set_buffer_entry,
        &raw const cmd_set_environment_entry,
        &raw const cmd_set_hook_entry,
        &raw const cmd_set_option_entry,
        &raw const cmd_set_window_option_entry,
        &raw const cmd_show_buffer_entry,
        &raw const cmd_show_environment_entry,
        &raw const cmd_show_hooks_entry,
        &raw const cmd_show_messages_entry,
        &raw const cmd_show_options_entry,
        &raw const cmd_show_prompt_history_entry,
        &raw const cmd_show_window_options_entry,
        &raw const cmd_source_file_entry,
        &raw const cmd_split_window_entry,
        &raw const cmd_start_server_entry,
        &raw const cmd_suspend_client_entry,
        &raw const cmd_swap_pane_entry,
        &raw const cmd_swap_window_entry,
        &raw const cmd_switch_client_entry,
        &raw const cmd_unbind_key_entry,
        &raw const cmd_unlink_window_entry,
        &raw const cmd_wait_for_entry,
        null(),
    ]
};

// Instance of a command.
#[repr(C)]
pub struct cmd {
    pub entry: *mut cmd_entry,
    pub args: *mut args,
    pub group: u32,
    pub file: *mut c_char,
    pub line: u32,

    pub qentry: tailq_entry<cmd>,
}
pub type cmds = tailq_head<cmd>;

pub struct qentry;
impl Entry<cmd, qentry> for cmd {
    unsafe fn entry(this: *mut Self) -> *mut tailq_entry<cmd> {
        unsafe { &raw mut (*this).qentry }
    }
}

// Next group number for new command list.
#[unsafe(no_mangle)]
pub static mut cmd_list_next_group: u32 = 1;

macro_rules! cmd_log_argv {
   ($argc:expr, $argv:expr, $fmt:literal $(, $args:expr)* $(,)?) => {
        crate::cmd_::cmd_log_argv_($argc, $argv, format_args!($fmt $(, $args)*))
    };
}
pub(crate) use cmd_log_argv;

// Log an argument vector.
pub unsafe fn cmd_log_argv_(argc: i32, argv: *mut *mut c_char, args: std::fmt::Arguments) {
    unsafe {
        let mut prefix = args.to_string();
        for i in 0..argc {
            log_debug!("{}: argv[{}]{}", prefix, i, _s(*argv.add(i as usize)));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_prepend_argv(
    argc: *mut c_int,
    argv: *mut *mut *mut c_char,
    arg: *const c_char,
) {
    unsafe {
        let new_argv: *mut *mut c_char =
            xreallocarray_::<*mut c_char>(null_mut(), (*argc) as usize + 1)
                .cast()
                .as_ptr();
        *new_argv = xstrdup(arg).as_ptr();
        for i in 0..*argc {
            *new_argv.add(1 + i as usize) = *(*argv).add(i as usize);
        }

        free(*argv as _);
        *argv = new_argv;
        *argc += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_append_argv(
    argc: *mut c_int,
    argv: *mut *mut *mut c_char,
    arg: *const c_char,
) {
    unsafe {
        *argv = xreallocarray_::<*mut c_char>(*argv, (*argc) as usize + 1).as_ptr();
        *(*argv).add((*argc) as usize) = xstrdup(arg).as_ptr();
        *argc += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_pack_argv(
    argc: c_int,
    argv: *mut *mut c_char,
    mut buf: *mut c_char,
    mut len: usize,
) -> c_int {
    unsafe {
        //
        if argc == 0 {
            return 0;
        }
        cmd_log_argv!(argc, argv, "cmd_pack_argv");

        *buf = b'\0' as c_char;
        for i in 0..argc {
            if strlcpy(buf, *argv.add(i as usize), len) >= len {
                return -1;
            }
            let arglen = strlen(*argv.add(i as usize)) + 1;
            buf = buf.add(arglen);
            len -= arglen;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_unpack_argv(
    mut buf: *mut c_char,
    mut len: usize,
    argc: c_int,
    argv: *mut *mut *mut c_char,
) -> c_int {
    unsafe {
        if argc == 0 {
            return 0;
        }
        *argv = xcalloc_::<*mut c_char>(argc as usize).as_ptr();

        *buf.add(len - 1) = b'\0' as c_char;
        for i in 0..argc {
            if len == 0 {
                cmd_free_argv(argc, *argv);
                return -1;
            }

            let arglen = strlen(buf) + 1;
            *(*argv).add(i as usize) = xstrdup(buf).as_ptr();

            buf = buf.add(arglen);
            len -= arglen;
        }
        cmd_log_argv!(argc, *argv, "cmd_unpack_argv");

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_copy_argv(argc: c_int, argv: *mut *mut c_char) -> *mut *mut c_char {
    unsafe {
        if argc == 0 {
            return null_mut();
        }
        let new_argv: *mut *mut c_char = xcalloc(argc as usize + 1, size_of::<*mut c_char>())
            .cast()
            .as_ptr();
        for i in 0..argc {
            if !(*argv.add(i as usize)).is_null() {
                *new_argv.add(i as usize) = xstrdup(*argv.add(i as usize)).as_ptr();
            }
        }
        new_argv
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_free_argv(argc: c_int, argv: *mut *mut c_char) {
    unsafe {
        if argc == 0 {
            return;
        }
        for i in 0..argc {
            free(*argv.add(i as usize) as _);
        }
        free(argv as _);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_stringify_argv(argc: c_int, argv: *mut *mut c_char) -> *mut c_char {
    unsafe {
        //char	*buf = NULL, *s;
        //size_t	 len = 0;
        //int	 i;
        let s: *mut c_char = null_mut();
        let mut buf: *mut c_char = null_mut();
        let mut len: usize = 0;

        if argc == 0 {
            return xstrdup(c"".as_ptr()).as_ptr();
        }

        for i in 0..argc {
            let s = args_escape(*argv.add(i as usize));
            log_debug!(
                "{}: {} {} = {}",
                "cmd_stringify_argv",
                i,
                _s(*argv.add(i as usize)),
                _s(s)
            );

            len += strlen(s) + 1;
            buf = xrealloc_(buf, len).as_ptr();

            if i == 0 {
                *buf = b'\0' as c_char;
            } else {
                strlcat(buf, c" ".as_ptr(), len);
            }
            strlcat(buf, s, len);

            free(s as _);
        }
        buf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_get_entry(cmd: *mut cmd) -> *mut cmd_entry {
    unsafe { (*cmd).entry }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_get_args(cmd: *mut cmd) -> *mut args {
    unsafe { (*cmd).args }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_get_group(cmd: *mut cmd) -> c_uint {
    unsafe { (*cmd).group }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_get_source(
    cmd: *mut cmd,
    file: *mut *const c_char,
    line: *mut c_uint,
) {
    unsafe {
        if !file.is_null() {
            *file = (*cmd).file;
        }
        if !line.is_null() {
            *line = (*cmd).line;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_get_alias(name: *const c_char) -> *mut c_char {
    unsafe {
        let o = options_get_only(global_options, c"command-alias".as_ptr());
        if o.is_null() {
            return null_mut();
        }
        let wanted = strlen(name);

        let mut a = options_array_first(o);
        while !a.is_null() {
            let ov = options_array_item_value(a);

            let equals = strchr((*ov).string, b'=' as i32);
            if !equals.is_null() {
                let n = equals.addr() - (*ov).string.addr();
                if n == wanted && strncmp(name, (*ov).string, n) == 0 {
                    return xstrdup(equals.add(1)).as_ptr();
                }
            }

            a = options_array_next(a);
        }
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_find(name: *const c_char, cause: *mut *mut c_char) -> *mut cmd_entry {
    let mut loop_: *mut *mut cmd_entry = null_mut();
    let mut entry: *mut cmd_entry = null_mut();
    let mut found: *mut cmd_entry = null_mut();

    let mut ambiguous: i32 = 0;
    type s_buf = [c_char; 8192];
    let mut s: s_buf = [0; 8192];

    unsafe {
        'ambiguous: {
            loop_ = &raw mut cmd_table as _; // TODO casting const pointer to mut ptr
            while !(*loop_).is_null() {
                entry = *loop_;
                if !(*entry).alias.is_null() && strcmp((*entry).alias, name) == 0 {
                    ambiguous = 0;
                    found = entry;
                    break;
                }

                if strncmp((*entry).name, name, strlen(name)) != 0 {
                    loop_ = loop_.add(1);
                    continue;
                }
                if !found.is_null() {
                    ambiguous = 1;
                }
                found = entry;

                if strcmp((*entry).name, name) == 0 {
                    break;
                }

                loop_ = loop_.add(1);
            }
            if ambiguous != 0 {
                break 'ambiguous;
            }
            if found.is_null() {
                // TODO BUG, for some reason name isn't properly NUL terminated
                xasprintf(cause, c"unknown command: %s".as_ptr(), name);
                return null_mut();
            }

            return found;
        }

        // ambiguous:
        s[0] = b'\0' as c_char;
        loop_ = &raw mut cmd_table as _;
        while !(*loop_).is_null() {
            entry = *loop_;
            if strncmp((*entry).name, name, strlen(name)) != 0 {
                continue;
            }
            if strlcat(&raw mut s as _, (*entry).name, size_of::<s_buf>()) >= size_of::<s_buf>() {
                break;
            }
            if strlcat(&raw mut s as _, c", ".as_ptr(), size_of::<s_buf>()) >= size_of::<s_buf>() {
                break;
            }
            loop_ = loop_.add(1);
        }
        s[strlen(&raw mut s as _) - 2] = b'\0' as c_char;
        xasprintf(
            cause,
            c"ambiguous command: %s, could be: %s".as_ptr(),
            name,
            s,
        );

        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_parse(
    values: *mut args_value,
    count: c_uint,
    file: *const c_char,
    line: c_uint,
    cause: *mut *mut c_char,
) -> *mut cmd {
    unsafe {
        let mut error: *mut c_char = null_mut();

        if count == 0 || (*values).type_ != args_type::ARGS_STRING {
            xasprintf(cause, c"no command".as_ptr());
            return null_mut();
        }
        let entry = cmd_find((*values).union_.string, cause);
        if entry.is_null() {
            return null_mut();
        }

        let args = args_parse(&raw mut (*entry).args, values, count, &raw mut error);
        if args.is_null() && error.is_null() {
            xasprintf(
                cause,
                c"usage: %s %s".as_ptr(),
                (*entry).name,
                (*entry).usage,
            );
            return null_mut();
        }
        if args.is_null() {
            xasprintf(cause, c"command %s: %s".as_ptr(), (*entry).name, error);
            free(error as _);
            return null_mut();
        }

        let cmd: *mut cmd = xcalloc(1, size_of::<cmd>()).cast().as_ptr();
        (*cmd).entry = entry;
        (*cmd).args = args;

        if !file.is_null() {
            (*cmd).file = xstrdup(file).as_ptr();
        }
        (*cmd).line = line;

        cmd
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_free(cmd: *mut cmd) {
    unsafe {
        free((*cmd).file as _);

        args_free((*cmd).args);
        free(cmd as _);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_copy(cmd: *mut cmd, argc: c_int, argv: *mut *mut c_char) -> *mut cmd {
    unsafe {
        let new_cmd: *mut cmd = xcalloc(1, size_of::<cmd>()).cast().as_ptr();
        (*new_cmd).entry = (*cmd).entry;
        (*new_cmd).args = args_copy((*cmd).args, argc, argv);

        if !(*cmd).file.is_null() {
            (*new_cmd).file = xstrdup((*cmd).file).as_ptr();
        }
        (*new_cmd).line = (*cmd).line;

        new_cmd
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_print(cmd: *mut cmd) -> *mut c_char {
    unsafe {
        let mut out: *mut c_char = null_mut();

        let s = args_print((*cmd).args);
        if *s != b'\0' as c_char {
            xasprintf(&raw mut out, c"%s %s".as_ptr(), (*(*cmd).entry).name, s);
        } else {
            out = xstrdup((*(*cmd).entry).name).as_ptr();
        }
        free(s as _);

        out
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_new() -> *mut cmd_list {
    unsafe {
        let cmdlist: *mut cmd_list = xcalloc(1, size_of::<cmd_list>()).cast().as_ptr();
        (*cmdlist).references = 1;
        (*cmdlist).group = cmd_list_next_group;
        cmd_list_next_group += 1;
        (*cmdlist).list = xcalloc(1, size_of::<cmds>()).cast().as_ptr();
        tailq_init((*cmdlist).list);
        cmdlist
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_append(cmdlist: *mut cmd_list, cmd: *mut cmd) {
    unsafe {
        (*cmd).group = (*cmdlist).group;
        tailq_insert_tail::<_, qentry>((*cmdlist).list, cmd);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_append_all(cmdlist: *mut cmd_list, from: *mut cmd_list) {
    unsafe {
        for cmd in tailq_foreach::<_, qentry>((*from).list).map(NonNull::as_ptr) {
            (*cmd).group = (*cmdlist).group;
        }
        tailq_concat::<_, qentry>((*cmdlist).list, (*from).list);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_move(cmdlist: *mut cmd_list, from: *mut cmd_list) {
    unsafe {
        tailq_concat::<_, qentry>((*cmdlist).list, (*from).list);
        (*cmdlist).group = cmd_list_next_group;
        cmd_list_next_group += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_free(cmdlist: *mut cmd_list) {
    unsafe {
        (*cmdlist).references -= 1;
        if (*cmdlist).references != 0 {
            return;
        }

        for cmd in tailq_foreach::<_, qentry>((*cmdlist).list).map(NonNull::as_ptr) {
            tailq_remove::<_, qentry>((*cmdlist).list, cmd);
            cmd_free(cmd);
        }
        free_((*cmdlist).list);
        free_(cmdlist);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_copy(
    cmdlist: *mut cmd_list,
    argc: c_int,
    argv: *mut *mut c_char,
) -> *mut cmd_list {
    unsafe {
        let mut group: u32 = (*cmdlist).group;
        let s = cmd_list_print(cmdlist, 0);
        log_debug!("{}: {}", "cmd_list_copy", _s(s));
        free(s as _);

        let new_cmdlist = cmd_list_new();
        for cmd in tailq_foreach((*cmdlist).list).map(NonNull::as_ptr) {
            if (*cmd).group != group {
                (*new_cmdlist).group = cmd_list_next_group;
                cmd_list_next_group += 1;
                group = (*cmd).group;
            }
            let new_cmd = cmd_copy(cmd, argc, argv);
            cmd_list_append(new_cmdlist, new_cmd);
        }

        let s = cmd_list_print(new_cmdlist, 0);
        log_debug!("{}: {}", "cmd_list_copy", _s(s));
        free(s as _);

        new_cmdlist
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_print(cmdlist: *mut cmd_list, escaped: c_int) -> *mut c_char {
    unsafe {
        let mut len = 1;
        let mut buf: *mut c_char = xcalloc(1, len).cast().as_ptr();

        for cmd in tailq_foreach::<_, qentry>((*cmdlist).list).map(NonNull::as_ptr) {
            let this = cmd_print(cmd);

            len += strlen(this) + 6;
            buf = xrealloc_(buf, len).as_ptr();

            strlcat(buf, this, len);

            let next = tailq_next::<_, _, qentry>(cmd);
            if !next.is_null() {
                if (*cmd).group != (*next).group {
                    if escaped != 0 {
                        strlcat(buf, c" \\;\\; ".as_ptr(), len);
                    } else {
                        strlcat(buf, c" ;; ".as_ptr(), len);
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if escaped != 0 {
                        strlcat(buf, c" \\; ".as_ptr(), len);
                    } else {
                        strlcat(buf, c" ; ".as_ptr(), len);
                    }
                }
            }

            free_(this);
        }

        buf
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_first(cmdlist: *mut cmd_list) -> *mut cmd {
    unsafe { tailq_first((*cmdlist).list) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_next(cmd: *mut cmd) -> *mut cmd {
    unsafe { tailq_next::<_, _, qentry>(cmd) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_all_have(cmdlist: *mut cmd_list, flag: cmd_flag) -> boolint {
    unsafe {
        boolint::from(
            tailq_foreach((*cmdlist).list)
                .into_iter()
                .all(|cmd| (*(*cmd.as_ptr()).entry).flags.intersects(flag)),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_list_any_have(cmdlist: *mut cmd_list, flag: cmd_flag) -> boolint {
    unsafe {
        tailq_foreach((*cmdlist).list)
            .into_iter()
            .any(|cmd| (*(*cmd.as_ptr()).entry).flags.intersects(flag))
            .into()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_mouse_at(
    wp: *mut window_pane,
    m: *mut mouse_event,
    xp: *mut c_uint,
    yp: *mut c_uint,
    last: c_int,
) -> c_int {
    unsafe {
        let x: u32;
        let mut y: u32;

        if last != 0 {
            x = (*m).lx + (*m).ox;
            y = (*m).ly + (*m).oy;
        } else {
            x = (*m).x + (*m).ox;
            y = (*m).y + (*m).oy;
        }
        log_debug!(
            "{}: x={}, y={}{}",
            "cmd_mouse_at",
            x,
            y,
            if last != 0 { " (last)" } else { "" }
        );

        if (*m).statusat == 0 && y >= (*m).statuslines {
            y -= (*m).statuslines;
        }

        if x < (*wp).xoff || x >= (*wp).xoff + (*wp).sx {
            return -1;
        }

        if y < (*wp).yoff || y >= (*wp).yoff + (*wp).sy {
            return -1;
        }

        if !xp.is_null() {
            *xp = x - (*wp).xoff;
        }
        if !yp.is_null() {
            *yp = y - (*wp).yoff;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_mouse_window(
    m: *mut mouse_event,
    sp: *mut *mut session,
) -> Option<NonNull<winlink>> {
    unsafe {
        let mut s: *mut session = null_mut();
        let mut wl: Option<NonNull<winlink>> = None;

        if (*m).valid == 0 {
            return None;
        }
        if (*m).s == -1
            || ({
                s = transmute_ptr(session_find_by_id((*m).s as u32));
                s.is_null()
            })
        {
            return None;
        }
        if (*m).w == -1 {
            wl = NonNull::new((*s).curw);
        } else {
            let w = window_find_by_id((*m).w as u32);
            if w.is_null() {
                return None;
            }
            wl = winlink_find_by_window(&raw mut (*s).windows, w);
        }
        if !sp.is_null() {
            *sp = s;
        }
        wl
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_mouse_pane(
    m: *mut mouse_event,
    sp: *mut *mut session,
    wlp: *mut *mut winlink,
) -> Option<NonNull<window_pane>> {
    unsafe {
        let wl = cmd_mouse_window(m, sp)?;
        let mut wp = None;

        if (*m).wp == -1 {
            wp = NonNull::new((*(*wl.as_ptr()).window).active);
        } else {
            let wp = NonNull::new(window_pane_find_by_id((*m).wp as u32))?;
            if !window_has_pane((*wl.as_ptr()).window, wp.as_ptr()) {
                return None;
            }
        }

        if !wlp.is_null() {
            *wlp = wl.as_ptr();
        }
        wp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_template_replace(
    template: *const c_char,
    s: *const c_char,
    idx: c_int,
) -> *mut c_char {
    unsafe {
        let quote = c"\"\\$;~";

        if strchr(template, b'%' as i32).is_null() {
            return xstrdup(template).cast().as_ptr();
        }

        let mut buf: *mut c_char = xmalloc(1).cast().as_ptr();
        *buf = b'\0' as c_char;
        let mut len = 0;
        let mut replaced = 0;

        let mut ptr = template;
        while *ptr != b'\0' as c_char {
            let ch = *ptr;
            ptr = ptr.add(1);
            if matches!(ch as c_uchar, b'%') {
                if *ptr < b'1' as c_char
                    || *ptr > b'9' as c_char
                    || *ptr as i32 - b'0' as i32 != idx
                {
                    if *ptr != b'%' as c_char || replaced != 0 {
                        break;
                    }
                    replaced = 1;
                }
                ptr = ptr.add(1);

                let quoted = *ptr == b'%' as c_char;
                if !quoted {
                    ptr = ptr.add(1);
                }

                buf = xrealloc_(buf, len + (strlen(s) * 3) + 1).as_ptr();
                let mut cp = s;
                while *cp != b'\0' as c_char {
                    if quoted && !strchr(quote.as_ptr(), *cp as i32).is_null() {
                        *buf.add(len) = b'\\' as c_char;
                        len += 1;
                    }
                    *buf.add(len) = *cp;
                    len += 1;
                    cp = cp.add(1);
                }
                *buf.add(len) = b'\0' as c_char;
                continue;
            }
            buf = xrealloc_(buf, len + 2).as_ptr();
            *buf.add(len) = ch;
            len += 1;
            *buf.add(len) = b'\0' as c_char;
        }

        buf
    }
}
