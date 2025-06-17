// Copyright (c) 2009 Jonathan Alvarado <radobobo@users.sourceforge.net>
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

use libc::{INT_MIN, strcmp, strlen};

use crate::*;

#[unsafe(no_mangle)]
pub static mut cmd_capture_pane_entry: cmd_entry = cmd_entry {
    name: c"capture-pane".as_ptr(),
    alias: c"capturep".as_ptr(),

    args: args_parse::new(c"ab:CeE:JNpPqS:Tt:", 0, 0, None),
    usage: c"[-aCeJNpPqT] [-b buffer-name] [-E end-line] [-S start-line] [-t target-pane]".as_ptr(),

    source: unsafe { zeroed() },
    target: cmd_entry_flag::new(b't', cmd_find_type::CMD_FIND_PANE, 0),

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_capture_pane_exec),
};

#[unsafe(no_mangle)]
pub static mut cmd_clear_history_entry: cmd_entry = cmd_entry {
    name: c"clear-history".as_ptr(),
    alias: c"clearhist".as_ptr(),

    args: args_parse::new(c"Ht:", 0, 0, None),
    usage: c"[-H] [-t target-pane]".as_ptr(),

    source: unsafe { zeroed() },
    target: cmd_entry_flag {
        flag: b't' as _,
        type_: cmd_find_type::CMD_FIND_PANE,
        flags: 0,
    },

    flags: cmd_flag::CMD_AFTERHOOK,
    exec: Some(cmd_capture_pane_exec),
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_capture_pane_append(
    mut buf: *mut c_char,
    len: *mut usize,
    line: *mut c_char,
    linelen: usize,
) -> *mut c_char {
    unsafe {
        buf = xrealloc_(buf, *len + linelen + 1).as_ptr();
        memcpy_(buf.add(*len), line, linelen);
        *len += linelen;
        buf
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_capture_pane_pending(
    args: *mut args,
    wp: *const window_pane,
    len: *mut usize,
) -> *mut c_char {
    let mut tmp: [c_char; 5] = [0; 5];

    unsafe {
        let pending = input_pending((*wp).ictx);
        if pending.is_null() {
            return xstrdup(c"".as_ptr()).as_ptr();
        }

        let mut line = EVBUFFER_DATA(pending);
        let linelen = EVBUFFER_LENGTH(pending);

        let mut buf = xstrdup(c"".as_ptr()).as_ptr();
        if args_has(args, b'C') != 0 {
            for i in 0usize..linelen {
                if *line.add(i) >= b' ' && *line.add(i) != b'\\' {
                    tmp[0] = *line.add(i) as _;
                    tmp[1] = b'\0' as _;
                } else {
                    xsnprintf(
                        &raw mut tmp as _,
                        size_of::<[c_char; 5]>(),
                        c"\\%03hho".as_ptr(),
                        *line.add(i) as usize,
                    );
                }
                buf =
                    cmd_capture_pane_append(buf, len, &raw mut tmp as _, strlen(&raw mut tmp as _));
            }
        } else {
            buf = cmd_capture_pane_append(buf, len, &raw mut line as _, linelen);
        }
        buf
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_capture_pane_history(
    args: *mut args,
    item: *mut cmdq_item,
    wp: *mut window_pane,
    len: *mut usize,
) -> *mut c_char {
    unsafe {
        let mut gd: *mut grid = null_mut();
        let mut gl: *const grid_line = null_mut();
        let mut gc: *mut grid_cell = null_mut();
        let mut n = 0;
        let mut join_lines = 0;
        let mut flags = grid_string_flags::empty();

        let mut tmp: u32 = 0;
        let mut bottom: u32 = 0;
        let mut cause: *mut c_char = null_mut();
        let buf: *mut c_char = null_mut();
        let mut line: *mut c_char = null_mut();

        let mut linelen: usize = 0;

        let sx = screen_size_x(&raw mut (*wp).base);
        if args_has(args, b'a') != 0 {
            gd = (*wp).base.saved_grid;
            if gd.is_null() {
                if args_has(args, b'q') == 0 {
                    cmdq_error(item, c"no alternate screen".as_ptr());
                    return null_mut();
                }
                return xstrdup(c"".as_ptr()).as_ptr();
            }
        } else {
            gd = (*wp).base.grid;
        }

        let sflag: *const c_char = args_get(args, b'S');
        let mut top = 0;
        if !sflag.is_null() && strcmp(sflag, c"-".as_ptr()) == 0 {
            top = 0;
        } else {
            n = args_strtonum_and_expand(
                args,
                b'S',
                libc::INT_MIN as i64,
                c_short::MAX as i64,
                item,
                &raw mut cause,
            );
            if !cause.is_null() {
                top = (*gd).hsize;
                free_(cause);
            } else if n < 0 && (-n) as u32 > (*gd).hsize {
                top = 0;
            } else {
                top = (*gd).hsize + n as u32;
            }
            if top > (*gd).hsize + (*gd).sy - 1 {
                top = (*gd).hsize + (*gd).sy - 1;
            }
        }

        let eflag: *const c_char = args_get(args, b'E');
        if !eflag.is_null() && strcmp(eflag, c"-".as_ptr()) == 0 {
            bottom = (*gd).hsize + (*gd).sy - 1;
        } else {
            n = args_strtonum_and_expand(
                args,
                b'E',
                INT_MIN as i64,
                i16::MAX as i64,
                item,
                &raw mut cause,
            );
            if !cause.is_null() {
                bottom = (*gd).hsize + (*gd).sy - 1;
                free_(cause);
            } else if n < 0 && (-n) as u32 > (*gd).hsize {
                bottom = 0;
            } else {
                bottom = (*gd).hsize + n as u32;
            }
            if bottom > (*gd).hsize + (*gd).sy - 1 {
                bottom = (*gd).hsize + (*gd).sy - 1;
            }
        }

        if bottom < top {
            tmp = bottom;
            bottom = top;
            top = tmp;
        }

        join_lines = args_has(args, b'J');
        if args_has(args, b'e') != 0 {
            flags |= grid_string_flags::GRID_STRING_WITH_SEQUENCES;
        }
        if args_has(args, b'C') != 0 {
            flags |= grid_string_flags::GRID_STRING_ESCAPE_SEQUENCES;
        }
        if join_lines == 0 && args_has(args, b'T') == 0 {
            flags |= grid_string_flags::GRID_STRING_EMPTY_CELLS;
        }
        if join_lines == 0 && args_has(args, b'N') == 0 {
            flags |= grid_string_flags::GRID_STRING_TRIM_SPACES;
        }

        let mut buf = null_mut();
        for i in top..=bottom {
            line = grid_string_cells(gd, 0, i, sx, &raw mut gc, flags, (*wp).screen);
            linelen = strlen(line);

            buf = cmd_capture_pane_append(buf, len, line, linelen);

            gl = grid_peek_line(gd, i);
            if join_lines == 0 || !(*gl).flags.intersects(grid_line_flag::WRAPPED) {
                *buf.add(*len) = b'\n' as _;
                (*len) += 1;
            }

            free_(line);
        }
        buf
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_capture_pane_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let args = cmd_get_args(self_);
        let c = cmdq_get_client(item);
        let wp = (*cmdq_get_target(item)).wp;

        if cmd_get_entry(self_) == &raw mut cmd_clear_history_entry {
            window_pane_reset_mode_all(wp);
            grid_clear_history((*wp).base.grid);
            if args_has(args, b'H') != 0 {
                screen_reset_hyperlinks((*wp).screen);
            }
            return cmd_retval::CMD_RETURN_NORMAL;
        }

        let mut len = 0;
        let buf = if args_has(args, b'P') != 0 {
            cmd_capture_pane_pending(args, wp, &raw mut len)
        } else {
            cmd_capture_pane_history(args, item, wp, &raw mut len)
        };
        if buf.is_null() {
            return cmd_retval::CMD_RETURN_ERROR;
        }

        if args_has(args, b'p') != 0 {
            if len > 0 && *buf.add(len - 1) == b'\n' as _ {
                len -= 1;
            }
            if (*c).flags.intersects(client_flag::CONTROL) {
                control_write!(c, "{1:0$}", len, _s(buf));
            } else {
                if file_can_print(c) == 0 {
                    cmdq_error(item, c"can't write to client".as_ptr());
                    free_(buf);
                    return cmd_retval::CMD_RETURN_ERROR;
                }
                file_print_buffer(c, buf as _, len);
                file_print!(c, "\n");
                free_(buf);
            }
        } else {
            let mut bufname = null();
            let mut cause = null_mut();
            if args_has(args, b'b') != 0 {
                bufname = args_get(args, b'b');
            }

            if paste_set(buf, len, bufname, &raw mut cause) != 0 {
                cmdq_error(item, c"%s".as_ptr(), cause);
                free_(cause);
                free_(buf);
                return cmd_retval::CMD_RETURN_ERROR;
            }
        }

        cmd_retval::CMD_RETURN_NORMAL
    }
}
