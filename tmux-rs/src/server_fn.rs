use super::*;

use libc::{WEXITSTATUS, WIFEXITED, close, gettimeofday, memcpy};

use crate::compat::{
    imsg::{IMSG_HEADER_SIZE, MAX_IMSGSIZE},
    queue::{tailq_empty, tailq_foreach},
    tree::rb_foreach,
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_redraw_client(c: *mut client) {
    unsafe {
        (*c).flags |= CLIENT_ALLREDRAWFLAGS;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_status_client(c: *mut client) {
    unsafe {
        (*c).flags |= client_flag::REDRAWSTATUS;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_redraw_session(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if ((*c).session == s) {
                server_redraw_client(c);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_redraw_session_group(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            server_redraw_session(s);
        } else {
            for s in tailq_foreach(&raw mut (*sg).sessions) {
                server_redraw_session(s.as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_status_session(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if ((*c).session == s) {
                server_status_client(c);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_status_session_group(s: *mut session) {
    unsafe {
        let sg = session_group_contains(s);
        if sg.is_null() {
            server_status_session(s);
        } else {
            for s in tailq_foreach(&raw mut (*sg).sessions) {
                server_status_session(s.as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_redraw_window(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if (!(*c).session.is_null() && (*(*(*c).session).curw).window == w) {
                server_redraw_client(c);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_redraw_window_borders(w: *mut window) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if (!(*c).session.is_null() && (*(*(*c).session).curw).window == w) {
                (*c).flags |= client_flag::REDRAWBORDERS;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_status_window(w: *mut window) {
    unsafe {
        /*
         * This is slightly different. We want to redraw the status line of any
         * clients containing this window rather than anywhere it is the
         * current window.
         */

        for s in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            if session_has(s, w) != 0 {
                server_status_session(s);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_lock() {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if (!(*c).session.is_null()) {
                server_lock_client(c);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_lock_session(s: *mut session) {
    unsafe {
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if ((*c).session == s) {
                server_lock_client(c);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_lock_client(c: *mut client) {
    unsafe {
        if (*c).flags.intersects(client_flag::CONTROL) {
            return;
        }

        if (*c).flags.intersects(client_flag::SUSPENDED) {
            return;
        }

        let cmd = options_get_string((*(*c).session).options, c"lock-command".as_ptr());
        if (*cmd == b'\0' as c_char || strlen(cmd) + 1 > MAX_IMSGSIZE - IMSG_HEADER_SIZE) {
            return;
        }

        tty_stop_tty(&raw mut (*c).tty);
        tty_raw(
            &raw mut (*c).tty,
            tty_term_string((*c).tty.term, tty_code_code::TTYC_SMCUP),
        );
        tty_raw(
            &raw mut (*c).tty,
            tty_term_string((*c).tty.term, tty_code_code::TTYC_CLEAR),
        );
        tty_raw(
            &raw mut (*c).tty,
            tty_term_string((*c).tty.term, tty_code_code::TTYC_E3),
        );

        (*c).flags |= client_flag::SUSPENDED;
        proc_send(
            (*c).peer,
            msgtype::MSG_LOCK,
            -1,
            cmd.cast(),
            strlen(cmd) + 1,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_kill_pane(wp: *mut window_pane) {
    unsafe {
        let w = (*wp).window;

        if (window_count_panes(w) == 1) {
            server_kill_window(w, 1);
            recalculate_sizes();
        } else {
            server_unzoom_window(w);
            server_client_remove_pane(wp);
            layout_close_pane(wp);
            window_remove_pane(w, wp);
            server_redraw_window(w);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_kill_window(w: *mut window, renumber: i32) {
    unsafe {
        for s in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            if session_has(s, w) == 0 {
                continue;
            }

            server_unzoom_window(w);
            while let Some(wl) = winlink_find_by_window(&raw mut (*s).windows, w) {
                if session_detach(s, wl.as_ptr()) != 0 {
                    server_destroy_session_group(s);
                    break;
                }
                server_redraw_session_group(s);
            }

            if (renumber != 0) {
                server_renumber_session(s);
            }
        }

        recalculate_sizes();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_renumber_session(s: *mut session) {
    unsafe {
        if options_get_number((*s).options, c"renumber-windows".as_ptr()) != 0 {
            let sg = session_group_contains(s);
            if (!sg.is_null()) {
                for s in tailq_foreach(&raw mut (*sg).sessions) {
                    session_renumber_windows(s.as_ptr());
                }
            } else {
                session_renumber_windows(s);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_renumber_all() {
    unsafe {
        for s in rb_foreach(&raw mut sessions) {
            server_renumber_session(s.as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_link_window(
    src: *mut session,
    srcwl: *mut winlink,
    dst: *mut session,
    mut dstidx: i32,
    killflag: i32,
    mut selectflag: i32,
    cause: *mut *mut c_char,
) -> i32 {
    unsafe {
        let mut dstwl = null_mut();

        let srcsg = session_group_contains(src);
        let dstsg = session_group_contains(dst);
        if (src != dst && !srcsg.is_null() && !dstsg.is_null() && srcsg == dstsg) {
            xasprintf(cause, c"sessions are grouped".as_ptr());
            return -1;
        }

        if (dstidx != -1) {
            dstwl = winlink_find_by_index(&raw mut (*dst).windows, dstidx);
        }
        if !dstwl.is_null() {
            if ((*dstwl).window == (*srcwl).window) {
                xasprintf(cause, c"same index: %d".as_ptr(), dstidx);
                return -1;
            }
            if (killflag != 0) {
                /*
                 * Can't use session_detach as it will destroy session
                 * if this makes it empty.
                 */
                notify_session_window(c"window-unlinked".as_ptr(), dst, (*dstwl).window);
                (*dstwl).flags &= !WINLINK_ALERTFLAGS;
                winlink_stack_remove(&raw mut (*dst).lastw, dstwl);
                winlink_remove(&raw mut (*dst).windows, dstwl);

                /* Force select/redraw if current. */
                if (dstwl == (*dst).curw) {
                    selectflag = 1;
                    (*dst).curw = null_mut();
                }
            }
        }

        if (dstidx == -1) {
            dstidx = -1 - options_get_number((*dst).options, c"base-index".as_ptr()) as i32;
        }
        dstwl = session_attach(dst, (*srcwl).window, dstidx, cause);
        if (dstwl.is_null()) {
            return -1;
        }

        if selectflag != 0 {
            session_select(dst, (*dstwl).idx);
        }
        server_redraw_session_group(dst);

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_unlink_window(s: *mut session, wl: *mut winlink) {
    unsafe {
        if session_detach(s, wl) != 0 {
            server_destroy_session_group(s);
        } else {
            server_redraw_session_group(s);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_destroy_pane(wp: *mut window_pane, notify: i32) {
    unsafe {
        let mut w = (*wp).window;
        let mut ctx: MaybeUninit<screen_write_ctx> = MaybeUninit::<screen_write_ctx>::uninit();
        let ctx = ctx.as_mut_ptr();

        let mut gc: MaybeUninit<grid_cell> = MaybeUninit::<grid_cell>::uninit();
        let gc = gc.as_mut_ptr();

        let mut sx = screen_size_x(&raw mut (*wp).base);
        let mut sy = screen_size_y(&raw mut (*wp).base);

        if ((*wp).fd != -1) {
            #[cfg(feature = "utempter")]
            {
                utempter_remove_record((*wp).fd);
            }
            bufferevent_free((*wp).event);
            (*wp).event = null_mut();
            close((*wp).fd);
            (*wp).fd = -1;
        }

        let mut remain_on_exit = options_get_number((*wp).options, c"remain-on-exit".as_ptr());
        if (remain_on_exit != 0 && !(*wp).flags.intersects(window_pane_flags::PANE_STATUSREADY)) {
            return;
        }
        'out: {
            match (remain_on_exit) {
                0 => (),
                1 | 2 => {
                    if remain_on_exit == 2 {
                        if (WIFEXITED((*wp).status) && WEXITSTATUS((*wp).status) == 0) {
                            break 'out;
                        }
                    }
                    if (*wp).flags.intersects(window_pane_flags::PANE_STATUSDRAWN) {
                        return;
                    }
                    (*wp).flags |= window_pane_flags::PANE_STATUSDRAWN;

                    gettimeofday(&raw mut (*wp).dead_time, null_mut());
                    if (notify != 0) {
                        notify_pane(c"pane-died".as_ptr(), wp);
                    }

                    let mut s =
                        options_get_string((*wp).options, c"remain-on-exit-format".as_ptr());
                    if *s != '\0' as c_char {
                        screen_write_start_pane(ctx, wp, &raw mut (*wp).base);
                        screen_write_scrollregion(ctx, 0, sy - 1);
                        screen_write_cursormove(ctx, 0, sy as i32 - 1, 0);
                        screen_write_linefeed(ctx, 1, 8);
                        memcpy_(gc, &raw const grid_default_cell, size_of::<grid_cell>());

                        let expanded =
                            format_single(null_mut(), s, null_mut(), null_mut(), null_mut(), wp);
                        format_draw(ctx, gc, sx, expanded, null_mut(), 0);
                        free_(expanded);

                        screen_write_stop(ctx);
                    }
                    (*wp).base.mode &= !MODE_CURSOR;

                    (*wp).flags |= window_pane_flags::PANE_REDRAW;
                    return;
                }
                _ => (),
            }
        }

        if (notify != 0) {
            notify_pane(c"pane-exited".as_ptr(), wp);
        }

        server_unzoom_window(w);
        server_client_remove_pane(wp);
        layout_close_pane(wp);
        window_remove_pane(w, wp);

        if tailq_empty(&raw mut (*w).panes) {
            server_kill_window(w, 1);
        } else {
            server_redraw_window(w);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_destroy_session_group(s: *mut session) {
    unsafe {
        let mut sg = session_group_contains(s);
        if (sg.is_null()) {
            server_destroy_session(s);
            session_destroy(s, 1, c"server_destroy_session_group".as_ptr());
        } else {
            for s in tailq_foreach(&raw mut (*sg).sessions).map(NonNull::as_ptr) {
                server_destroy_session(s);
                session_destroy(s, 1, c"server_destroy_session_group".as_ptr());
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_find_session(
    s: *mut session,
    f: unsafe extern "C" fn(*mut session, *mut session) -> i32,
) -> *mut session {
    unsafe {
        let mut s_out: *mut session = null_mut();
        for s_loop in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            if (s_loop != s && (s_out.is_null() || f(s_loop, s_out) != 0)) {
                s_out = s_loop;
            }
        }
        s_out
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_newer_session(s_loop: *mut session, s_out: *mut session) -> i32 {
    unsafe {
        (timer::new(&raw const (*s_loop).activity_time)
            > timer::new(&raw const (*s_out).activity_time)) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_newer_detached_session(
    s_loop: *mut session,
    s_out: *mut session,
) -> i32 {
    unsafe {
        if (*s_loop).attached != 0 {
            return 0;
        }
        server_newer_session(s_loop, s_out)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_destroy_session(s: *mut session) {
    unsafe {
        let detach_on_destroy = options_get_number((*s).options, c"detach-on-destroy".as_ptr());

        let mut s_new: *mut session = if (detach_on_destroy == 0) {
            server_find_session(s, server_newer_session)
        } else if (detach_on_destroy == 2) {
            server_find_session(s, server_newer_detached_session)
        } else if (detach_on_destroy == 3) {
            session_previous_session(s)
        } else if (detach_on_destroy == 4) {
            session_next_session(s)
        } else {
            null_mut()
        };

        if (s_new == s) {
            s_new = null_mut()
        }
        for c in tailq_foreach(&raw mut clients).map(NonNull::as_ptr) {
            if ((*c).session != s) {
                continue;
            }
            (*c).session = null_mut();
            (*c).last_session = null_mut();
            server_client_set_session(c, s_new);
            if (s_new.is_null()) {
                (*c).flags |= client_flag::EXIT;
            }
        }
        recalculate_sizes();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_check_unattached() {
    unsafe {
        for s in rb_foreach(&raw mut sessions).map(NonNull::as_ptr) {
            if ((*s).attached != 0) {
                continue;
            }
            match options_get_number((*s).options, c"destroy-unattached".as_ptr()) {
                0 => continue, // off
                1 => (),       // on
                2 => {
                    /* keep-last */
                    let sg = session_group_contains(s);
                    if (sg.is_null() || session_group_count(sg) <= 1) {
                        continue;
                    }
                }
                3 => {
                    /* keep-group */
                    let sg = session_group_contains(s);
                    if (!sg.is_null() && session_group_count(sg) == 1) {
                        continue;
                    }
                }
                _ => (),
            }
            session_destroy(s, 1, c"server_check_unattached".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn server_unzoom_window(w: *mut window) {
    unsafe {
        if (window_unzoom(w, 1) == 0) {
            server_redraw_window(w);
        }
    }
}
