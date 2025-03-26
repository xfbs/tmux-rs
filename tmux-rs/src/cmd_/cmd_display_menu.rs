use compat_rs::queue::tailq_foreach;
use libc::{strcmp, strtol};

use crate::{options_::options_find_choice, *};

#[unsafe(no_mangle)]
static mut cmd_display_menu_entry : cmd_entry = cmd_entry  {
    name : c"display-menu".as_ptr(),
    alias : c"menu".as_ptr(),

    args : args_parse::new( c"b:c:C:H:s:S:MOt:T:x:y:", 1, -1, Some(cmd_display_menu_args_parse)),
    usage : c"[-MO] [-b border-lines] [-c target-client] [-C starting-choice] [-H selected-style] [-s style] [-S border-style] [-t target-pane][-T title] [-x position] [-y position] name key command ...".as_ptr(),
    target : cmd_entry_flag::new( b't', cmd_find_type::CMD_FIND_PANE, 0 ),

    flags : cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_CFLAG),
    exec : Some(cmd_display_menu_exec),
..unsafe{zeroed()}
};

#[unsafe(no_mangle)]
static mut cmd_display_popup_entry : cmd_entry = cmd_entry  {
    name : c"display-popup".as_ptr(),
    alias : c"popup".as_ptr(),

    args : args_parse::new( c"Bb:Cc:d:e:Eh:s:S:t:T:w:x:y:", 0, -1, None ),
    usage : c"[-BCE] [-b border-lines] [-c target-client] [-d start-directory] [-e environment] [-h height] [-s style] [-S border-style] [-t target-pane][-T title] [-w width] [-x position] [-y position] [shell-command]".as_ptr(),
    target : cmd_entry_flag::new( b't', cmd_find_type::CMD_FIND_PANE, 0 ),

    flags : cmd_flag::CMD_AFTERHOOK.union(cmd_flag::CMD_CLIENT_CFLAG),
    exec : Some(cmd_display_popup_exec),
..unsafe{zeroed()}
};

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_display_menu_args_parse(
    args: *mut args,
    idx: u32,
    cause: *mut *mut c_char,
) -> args_parse_type {
    let mut i: u32 = 0;
    let mut type_ = args_parse_type::ARGS_PARSE_STRING;

    loop {
        type_ = args_parse_type::ARGS_PARSE_STRING;
        if (i == idx) {
            break;
        }

        unsafe {
            if *args_string(args, i) == b'\0' as _ {
                i += 1;
                continue;
            }
            i += 1;
        }

        type_ = args_parse_type::ARGS_PARSE_STRING;
        if (i == idx) {
            break;
        }
        i += 1;

        type_ = args_parse_type::ARGS_PARSE_COMMANDS_OR_STRING;
        if (i == idx) {
            break;
        }
        i += 1;
    }
    type_
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_display_menu_get_position(
    tc: *mut client,
    item: *mut cmdq_item,
    args: *mut args,
    px: *mut u32,
    py: *mut u32,
    w: u32,
    h: u32,
) -> i32 {
    unsafe {
        let mut tty = &raw mut (*tc).tty;
        let mut target = cmdq_get_target(item);
        let mut event = cmdq_get_event(item);
        let mut s = (*tc).session;
        let mut wl = (*target).wl;
        let mut wp = (*target).wp;
        let mut ranges = null_mut();
        let mut sr = null_mut();
        //const char		*xp, *yp;
        //char			*p;
        //int			 top;
        //u_int			 line, ox, oy, sx, sy, lines, position;
        let mut line: u32 = 0;
        let mut ox: u32 = 0;
        let mut oy: u32 = 0;
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;
        //long			 n;
        let mut n: c_long = 0;
        //struct format_tree	*ft;

        /*
         * Work out the position from the -x and -y arguments. This is the
         * bottom-left position.
         */

        /* If the popup is too big, stop now. */
        if (w > (*tty).sx || h > (*tty).sy) {
            return (0);
        }

        /* Create format with mouse position if any. */
        let ft = format_create_from_target(item);
        if ((*event).m.valid != 0) {
            format_add(ft, c"popup_mouse_x".as_ptr(), c"%u".as_ptr(), (*event).m.x);
            format_add(ft, c"popup_mouse_y".as_ptr(), c"%u".as_ptr(), (*event).m.y);
        }

        /*
         * If there are any status lines, add this window position and the
         * status line position.
         */
        let mut top = status_at_line(tc);
        if (top != -1) {
            let mut lines = status_line_size(tc);
            if (top == 0) {
                top = lines as i32;
            } else {
                top = 0;
            }
            let mut position = options_get_number((*s).options, c"status-position".as_ptr());

            for line_ in 0..lines {
                line = line_;
                ranges = &raw mut (*tc).status.entries[line as usize].ranges;
                for sr_ in compat_rs::queue::tailq_foreach(ranges) {
                    sr = sr_.as_ptr();
                    if ((*sr).type_ != style_range_type::STYLE_RANGE_WINDOW) {
                        continue;
                    }
                    if ((*sr).argument == (*wl).idx as u32) {
                        break;
                    }
                    continue;
                }
                if (!sr.is_null()) {
                    break;
                }
            }

            if (!sr.is_null()) {
                format_add(ft, c"popup_window_status_line_x".as_ptr(), c"%u".as_ptr(), (*sr).start);
                if (position == 0) {
                    format_add(ft, c"popup_window_status_line_y".as_ptr(), c"%u".as_ptr(), line + 1 + h);
                } else {
                    format_add(
                        ft,
                        c"popup_window_status_line_y".as_ptr(),
                        c"%u".as_ptr(),
                        (*tty).sy - lines + line,
                    );
                }
            }

            if (position == 0) {
                format_add(ft, c"popup_status_line_y".as_ptr(), c"%u".as_ptr(), lines + h);
            } else {
                format_add(ft, c"popup_status_line_y".as_ptr(), c"%u".as_ptr(), (*tty).sy - lines);
            }
        } else {
            top = 0;
        }

        /* Popup width and height. */
        format_add(ft, c"popup_width".as_ptr(), c"%u".as_ptr(), w);
        format_add(ft, c"popup_height".as_ptr(), c"%u".as_ptr(), h);

        /* Position so popup is in the centre. */
        n = ((*tty).sx - 1) as c_long / 2 - w as c_long / 2;
        if (n < 0) {
            format_add(ft, c"popup_centre_x".as_ptr(), c"%u".as_ptr(), 0);
        } else {
            format_add(ft, c"popup_centre_x".as_ptr(), c"%ld".as_ptr(), n);
        }
        n = (((*tty).sy - 1) / 2 + h / 2) as i64;
        if (n >= (*tty).sy as i64) {
            format_add(ft, c"popup_centre_y".as_ptr(), c"%u".as_ptr(), (*tty).sy - h);
        } else {
            format_add(ft, c"popup_centre_y".as_ptr(), c"%ld".as_ptr(), n);
        }

        /* Position of popup relative to mouse. */
        if ((*event).m.valid != 0) {
            n = (*event).m.x as c_long - w as c_long / 2;
            if (n < 0) {
                format_add(ft, c"popup_mouse_centre_x".as_ptr(), c"%u".as_ptr(), 0);
            } else {
                format_add(ft, c"popup_mouse_centre_x".as_ptr(), c"%ld".as_ptr(), n);
            }
            n = ((*event).m.y - h / 2) as i64;
            if (n + h as c_long >= (*tty).sy as i64) {
                format_add(ft, c"popup_mouse_centre_y".as_ptr(), c"%u".as_ptr(), (*tty).sy - h);
            } else {
                format_add(ft, c"popup_mouse_centre_y".as_ptr(), c"%ld".as_ptr(), n);
            }
            n = (*event).m.y as c_long + h as c_long;
            if (n >= (*tty).sy as c_long) {
                format_add(ft, c"popup_mouse_top".as_ptr(), c"%u".as_ptr(), (*tty).sy - 1);
            } else {
                format_add(ft, c"popup_mouse_top".as_ptr(), c"%ld".as_ptr(), n);
            }
            n = ((*event).m.y - h) as c_long;
            if (n < 0) {
                format_add(ft, c"popup_mouse_bottom".as_ptr(), c"%u".as_ptr(), 0);
            } else {
                format_add(ft, c"popup_mouse_bottom".as_ptr(), c"%ld".as_ptr(), n);
            }
        }

        /* Position in pane. */
        tty_window_offset(&raw mut (*tc).tty, &raw mut ox, &raw mut oy, &raw mut sx, &raw mut sy);
        n = top as i64 + (*wp).yoff as i64 - oy as i64 + h as i64;
        if (n >= (*tty).sy as i64) {
            format_add(ft, c"popup_pane_top".as_ptr(), c"%u".as_ptr(), (*tty).sy - h);
        } else {
            format_add(ft, c"popup_pane_top".as_ptr(), c"%ld".as_ptr(), n);
        }
        format_add(
            ft,
            c"popup_pane_bottom".as_ptr(),
            c"%u".as_ptr(),
            top + (*wp).yoff as i32 + (*wp).sy as i32 - oy as i32,
        );
        format_add(ft, c"popup_pane_left".as_ptr(), c"%u".as_ptr(), (*wp).xoff - ox);
        n = (*wp).xoff as c_long + (*wp).sx as i64 - ox as i64 - w as i64;
        if (n < 0) {
            format_add(ft, c"popup_pane_right".as_ptr(), c"%u".as_ptr(), 0);
        } else {
            format_add(ft, c"popup_pane_right".as_ptr(), c"%ld".as_ptr(), n);
        }

        /* Expand horizontal position. */
        let mut xp = args_get_(args, 'x');
        if (xp.is_null() || strcmp(xp, c"C".as_ptr()) == 0) {
            xp = c"#{popup_centre_x}".as_ptr();
        } else if (strcmp(xp, c"R".as_ptr()) == 0) {
            xp = c"#{popup_pane_right}".as_ptr();
        } else if (strcmp(xp, c"P".as_ptr()) == 0) {
            xp = c"#{popup_pane_left}".as_ptr();
        } else if (strcmp(xp, c"M".as_ptr()) == 0) {
            xp = c"#{popup_mouse_centre_x}".as_ptr();
        } else if (strcmp(xp, c"W".as_ptr()) == 0) {
            xp = c"#{popup_window_status_line_x}".as_ptr();
        }
        let p = format_expand(ft, xp);
        n = strtol(p, null_mut(), 10);
        if (n + w as i64 >= (*tty).sx as i64) {
            n = (*tty).sx as i64 - w as i64;
        } else if (n < 0) {
            n = 0;
        }
        *px = n as u32;
        log_debug!(
            "{}: -x: {} = {} = {} (-w {})",
            "cmd_display_menu_get_position",
            _s(xp),
            _s(p),
            *px,
            w,
        );
        free_(p);

        /* Expand vertical position  */
        let mut yp = args_get_(args, 'y');
        if (yp.is_null() || strcmp(yp, c"C".as_ptr()) == 0) {
            yp = c"#{popup_centre_y}".as_ptr();
        } else if (strcmp(yp, c"P".as_ptr()) == 0) {
            yp = c"#{popup_pane_bottom}".as_ptr();
        } else if (strcmp(yp, c"M".as_ptr()) == 0) {
            yp = c"#{popup_mouse_top}".as_ptr();
        } else if (strcmp(yp, c"S".as_ptr()) == 0) {
            yp = c"#{popup_status_line_y}".as_ptr();
        } else if (strcmp(yp, c"W".as_ptr()) == 0) {
            yp = c"#{popup_window_status_line_y}".as_ptr();
        }
        let mut p = format_expand(ft, yp);
        n = strtol(p, null_mut(), 10);
        if (n < h as i64) {
            n = 0;
        } else {
            n -= h as i64;
        }
        if (n + h as i64 >= (*tty).sy as i64) {
            n = (*tty).sy as i64 - h as i64;
        } else if (n < 0) {
            n = 0;
        }
        *py = n as u32;
        log_debug!(
            "{}: -y: {} = {} = {} (-h {})",
            "cmd_display_menu_get_position",
            _s(yp),
            _s(p),
            *py,
            h,
        );
        free_(p);

        format_free(ft);
        1
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_display_menu_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut event = cmdq_get_event(item);
        let mut tc = cmdq_get_target_client(item);
        let mut menu = null_mut();
        let mut menu_item: menu_item = zeroed();
        let mut key = null();
        let mut name = null();

        let mut style = args_get_(args, 's');
        let mut border_style = args_get_(args, 'S');
        let mut selected_style = args_get_(args, 'H');
        let mut lines = box_lines::BOX_LINES_DEFAULT;
        let mut title;
        let mut cause = null_mut();
        let mut flags = 0;
        let mut starting_choice: i32 = 0;
        let mut px: u32 = 0;
        let mut py: u32 = 0;
        let mut i: u32 = 0;
        let mut count = args_count(args);
        let mut o = (*(*(*(*target).s).curw).window).options;

        if (!(*tc).overlay_draw.is_none()) {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        if (args_has_(args, 'C')) {
            if (strcmp(args_get(args, b'C'), c"-".as_ptr()) == 0) {
                starting_choice = -1;
            } else {
                starting_choice = args_strtonum(args, b'C', 0, u32::MAX as i64, &raw mut cause) as i32;
                if (!cause.is_null()) {
                    cmdq_error(item, c"starting choice %s".as_ptr(), cause);
                    free_(cause);
                    return (cmd_retval::CMD_RETURN_ERROR);
                }
            }
        }

        title = if (args_has_(args, 'T')) {
            format_single_from_target(item, args_get(args, b'T'))
        } else {
            xstrdup_(c"").as_ptr()
        };
        menu = menu_create(title);
        free_(title);

        i = 0;
        while i != count {
            name = args_string(args, i);
            i += 1;
            if (*name == b'\0' as _) {
                menu_add_item(menu, null_mut(), item, tc, target);
                continue;
            }

            if (count - i < 2) {
                cmdq_error(item, c"not enough arguments".as_ptr());
                menu_free(menu);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
            key = args_string(args, i);
            i += 1;

            menu_item.name = name;
            menu_item.key = key_string_lookup_string(key);
            menu_item.command = args_string(args, i);
            i += 1;

            menu_add_item(menu, &raw mut menu_item, item, tc, target);
        }
        if (menu.is_null()) {
            cmdq_error(item, c"invalid menu arguments".as_ptr());
            return (cmd_retval::CMD_RETURN_ERROR);
        }
        if ((*menu).count == 0) {
            menu_free(menu);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        if cmd_display_menu_get_position(
            tc,
            item,
            args,
            &raw mut px,
            &raw mut py,
            (*menu).width + 4,
            (*menu).count + 2,
        ) == 0
        {
            menu_free(menu);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        let value = args_get_(args, 'b');
        if (!value.is_null()) {
            let oe = options_get(o, c"menu-border-lines".as_ptr());
            let lines = options_find_choice(options_table_entry(oe), value, &raw mut cause);
            if (lines == -1) {
                cmdq_error(item, c"menu-border-lines %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        if (args_has_(args, 'O')) {
            flags |= MENU_STAYOPEN;
        }
        if (!(*event).m.valid != 0 && !args_has_(args, 'M')) {
            flags |= MENU_NOMOUSE;
        }
        if (menu_display(
            menu,
            flags,
            starting_choice,
            item,
            px,
            py,
            tc,
            lines,
            style,
            selected_style,
            border_style,
            target,
            None,
            null_mut(),
        ) != 0)
        {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        cmd_retval::CMD_RETURN_WAIT
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn cmd_display_popup_exec(self_: *mut cmd, item: *mut cmdq_item) -> cmd_retval {
    unsafe {
        let mut args = cmd_get_args(self_);
        let mut target = cmdq_get_target(item);
        let mut s = (*target).s;
        let mut tc = cmdq_get_target_client(item);
        let mut tty = &raw mut (*tc).tty;
        //const char		*value, *shell, *shellcmd = NULL;
        let mut style = args_get(args, b's');
        let mut border_style = args_get(args, b'S');
        let mut cause: *mut c_char = null_mut();
        //char			*cwd, *cause = NULL, **argv = NULL, *title;
        let mut argc = 0;
        let mut lines = box_lines::BOX_LINES_DEFAULT as i32;
        let mut px = 0;
        let mut py = 0;
        let mut w: i32 = 0;
        let mut h: u32 = 0;
        let mut count = args_count(args);
        //struct args_value	*av;
        let mut env = null_mut();
        let mut o = (*(*(*s).curw).window).options;
        // struct options_entry	*oe;

        if (args_has_(args, 'C')) {
            server_client_clear_overlay(tc);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        if !(*tc).overlay_draw.is_none() {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        h = (*tty).sy / 2;
        if (args_has_(args, 'h')) {
            h = args_percentage(args, b'h', 1, (*tty).sy as i64, (*tty).sy as i64, &raw mut cause) as u32;
            if (!cause.is_null()) {
                cmdq_error(item, c"height %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        let mut w = (*tty).sx / 2;
        if (args_has_(args, 'w')) {
            w = args_percentage(args, b'w', 1, (*tty).sx as i64, (*tty).sx as i64, &raw mut cause) as u32;
            if (!cause.is_null()) {
                cmdq_error(item, c"width %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        if (w > (*tty).sx) {
            w = (*tty).sx;
        }
        if (h > (*tty).sy) {
            h = (*tty).sy;
        }
        if (cmd_display_menu_get_position(tc, item, args, &raw mut px, &raw mut py, w, h) == 0) {
            return (cmd_retval::CMD_RETURN_NORMAL);
        }

        let mut value = args_get(args, b'b');
        if (args_has_(args, 'B')) {
            lines = box_lines::BOX_LINES_NONE as i32;
        } else if (!value.is_null()) {
            let oe = options_get(o, c"popup-border-lines".as_ptr());
            lines = options_find_choice(options_table_entry(oe), value, &raw mut cause);
            if (!cause.is_null()) {
                cmdq_error(item, c"popup-border-lines %s".as_ptr(), cause);
                free_(cause);
                return (cmd_retval::CMD_RETURN_ERROR);
            }
        }

        value = args_get(args, b'd');
        let mut cwd = if (!value.is_null()) {
            format_single_from_target(item, value)
        } else {
            xstrdup(server_client_get_cwd(tc, s)).as_ptr()
        };
        let mut shellcmd = null();
        if (count == 0) {
            shellcmd = options_get_string((*s).options, c"default-command".as_ptr());
        } else if (count == 1) {
            shellcmd = args_string(args, 0);
        }

        let mut shell = null();
        let mut argv = null_mut();

        if (count <= 1 && (shellcmd.is_null() || *shellcmd == b'\0' as _)) {
            shellcmd = null_mut();
            shell = options_get_string((*s).options, c"default-shell".as_ptr());
            if (checkshell(shell) == 0) {
                shell = _PATH_BSHELL;
            }
            cmd_append_argv(&raw mut argc, &raw mut argv, shell);
        } else {
            args_to_vector(args, &raw mut argc, &raw mut argv);
        }

        if (args_has(args, b'e') >= 1) {
            env = environ_create().as_ptr();
            let mut av = args_first_value(args, b'e');
            while (!av.is_null()) {
                environ_put(env, (*av).union_.string, 0);
                av = args_next_value(av);
            }
        }

        let mut title = if args_has_(args, 'T') {
            format_single_from_target(item, args_get(args, b'T'))
        } else {
            xstrdup_(c"").as_ptr()
        };
        let mut flags = 0;
        if (args_has(args, b'E') > 1) {
            flags |= POPUP_CLOSEEXITZERO;
        } else if (args_has_(args, 'E')) {
            flags |= POPUP_CLOSEEXIT;
        }
        if (popup_display(
            flags,
            std::mem::transmute::<_, box_lines>(lines),
            item,
            px,
            py,
            w,
            h,
            env,
            shellcmd,
            argc,
            argv,
            cwd,
            title,
            tc,
            s,
            style,
            border_style,
            None,
            null_mut(),
        ) != 0)
        {
            cmd_free_argv(argc, argv);
            if (!env.is_null()) {
                environ_free(env);
            }
            free_(cwd);
            free_(title);
            return (cmd_retval::CMD_RETURN_NORMAL);
        }
        if (!env.is_null()) {
            environ_free(env);
        }
        free_(cwd);
        free_(title);
        cmd_free_argv(argc, argv);

        cmd_retval::CMD_RETURN_WAIT
    }
}
