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

use crate::*;

use libc::strtol;

use crate::compat::queue::tailq_first;

#[unsafe(no_mangle)]
static mut cmd_send_keys_entry: cmd_entry = cmd_entry {
    name: c"send-keys".as_ptr(),
    alias: c"send".as_ptr(),

    args: args_parse::new(c"c:FHKlMN:Rt:X", 0, -1, None),
    usage: c"[-FHKlMRX] [-c target-client] [-N repeat-count] -t target-pane key ...".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK
        .union(cmd_flag::CMD_CLIENT_CFLAG)
        .union(cmd_flag::CMD_CLIENT_CANFAIL),
    exec: Some(cmd_send_keys_exec),

    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
static mut cmd_send_prefix_entry: cmd_entry = cmd_entry {
    name: c"send-prefix".as_ptr(),
    alias: null(),

    args: args_parse::new(c"2t:", 0, 0, None),
    usage: c"[-2] -t target-pane".as_ptr(),

    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_send_keys_exec),
    ..unsafe { zeroed() }
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_send_keys_inject_key(
    item: *mut cmdq_item,
    mut after: *mut cmdq_item,
    args: *mut args,
    key: key_code,
) -> *mut cmdq_item {
    unsafe {
        let mut target = cmdq_get_target(item);
        let mut tc = cmdq_get_target_client(item);
        let mut s = (*target).s;
        let mut wl = (*target).wl;
        let mut wp = (*target).wp;
        //struct window_mode_entry *wme;
        // struct key_binding *bd;
        // struct *event;

        if (args_has_(args, 'K')) {
            if tc.is_null() {
                return item;
            }
            let event = xmalloc_::<key_event>().as_ptr();
            (*event).key = key | KEYC_SENT;
            memset0(&raw mut (*event).m);
            if server_client_handle_key(tc, event) == 0 {
                free_(event);
            }
            return item;
        }

        let wme = tailq_first(&raw mut (*wp).modes);
        if (wme.is_null() || (*(*wme).mode).key_table.is_none()) {
            if window_pane_key(wp, tc, s, wl, key, null_mut()) != 0 {
                return null_mut();
            }
            return item;
        }

        let mut table = key_bindings_get_table((*(*wme).mode).key_table.unwrap()(wme), 1);

        let bd = key_bindings_get(NonNull::new(table).unwrap(), key & !KEYC_MASK_FLAGS);
        if (!bd.is_null()) {
            (*table).references += 1;
            after = key_bindings_dispatch(bd, after, tc, null_mut(), target);
            key_bindings_unref_table(table);
        }
        after
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_send_keys_inject_string(
    item: *mut cmdq_item,
    mut after: *mut cmdq_item,
    args: *mut args,
    i: i32,
) -> *mut cmdq_item {
    unsafe {
        let mut s = args_string(args, i as u32);
        let mut ud: *mut utf8_data;
        let mut loop_: *mut utf8_data;
        let mut uc: utf8_char = 0;
        let mut key: key_code;
        let mut endptr: *mut c_char = null_mut();
        let mut n: c_long = 0;
        // struct utf8_data *ud, *loop_;
        // utf8_char uc;
        // key_code key;
        // char *endptr;
        // long n;
        // int literal;

        if (args_has_(args, 'H')) {
            let n = strtol(s, &raw mut endptr, 16);
            if *s == b'\0' as _ || n < 0 || n > 0xff || *endptr != b'\0' as _ {
                return item;
            }
            return cmd_send_keys_inject_key(item, after, args, KEYC_LITERAL | n as u64);
        }

        let mut literal = args_has_(args, 'l');
        if (!literal) {
            key = key_string_lookup_string(s);
            if (key != KEYC_NONE && key != KEYC_UNKNOWN) {
                after = cmd_send_keys_inject_key(item, after, args, key);
                if !after.is_null() {
                    return after;
                }
            }
            literal = true;
        }
        if (literal) {
            ud = utf8_fromcstr(s);
            loop_ = ud;
            while (*loop_).size != 0 {
                if ((*loop_).size == 1 && (*loop_).data[0] <= 0x7f) {
                    key = (*loop_).data[0] as _;
                } else {
                    if (utf8_from_data(loop_, &raw mut uc) != utf8_state::UTF8_DONE) {
                        loop_ = loop_.add(1);
                        continue;
                    }
                    key = uc as _;
                }
                after = cmd_send_keys_inject_key(item, after, args, key);
                loop_ = loop_.add(1);
            }
            free_(ud);
        }
        after
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_send_keys_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut tc = cmdq_get_target_client(item);
        let mut s = (*target).s;
        let mut wl = (*target).wl;
        let mut wp = (*target).wp;
        let mut event = cmdq_get_event(item);
        let mut m = &raw mut (*event).m;
        let mut wme = tailq_first(&raw mut (*wp).modes);
        let mut after: *mut cmdq_item = item;
        let mut key: key_code = 0;
        // u_int i, np = 1;
        let mut np: u32 = 1;
        let count = args_count(args);
        let mut cause: *mut c_char = null_mut();

        if (args_has_(args, 'N')) {
            np = args_strtonum_and_expand(args, b'N', 1, u32::MAX as i64, item, &raw mut cause)
                as u32;
            if (!cause.is_null()) {
                cmdq_error(item, c"repeat count %s".as_ptr(), cause);
                free_(cause);
                return cmd_retval::CMD_RETURN_ERROR;
            }
            if (!wme.is_null() && (args_has_(args, 'X') || count == 0)) {
                if ((*(*wme).mode).command.is_none()) {
                    cmdq_error(item, c"not in a mode".as_ptr());
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                (*wme).prefix = np;
            }
        }

        if (args_has_(args, 'X')) {
            if (wme.is_null() || (*(*wme).mode).command.is_none()) {
                cmdq_error(item, c"not in a mode".as_ptr());
                return cmd_retval::CMD_RETURN_ERROR;
            }
            if (*m).valid == 0 {
                m = null_mut();
            }
            (*(*wme).mode).command.unwrap()(NonNull::new_unchecked(wme), tc, s, wl, args, m);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if (args_has_(args, 'M')) {
            wp = transmute_ptr(cmd_mouse_pane(m, &raw mut s, null_mut()));
            if (wp.is_null()) {
                cmdq_error(item, c"no mouse target".as_ptr());
                return cmd_retval::CMD_RETURN_ERROR;
            }
            window_pane_key(wp, tc, s, wl, (*m).key, m);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if (cmd_get_entry(self_) == &raw mut cmd_send_prefix_entry) {
            key = if (args_has_(args, '2')) {
                options_get_number((*s).options, c"prefix2".as_ptr()) as u64
            } else {
                options_get_number((*s).options, c"prefix".as_ptr()) as u64
            };
            cmd_send_keys_inject_key(item, item, args, key);
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        if (args_has_(args, 'R')) {
            colour_palette_clear(&raw mut (*wp).palette);
            input_reset((*wp).ictx, 1);
            (*wp).flags |= (window_pane_flags::PANE_STYLECHANGED | window_pane_flags::PANE_REDRAW);
        }

        if (count == 0) {
            if args_has_(args, 'N') || args_has_(args, 'R') {
                return cmd_retval::CMD_RETURN_NORMAL;
            }
            while np != 0 {
                cmd_send_keys_inject_key(item, null_mut(), args, (*event).key);
                np -= 1;
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        while np != 0 {
            for i in 0..count {
                after = cmd_send_keys_inject_string(item, after, args, i as i32);
            }
            np -= 1;
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
