use crate::colour::colour_split_rgb_;

use super::*;

#[rustfmt::skip]
unsafe extern "C" {
    // pub fn tty_create_log();
    pub fn tty_window_bigger(_: *mut tty) -> c_int;
    pub fn tty_window_offset( _: *mut tty, _: *mut c_uint, _: *mut c_uint, _: *mut c_uint, _: *mut c_uint,) -> c_int;
    pub fn tty_update_window_offset(_: *mut window);
    pub fn tty_update_client_offset(_: *mut client);
    // pub fn tty_raw(_: *mut tty, _: *const c_char);
    pub fn tty_attributes( _: *mut tty, _: *const grid_cell, _: *const grid_cell, _: *mut colour_palette, _: *mut hyperlinks,);
    pub fn tty_reset(_: *mut tty);
    pub fn tty_region_off(_: *mut tty);
    pub fn tty_m_in_off(_: *mut tty);
    pub fn tty_cursor(_: *mut tty, _: c_uint, _: c_uint);
    pub fn tty_clipboard_query(_: *mut tty);
    // pub fn tty_putcode(_: *mut tty, _: tty_code_code);
    // pub fn tty_putcode_i(_: *mut tty, _: tty_code_code, _: c_int);
    // pub fn tty_putcode_ii(_: *mut tty, _: tty_code_code, _: c_int, _: c_int);
    // pub fn tty_putcode_iii(_: *mut tty, _: tty_code_code, _: c_int, _: c_int, _: c_int);
    // pub fn tty_putcode_s(_: *mut tty, _: tty_code_code, _: *const c_char);
    // pub fn tty_putcode_ss(_: *mut tty, _: tty_code_code, _: *const c_char, _: *const c_char);
    // pub fn tty_puts(_: *mut tty, _: *const c_char);
    // pub fn tty_putc(_: *mut tty, _: c_uchar);
    // pub fn tty_putn(_: *mut tty, _: *const c_void, _: usize, _: c_uint);
    pub fn tty_cell( _: *mut tty, _: *const grid_cell, _: *const grid_cell, _: *mut colour_palette, _: *mut hyperlinks,);
    // pub fn tty_init(_: *mut tty, _: *mut client) -> c_int;
    // pub fn tty_resize(_: *mut tty);
    // pub fn tty_set_size(_: *mut tty, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    // pub fn tty_start_tty(_: *mut tty);
    // pub fn tty_send_requests(_: *mut tty);
    // pub fn tty_repeat_requests(_: *mut tty);
    // pub fn tty_stop_tty(_: *mut tty);
    // pub fn tty_set_title(_: *mut tty, _: *const c_char);
    // pub fn tty_set_path(_: *mut tty, _: *const c_char);
    pub fn tty_update_mode(_: *mut tty, _: c_int, _: *mut screen);
    pub fn tty_draw_line( _: *mut tty, _: *mut screen, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: c_uint, _: *const grid_cell, _: *mut colour_palette,);
    pub fn tty_sync_start(_: *mut tty);
    pub fn tty_sync_end(_: *mut tty);
    // pub fn tty_open(_: *mut tty, _: *mut *mut c_char) -> c_int;
    // pub fn tty_close(_: *mut tty);
    // pub fn tty_free(_: *mut tty);
    // pub fn tty_update_features(_: *mut tty);
    pub fn tty_set_selection(_: *mut tty, _: *const c_char, _: *const c_char, _: usize);
    pub fn tty_write( _: Option<unsafe extern "C" fn(_: *mut tty, _: *const tty_ctx)>, _: *mut tty_ctx,);
    pub fn tty_cmd_alignmenttest(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_cell(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_cells(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearendofline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearendofscreen(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearscreen(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearstartofline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearstartofscreen(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_deletecharacter(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_clearcharacter(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_deleteline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_insertcharacter(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_insertline(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_linefeed(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_scrollup(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_scrolldown(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_reverseindex(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_setselection(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_rawstring(_: *mut tty, _: *const tty_ctx);
    pub fn tty_cmd_syncstart(_: *mut tty, _: *const tty_ctx);
    pub fn tty_default_colours(_: *mut grid_cell, _: *mut window_pane);

    pub fn tty_margin_off(tty: *mut tty);
}

#[unsafe(no_mangle)]
static mut tty_log_fd: i32 = -1;

unsafe extern "C" {
    pub fn tty_invalidate(tty: *mut tty);
}
/*
static void tty_set_italics(struct tty *);
static int tty_try_colour(struct tty *, int, const char *);
static void tty_force_cursor_colour(struct tty *, int);
static void tty_cursor_pane(struct tty *, const struct tty_ctx *, u_int, u_int);
static void tty_cursor_pane_unless_wrap(struct tty *, const struct tty_ctx *, u_int, u_int);
static void tty_invalidate(struct tty *);

static void tty_colours(struct tty *, const struct grid_cell *);
static void tty_check_fg(struct tty *, struct colour_palette *, struct grid_cell *);
static void tty_check_bg(struct tty *, struct colour_palette *, struct grid_cell *);
static void tty_check_us(struct tty *, struct colour_palette *, struct grid_cell *);
static void tty_colours_fg(struct tty *, const struct grid_cell *);
static void tty_colours_bg(struct tty *, const struct grid_cell *);
static void tty_colours_us(struct tty *, const struct grid_cell *);

static void tty_region_pane(struct tty *, const struct tty_ctx *, u_int, u_int);
static void tty_region(struct tty *, u_int, u_int);
static void tty_margin_pane(struct tty *, const struct tty_ctx *);
static void tty_margin(struct tty *, u_int, u_int);
static int tty_large_region(struct tty *, const struct tty_ctx *);
static int tty_fake_bce(const struct tty *, const struct grid_cell *, u_int);
static void tty_redraw_region(struct tty *, const struct tty_ctx *);
static void tty_emulate_repeat(struct tty *, enum tty_code_code, enum tty_code_code, u_int);
static void tty_repeat_space(struct tty *, u_int);
static void tty_draw_pane(struct tty *, const struct tty_ctx *, u_int);
static void tty_default_attributes(struct tty *, const struct grid_cell *, struct colour_palette *, u_int, struct hyperlinks *);
static int tty_check_overlay(struct tty *, u_int, u_int);
static void tty_check_overlay_range(struct tty *, u_int, u_int, u_int, struct overlay_ranges *);

#ifdef ENABLE_SIXEL
static void tty_write_one(void (*)(struct tty *, const struct tty_ctx *), struct client *, struct tty_ctx *);
#endif
*/

#[inline]
unsafe fn tty_use_margin(tty: *const tty) -> bool {
    unsafe { (*(*tty).term).flags.intersects(term_flags::TERM_DECSLRM) }
}

#[inline]
unsafe fn tty_full_width(tty: *const tty, ctx: *const tty_ctx) -> bool {
    unsafe { ((*ctx).xoff == 0 && (*ctx).sx >= (*tty).sx) }
}

const TTY_BLOCK_INTERVAL: usize = 100_000; // 100 millis
const TTY_QUERY_TIMEOUT: i32 = 5;
const TTY_REQUEST_LIMIT: i32 = 30;

#[allow(non_snake_case)]
#[inline]
unsafe fn TTY_BLOCK_START(tty: *const tty) -> u32 {
    unsafe { (1 + ((*tty).sx * (*tty).sy) * 8) }
}

#[allow(non_snake_case)]
#[inline]
unsafe fn TTY_BLOCK_STOP(tty: *const tty) -> u32 {
    unsafe { (1 + ((*tty).sx * (*tty).sy) / 8) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_create_log() {
    unsafe {
        let mut name: [c_char; 64] = [0; 64];

        xsnprintf(
            (&raw mut name).cast(),
            64,
            c"tmux-out-%ld.log".as_ptr(),
            libc::getpid() as i64,
        );

        tty_log_fd = libc::open(
            (&raw const name).cast(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC,
            0o644,
        );
        if (tty_log_fd != -1 && libc::fcntl(tty_log_fd, libc::F_SETFD, libc::FD_CLOEXEC) == -1) {
            fatal(c"fcntl failed".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_init(tty: *mut tty, c: *mut client) -> i32 {
    unsafe {
        if libc::isatty((*c).fd) == 0 {
            return -1;
        }

        memset0(tty);
        (*tty).client = c;

        (*tty).cstyle = screen_cursor_style::SCREEN_CURSOR_DEFAULT;
        (*tty).ccolour = -1;
        (*tty).fg = -1;
        (*tty).bg = -1;

        if (libc::tcgetattr((*c).fd, &raw mut (*tty).tio) != 0) {
            return -1;
        }

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_resize(tty: *mut tty) {
    unsafe {
        let mut c = (*tty).client;
        let mut ws: libc::winsize = zeroed();
        let mut sx: u32 = 0;
        let mut sy: u32 = 0;
        let mut xpixel: u32 = 0;
        let mut ypixel: u32 = 0;

        if libc::ioctl((*c).fd, libc::TIOCGWINSZ, &raw mut ws) != -1 {
            sx = ws.ws_col as u32;
            if (sx == 0) {
                sx = 80;
                xpixel = 0;
            } else {
                xpixel = ws.ws_xpixel as u32 / sx;
            }
            sy = ws.ws_row as u32;
            if (sy == 0) {
                sy = 24;
                ypixel = 0;
            } else {
                ypixel = ws.ws_ypixel as u32 / sy;
            }
        } else {
            sx = 80;
            sy = 24;
            xpixel = 0;
            ypixel = 0;
        }
        // log_debug("%s: %s now %ux%u (%ux%u)", __func__, (*c).name, sx, sy, xpixel, ypixel);
        tty_set_size(tty, sx, sy, xpixel, ypixel);
        tty_invalidate(tty);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_set_size(tty: *mut tty, sx: u32, sy: u32, xpixel: u32, ypixel: u32) {
    unsafe {
        (*tty).sx = sx;
        (*tty).sy = sy;
        (*tty).xpixel = xpixel;
        (*tty).ypixel = ypixel;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_read_callback(_fd: i32, _events: i16, data: *mut c_void) {
    unsafe {
        let mut tty = data as *mut tty;
        let c = (*tty).client;
        let name = (*c).name;
        let size = EVBUFFER_LENGTH((*tty).in_);

        let nread = evbuffer_read((*tty).in_, (*c).fd, -1);
        if (nread == 0 || nread == -1) {
            if (nread == 0) {
                // log_debug!("%s: read closed", name);
            } else {
                // log_debug!("%s: read error: %s", name, strerror(errno!()));
            }
            event_del(&raw mut (*tty).event_in);
            server_client_lost((*tty).client);
            return;
        }
        // log_debug("%s: read %d bytes (already %zu)", name, nread, size);

        while (tty_keys_next(tty) != 0) {}
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_timer_callback(_fd: i32, events: i16, data: *mut c_void) {
    unsafe {
        let mut tty = data as *mut tty;
        let mut c = (*tty).client;
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: TTY_BLOCK_INTERVAL as i64,
        };

        // log_debug("%s: %zu discarded", (*c).name, (*tty).discarded);

        (*c).flags |= CLIENT_ALLREDRAWFLAGS;
        (*c).discarded += (*tty).discarded;

        if (*tty).discarded < TTY_BLOCK_STOP(tty) as usize {
            (*tty).flags &= !tty_flags::TTY_BLOCK;
            tty_invalidate(tty);
            return;
        }
        (*tty).discarded = 0;
        evtimer_add(&raw mut (*tty).timer, &raw mut tv);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_block_maybe(tty: *mut tty) -> i32 {
    unsafe {
        let mut c = (*tty).client;
        let size = EVBUFFER_LENGTH((*tty).out);
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: TTY_BLOCK_INTERVAL as i64,
        };

        if size == 0 {
            (*tty).flags &= !tty_flags::TTY_NOBLOCK;
        } else if (*tty).flags.intersects(tty_flags::TTY_NOBLOCK) {
            return 0;
        }

        if size < TTY_BLOCK_START(tty) as usize {
            return 0;
        }

        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            return 1;
        }
        (*tty).flags |= tty_flags::TTY_BLOCK;

        // log_debug("%s: can't keep up, %zu discarded", (*c).name, size);

        evbuffer_drain((*tty).out, size);
        (*c).discarded += size;

        (*tty).discarded = 0;
        evtimer_add(&raw mut (*tty).timer, &raw const tv);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_write_callback(_fd: i32, _events: i16, data: *mut c_void) {
    unsafe {
        let mut tty = data as *mut tty;
        let mut c = (*tty).client;
        let mut size = EVBUFFER_LENGTH((*tty).out);

        let nwrite: i32 = evbuffer_write((*tty).out, (*c).fd);
        if (nwrite == -1) {
            return;
        }
        // log_debug("%s: wrote %d bytes (of %zu)", (*c).name, nwrite, size);

        if ((*c).redraw > 0) {
            if nwrite as usize >= (*c).redraw {
                (*c).redraw = 0;
            } else {
                (*c).redraw -= nwrite as usize;
            }
            // log_debug("%s: waiting for redraw, %zu bytes left", (*c).name, (*c).redraw);
        } else if tty_block_maybe(tty) != 0 {
            return;
        }

        if (EVBUFFER_LENGTH((*tty).out) != 0) {
            event_add(&raw mut (*tty).event_out, null_mut());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_open(tty: *mut tty, cause: *mut *mut c_char) -> i32 {
    unsafe {
        let mut c = (*tty).client;

        (*tty).term = tty_term_create(
            tty,
            (*c).term_name,
            (*c).term_caps,
            (*c).term_ncaps,
            &raw mut (*c).term_features,
            cause,
        );
        if (*tty).term.is_null() {
            tty_close(tty);
            return -1;
        }
        (*tty).flags |= tty_flags::TTY_OPENED;

        (*tty).flags &= !(tty_flags::TTY_NOCURSOR
            | tty_flags::TTY_FREEZE
            | tty_flags::TTY_BLOCK
            | tty_flags::TTY_TIMER);

        event_set(
            &raw mut (*tty).event_in,
            (*c).fd,
            EV_PERSIST | EV_READ,
            Some(tty_read_callback),
            tty.cast(),
        );
        (*tty).in_ = evbuffer_new();
        if (*tty).in_.is_null() {
            fatal(c"out of memory".as_ptr());
        }

        event_set(
            &raw mut (*tty).event_out,
            (*c).fd,
            EV_WRITE,
            Some(tty_write_callback),
            tty.cast(),
        );
        (*tty).out = evbuffer_new();
        if (*tty).out.is_null() {
            fatal(c"out of memory".as_ptr());
        }

        evtimer_set(&raw mut (*tty).timer, Some(tty_timer_callback), tty.cast());

        tty_start_tty(tty);
        tty_keys_build(tty);

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_start_timer_callback(_fd: i32, _events: i16, data: *mut c_void) {
    unsafe {
        let mut tty = data as *mut tty;
        let mut c = (*tty).client;

        // log_debug("%s: start timer fired", (*c).name);
        if (*tty)
            .flags
            .intersects(tty_flags::TTY_HAVEDA | tty_flags::TTY_HAVEDA2 | tty_flags::TTY_HAVEXDA)
        {
            tty_update_features(tty);
        }
        (*tty).flags |= TTY_ALL_REQUEST_FLAGS;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_start_tty(tty: *mut tty) {
    unsafe {
        let mut c = (*tty).client;
        let mut tio: libc::termios = zeroed();
        let mut tv = libc::timeval {
            tv_sec: TTY_QUERY_TIMEOUT as i64,
            tv_usec: 0,
        };

        setblocking((*c).fd, 0);
        event_add(&raw mut (*tty).event_in, null_mut());

        memcpy__(&raw mut tio, &raw const (*tty).tio);
        tio.c_iflag &= !(libc::IXON
            | libc::IXOFF
            | libc::ICRNL
            | libc::INLCR
            | libc::IGNCR
            | libc::IMAXBEL
            | libc::ISTRIP);
        tio.c_iflag |= libc::IGNBRK;
        tio.c_oflag &= !(libc::OPOST | libc::ONLCR | libc::OCRNL | libc::ONLRET);
        tio.c_lflag &= !(libc::IEXTEN
            | libc::ICANON
            | libc::ECHO
            | libc::ECHOE
            | libc::ECHONL
            | libc::ECHOCTL
            | libc::ECHOPRT
            | libc::ECHOKE
            | libc::ISIG);
        tio.c_cc[libc::VMIN] = 1;
        tio.c_cc[libc::VTIME] = 0;
        if (libc::tcsetattr((*c).fd, libc::TCSANOW, &raw mut tio) == 0) {
            libc::tcflush((*c).fd, libc::TCOFLUSH);
        }

        tty_putcode(tty, tty_code_code::TTYC_SMCUP);

        tty_putcode(tty, tty_code_code::TTYC_SMKX);
        tty_putcode(tty, tty_code_code::TTYC_CLEAR);

        if (tty_acs_needed(tty) != 0) {
            // log_debug("%s: using capabilities for ACS", (*c).name);
            tty_putcode(tty, tty_code_code::TTYC_ENACS);
        } else {
            // log_debug("%s: using UTF-8 for ACS", (*c).name);
        }

        tty_putcode(tty, tty_code_code::TTYC_CNORM);
        if tty_term_has((*tty).term, tty_code_code::TTYC_KMOUS).as_bool() {
            tty_puts(tty, c"\x1b[?1000l\x1b[?1002l\x1b[?1003l".as_ptr());
            tty_puts(tty, c"\x1b[?1006l\x1b[?1005l".as_ptr());
        }
        if tty_term_has((*tty).term, tty_code_code::TTYC_ENBP).as_bool() {
            tty_putcode(tty, tty_code_code::TTYC_ENBP);
        }

        evtimer_set(
            &raw mut (*tty).start_timer,
            Some(tty_start_timer_callback),
            tty.cast(),
        );
        evtimer_add(&raw mut (*tty).start_timer, &raw const tv);

        (*tty).flags |= tty_flags::TTY_STARTED;
        tty_invalidate(tty);

        if ((*tty).ccolour != -1) {
            tty_force_cursor_colour(tty, -1);
        }

        (*tty).mouse_drag_flag = 0;
        (*tty).mouse_drag_update = None;
        (*tty).mouse_drag_release = None;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_send_requests(tty: *mut tty) {
    unsafe {
        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }

        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            // TODO I think the original C code has a bug and it should be as follows, double check
            if !(*tty).flags.intersects(tty_flags::TTY_HAVEDA) {
                tty_puts(tty, c"\x1b[c".as_ptr());
            }
            if !(*tty).flags.intersects(tty_flags::TTY_HAVEDA2) {
                tty_puts(tty, c"\x1b[>c".as_ptr());
            }
            if !(*tty).flags.intersects(tty_flags::TTY_HAVEXDA) {
                tty_puts(tty, c"\x1b[>q".as_ptr());
            }
            tty_puts(tty, c"\x1b]10;?\x1b\\".as_ptr());
            tty_puts(tty, c"\x1b]11;?\x1b\\".as_ptr());
        } else {
            (*tty).flags |= TTY_ALL_REQUEST_FLAGS;
        }
        (*tty).last_requests = libc::time(null_mut());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_repeat_requests(tty: *mut tty) {
    unsafe {
        let mut t = libc::time(null_mut());

        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }

        if (t - (*tty).last_requests <= TTY_REQUEST_LIMIT as i64) {
            return;
        }
        (*tty).last_requests = t;

        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            tty_puts(tty, c"\x1b]10;?\x1b\\".as_ptr());
            tty_puts(tty, c"\x1b]11;?\x1b\\".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_stop_tty(tty: *mut tty) {
    unsafe {
        let mut c = (*tty).client;
        let mut ws: libc::winsize = zeroed();

        if !(*tty).flags.intersects(tty_flags::TTY_STARTED) {
            return;
        }
        (*tty).flags &= !tty_flags::TTY_STARTED;

        evtimer_del(&raw mut (*tty).start_timer);

        event_del(&raw mut (*tty).timer);
        (*tty).flags &= !tty_flags::TTY_BLOCK;

        event_del(&raw mut (*tty).event_in);
        event_del(&raw mut (*tty).event_out);

        /*
         * Be flexible about error handling and try not kill the server just
         * because the fd is invalid. Things like ssh -t can easily leave us
         * with a dead tty.
         */
        if (libc::ioctl((*c).fd, libc::TIOCGWINSZ, &ws) == -1) {
            return;
        }
        if (libc::tcsetattr((*c).fd, libc::TCSANOW, &(*tty).tio) == -1) {
            return;
        }

        tty_raw(
            tty,
            tty_term_string_ii(
                (*tty).term,
                tty_code_code::TTYC_CSR,
                0,
                ws.ws_row as i32 - 1,
            ),
        );
        if (tty_acs_needed(tty) != 0) {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_RMACS));
        }
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_SGR0));
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_RMKX));
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_CLEAR));
        if ((*tty).cstyle != screen_cursor_style::SCREEN_CURSOR_DEFAULT) {
            if tty_term_has((*tty).term, tty_code_code::TTYC_SE).as_bool() {
                tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_SE));
            } else if tty_term_has((*tty).term, tty_code_code::TTYC_SS).as_bool() {
                tty_raw(
                    tty,
                    tty_term_string_i((*tty).term, tty_code_code::TTYC_SS, 0),
                );
            }
        }
        if ((*tty).ccolour != -1) {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_CR));
        }

        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_CNORM));
        if (tty_term_has((*tty).term, tty_code_code::TTYC_KMOUS).as_bool()) {
            tty_raw(tty, c"\x1b[?1000l\x1b[?1002l\x1b[?1003l".as_ptr());
            tty_raw(tty, c"\x1b[?1006l\x1b[?1005l".as_ptr());
        }
        if tty_term_has((*tty).term, tty_code_code::TTYC_DSBP).as_bool() {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSBP));
        }

        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            tty_raw(tty, c"\x1b[?7727l".as_ptr());
        }
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSFCS));
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSEKS));

        if (tty_use_margin(tty)) {
            tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_DSMG));
        }
        tty_raw(tty, tty_term_string((*tty).term, tty_code_code::TTYC_RMCUP));

        setblocking((*c).fd, 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_close(tty: *mut tty) {
    unsafe {
        if event_initialized(&raw mut (*tty).key_timer).as_bool() {
            evtimer_del(&raw mut (*tty).key_timer);
        }
        tty_stop_tty(tty);

        if (*tty).flags.intersects(tty_flags::TTY_OPENED) {
            evbuffer_free((*tty).in_);
            event_del(&raw mut (*tty).event_in);
            evbuffer_free((*tty).out);
            event_del(&raw mut (*tty).event_out);

            tty_term_free((*tty).term);
            tty_keys_free(tty);

            (*tty).flags &= !tty_flags::TTY_OPENED;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_free(tty: *mut tty) {
    unsafe {
        tty_close(tty);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_update_features(tty: *mut tty) {
    unsafe {
        let mut c = (*tty).client;

        if tty_apply_features((*tty).term, (*c).term_features).as_bool() {
            tty_term_apply_overrides((*tty).term);
        }

        if (tty_use_margin(tty)) {
            tty_putcode(tty, tty_code_code::TTYC_ENMG);
        }
        if options_get_number(global_options, c"extended-keys".as_ptr()) != 0 {
            tty_puts(tty, tty_term_string((*tty).term, tty_code_code::TTYC_ENEKS));
        }
        if options_get_number(global_options, c"focus-events".as_ptr()) != 0 {
            tty_puts(tty, tty_term_string((*tty).term, tty_code_code::TTYC_ENFCS));
        }
        if (*(*tty).term).flags.intersects(term_flags::TERM_VT100LIKE) {
            tty_puts(tty, c"\x1b[?7727h".as_ptr());
        }

        /*
         * Features might have changed since the first draw during attach. For
         * example, this happens when DA responses are received.
         */
        server_redraw_client(c);

        tty_invalidate(tty);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_raw(tty: *mut tty, mut s: *const c_char) {
    unsafe {
        let mut c = (*tty).client;

        let mut slen = strlen(s);
        for i in 0..5 {
            let n = libc::write((*c).fd, s.cast(), slen);
            if (n >= 0) {
                s = s.add(n as usize);
                slen -= n as usize;
                if (slen == 0) {
                    break;
                }
            } else if (n == -1 && errno!() != libc::EAGAIN) {
                break;
            }
            libc::usleep(100);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putcode(tty: *mut tty, code: tty_code_code) {
    unsafe {
        tty_puts(tty, tty_term_string((*tty).term, code));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putcode_i(tty: *mut tty, code: tty_code_code, a: i32) {
    unsafe {
        if (a < 0) {
            return;
        }
        tty_puts(tty, tty_term_string_i((*tty).term, code, a));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putcode_ii(tty: *mut tty, code: tty_code_code, a: i32, b: i32) {
    unsafe {
        if (a < 0 || b < 0) {
            return;
        }
        tty_puts(tty, tty_term_string_ii((*tty).term, code, a, b));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putcode_iii(
    tty: *mut tty,
    code: tty_code_code,
    a: i32,
    b: i32,
    c: i32,
) {
    unsafe {
        if (a < 0 || b < 0 || c < 0) {
            return;
        }
        tty_puts(tty, tty_term_string_iii((*tty).term, code, a, b, c));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putcode_s(tty: *mut tty, code: tty_code_code, a: *const c_char) {
    unsafe {
        if !a.is_null() {
            tty_puts(tty, tty_term_string_s((*tty).term, code, a));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putcode_ss(
    tty: *mut tty,
    code: tty_code_code,
    a: *const c_char,
    b: *const c_char,
) {
    unsafe {
        if !a.is_null() && !b.is_null() {
            tty_puts(tty, tty_term_string_ss((*tty).term, code, a, b));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_add(tty: *mut tty, buf: *const c_char, len: usize) {
    unsafe {
        let c = (*tty).client;

        if (*tty).flags.intersects(tty_flags::TTY_BLOCK) {
            (*tty).discarded += len;
            return;
        }

        evbuffer_add((*tty).out, buf.cast(), len);
        // log_debug("%s: %.*s", (*c).name, (int)len, buf);
        (*c).written += len;

        if (tty_log_fd != -1) {
            libc::write(tty_log_fd, buf.cast(), len);
        }
        if (*tty).flags.intersects(tty_flags::TTY_STARTED) {
            event_add(&raw mut (*tty).event_out, null_mut());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_puts(tty: *mut tty, s: *const c_char) {
    unsafe {
        if (*s != b'\0' as i8) {
            tty_add(tty, s, strlen(s));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putc(tty: *mut tty, ch: u8) {
    unsafe {
        if (*(*tty).term).flags.intersects(term_flags::TERM_NOAM)
            && ch >= 0x20
            && ch != 0x7f
            && (*tty).cy == (*tty).sy - 1
            && (*tty).cx + 1 >= (*tty).sx
        {
            return;
        }

        if (*tty).cell.attr & GRID_ATTR_CHARSET != 0 {
            let acs = tty_acs_get(tty, ch);
            if !acs.is_null() {
                tty_add(tty, acs, strlen(acs));
            } else {
                tty_add(tty, (&raw const ch).cast(), 1);
            }
        } else {
            tty_add(tty, (&raw const ch).cast(), 1);
        }

        if (ch >= 0x20 && ch != 0x7f) {
            if ((*tty).cx >= (*tty).sx) {
                (*tty).cx = 1;
                if ((*tty).cy != (*tty).rlower) {
                    (*tty).cy += 1;
                }

                /*
                 * On !am terminals, force the cursor position to where
                 * we think it should be after a line wrap - this means
                 * it works on sensible terminals as well.
                 */
                if (*(*tty).term).flags.intersects(term_flags::TERM_NOAM) {
                    tty_putcode_ii(
                        tty,
                        tty_code_code::TTYC_CUP,
                        (*tty).cy as i32,
                        (*tty).cx as i32,
                    );
                }
            } else {
                (*tty).cx += 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_putn(tty: *mut tty, buf: *const c_void, mut len: usize, width: u32) {
    unsafe {
        if (*(*tty).term).flags.intersects(term_flags::TERM_NOAM)
            && (*tty).cy == (*tty).sy - 1
            && (*tty).cx as usize + len >= (*tty).sx as usize
        {
            len = ((*tty).sx - (*tty).cx - 1) as usize;
        }

        tty_add(tty, buf.cast(), len);
        if ((*tty).cx + width > (*tty).sx) {
            (*tty).cx = ((*tty).cx + width) - (*tty).sx;
            if ((*tty).cx <= (*tty).sx) {
                (*tty).cy += 1;
            } else {
                (*tty).cx = u32::MAX;
                (*tty).cy = u32::MAX;
            }
        } else {
            (*tty).cx += width;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_set_italics(tty: *mut tty) {
    unsafe {
        if tty_term_has((*tty).term, tty_code_code::TTYC_SITM).as_bool() {
            let s = options_get_string(global_options, c"default-terminal".as_ptr());
            if (libc::strcmp(s, c"screen".as_ptr()) != 0
                && libc::strncmp(s, c"screen-".as_ptr(), 7) != 0)
            {
                tty_putcode(tty, tty_code_code::TTYC_SITM);
                return;
            }
        }
        tty_putcode(tty, tty_code_code::TTYC_SMSO);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_set_title(tty: *mut tty, title: *const c_char) {
    unsafe {
        if !tty_term_has((*tty).term, tty_code_code::TTYC_TSL)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_FSL)
        {
            return;
        }

        tty_putcode(tty, tty_code_code::TTYC_TSL);
        tty_puts(tty, title);
        tty_putcode(tty, tty_code_code::TTYC_FSL);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_set_path(tty: *mut tty, title: *const c_char) {
    unsafe {
        if (!tty_term_has((*tty).term, tty_code_code::TTYC_SWD)
            || !tty_term_has((*tty).term, tty_code_code::TTYC_FSL))
        {
            return;
        }

        tty_putcode(tty, tty_code_code::TTYC_SWD);
        tty_puts(tty, title);
        tty_putcode(tty, tty_code_code::TTYC_FSL);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tty_force_cursor_colour(tty: *mut tty, mut c: i32) {
    unsafe {
        let mut s: [c_char; 13] = [0; 13];

        if (c != -1) {
            c = colour_force_rgb(c);
        }
        if (c == (*tty).ccolour) {
            return;
        }
        if (c == -1) {
            tty_putcode(tty, tty_code_code::TTYC_CR);
        } else {
            let (r, g, b) = colour_split_rgb_(c);
            xsnprintf(
                (&raw mut s).cast(),
                13,
                c"rgb:%02hhx/%02hhx/%02hhx".as_ptr(),
                r as u32,
                g as u32,
                b as u32,
            );
            tty_putcode_s(tty, tty_code_code::TTYC_CS, (&raw const s).cast());
        }
        (*tty).ccolour = c;
    }
}

/*
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_update_cursor(tty: *mut tty , mode: i32, s: *mut screen ) -> i32 {
unsafe {
  enum screen_cursor_style cstyle;
  int ccolour, changed, cmode = mode;

  /* Set cursor colour if changed. */
  if (s != NULL) {
    ccolour = (*s).ccolour;
    if ((*s).ccolour == -1) {
      ccolour = (*s).default_ccolour;
    }
    tty_force_cursor_colour(tty, ccolour);
  }

  /* If cursor is off, set as invisible. */
  if (~cmode & MODE_CURSOR) {
    if ((*tty).mode & MODE_CURSOR) {
      tty_putcode(tty, TTYC_CIVIS);
    }
    return cmode;
  }

  /* Check if blinking or very visible flag changed or style changed. */
  if (s == NULL) {
    cstyle = (*tty).cstyle;
  } else {
    cstyle = (*s).cstyle;
    if (cstyle == SCREEN_CURSOR_DEFAULT) {
      if (~cmode & MODE_CURSOR_BLINKING_SET) {
        if ((*s).default_mode & MODE_CURSOR_BLINKING) {
          cmode |= MODE_CURSOR_BLINKING;
        } else {
          cmode &= ~MODE_CURSOR_BLINKING;
        }
      }
      cstyle = (*s).default_cstyle;
    }
  }

  /* If nothing changed, do nothing. */
  changed = cmode ^ (*tty).mode;
  if ((changed & CURSOR_MODES) == 0 && cstyle == (*tty).cstyle) {
    return cmode;
  }

  /*
   * Set cursor style. If an explicit style has been set with DECSCUSR,
   * set it if supported, otherwise send cvvis for blinking styles.
   *
   * If no style, has been set (SCREEN_CURSOR_DEFAULT), then send cvvis
   * if either the blinking or very visible flags are set.
   */
  tty_putcode(tty, TTYC_CNORM);
  switch (cstyle) {
  case SCREEN_CURSOR_DEFAULT:
    if ((*tty).cstyle != SCREEN_CURSOR_DEFAULT) {
      if (tty_term_has((*tty).term, TTYC_SE)) {
        tty_putcode(tty, TTYC_SE);
      } else {
        tty_putcode_i(tty, TTYC_SS, 0);
      }
    }
    if (cmode & (MODE_CURSOR_BLINKING | MODE_CURSOR_VERY_VISIBLE)) {
      tty_putcode(tty, TTYC_CVVIS);
    }
    break;
  case SCREEN_CURSOR_BLOCK:
    if (tty_term_has((*tty).term, TTYC_SS)) {
      if (cmode & MODE_CURSOR_BLINKING) {
        tty_putcode_i(tty, TTYC_SS, 1);
      } else {
        tty_putcode_i(tty, TTYC_SS, 2);
      }
    } else if (cmode & MODE_CURSOR_BLINKING) {
      tty_putcode(tty, TTYC_CVVIS);
    }
    break;
  case SCREEN_CURSOR_UNDERLINE:
    if (tty_term_has((*tty).term, TTYC_SS)) {
      if (cmode & MODE_CURSOR_BLINKING) {
        tty_putcode_i(tty, TTYC_SS, 3);
      } else {
        tty_putcode_i(tty, TTYC_SS, 4);
      }
    } else if (cmode & MODE_CURSOR_BLINKING) {
      tty_putcode(tty, TTYC_CVVIS);
    }
    break;
  case SCREEN_CURSOR_BAR:
    if (tty_term_has((*tty).term, TTYC_SS)) {
      if (cmode & MODE_CURSOR_BLINKING) {
        tty_putcode_i(tty, TTYC_SS, 5);
      } else {
        tty_putcode_i(tty, TTYC_SS, 6);
      }
    } else if (cmode & MODE_CURSOR_BLINKING) {
      tty_putcode(tty, TTYC_CVVIS);
    }
    break;
  }
  (*tty).cstyle = cstyle;
  return cmode;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_update_mode(tty: *mut tty , mode: i32, s: *mut screen ) {
unsafe {
  struct tty_term *term = (*tty).term;
  struct client *c = (*tty).client;
  int changed;

  if ((*tty).flags & TTY_NOCURSOR) {
    mode &= ~MODE_CURSOR;
  }

  if (tty_update_cursor(tty, mode, s) & MODE_CURSOR_BLINKING) {
    mode |= MODE_CURSOR_BLINKING;
  } else {
    mode &= ~MODE_CURSOR_BLINKING;
  }

  changed = mode ^ (*tty).mode;
  if (log_get_level() != 0 && changed != 0) {
    log_debug("%s: current mode %s", (*c).name,
              screen_mode_to_string((*tty).mode));
    log_debug("%s: setting mode %s", (*c).name, screen_mode_to_string(mode));
  }

  if ((changed & ALL_MOUSE_MODES) && tty_term_has(term, TTYC_KMOUS)) {
    /*
     * If the mouse modes have changed, clear then all and apply
     * again. There are differences in how terminals track the
     * various bits.
     */
    tty_puts(tty, "\033[?1006l\033[?1000l\033[?1002l\033[?1003l");
    if (mode & ALL_MOUSE_MODES) {
      tty_puts(tty, "\033[?1006h");
    }
    if (mode & MODE_MOUSE_ALL) {
      tty_puts(tty, "\033[?1000h\033[?1002h\033[?1003h");
    } else if (mode & MODE_MOUSE_BUTTON) {
      tty_puts(tty, "\033[?1000h\033[?1002h");
    } else if (mode & MODE_MOUSE_STANDARD) {
      tty_puts(tty, "\033[?1000h");
    }
  }
  (*tty).mode = mode;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_emulate_repeat(tty: *mut tty , code: tty_code_code , code1: tty_code_code , n: u32) {
unsafe {
  if (tty_term_has((*tty).term, code)) {
    tty_putcode_i(tty, code, n);
  } else {
    while (n-- > 0) {
      tty_putcode(tty, code1);
    }
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_repeat_space(tty: *mut tty , n: u32) {
unsafe {
  static char s[500];

  if (*s != ' ') {
    memset(s, ' ', sizeof s);
  }

  while (n > sizeof s) {
    tty_putn(tty, s, sizeof s, sizeof s);
    n -= sizeof s;
  }
  if (n != 0) {
    tty_putn(tty, s, n, n);
  }
}
}

/* Is this window bigger than the terminal? */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_window_bigger(tty: *mut tty ) -> i32 {
unsafe {
  struct client *c = (*tty).client;
  struct window *w = (*(*(*c).session).curw).window;

  return (*tty).sx < (*w).sx || (*tty).sy - status_line_size(c) < (*w).sy;
}
}

/// What offset should this window be drawn at?
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_window_offset(tty: *mut tty , ox: *mut u32 , oy: *mut u32, sx: *mut u32, sy: *mut u32) -> i32 {
unsafe{
  *ox = (*tty).oox;
  *oy = (*tty).ooy;
  *sx = (*tty).osx;
  *sy = (*tty).osy;

  (*tty).oflag
}
}

/// What offset should this window be drawn at?
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_window_offset1(tty: *mut tty , ox: *mut u32, oy: *mut u32, sx: *mut u32, sy: *mut u32) -> i32 {
unsafe {
  let mut c = (*tty).client;
  let mut w = (*(*(*c).session).curw).window;
  let mut wp = server_client_get_pane(c);
  // u_int cx, cy, lines;

  let lines: u32 = status_line_size(c);

  if ((*tty).sx >= (*w).sx && (*tty).sy - lines >= (*w).sy) {
    *ox = 0;
    *oy = 0;
    *sx = (*w).sx;
    *sy = (*w).sy;

    (*c).pan_window = NULL;
    return 0;
  }

  *sx = (*tty).sx;
  *sy = (*tty).sy - lines;

  if ((*c).pan_window == w) {
    if (*sx >= (*w).sx) {
      (*c).pan_ox = 0;
    } else if ((*c).pan_ox + *sx > (*w).sx) {
      (*c).pan_ox = (*w).sx - *sx;
    }
    *ox = (*c).pan_ox;
    if (*sy >= (*w).sy) {
      (*c).pan_oy = 0;
    } else if ((*c).pan_oy + *sy > (*w).sy) {
      (*c).pan_oy = (*w).sy - *sy;
    }
    *oy = (*c).pan_oy;
    return 1;
  }

  if (~(*(*wp).screen).mode & MODE_CURSOR) {
    *ox = 0;
    *oy = 0;
  } else {
    cx = (*wp).xoff + (*(*wp).screen).cx;
    cy = (*wp).yoff + (*(*wp).screen).cy;

    if (cx < *sx) {
      *ox = 0;
    } else if (cx > (*w).sx - *sx) {
      *ox = (*w).sx - *sx;
    } else {
      *ox = cx - *sx / 2;
    }

    if (cy < *sy) {
      *oy = 0;
    } else if (cy > (*w).sy - *sy) {
      *oy = (*w).sy - *sy;
    } else {
      *oy = cy - *sy / 2;
    }
  }

  (*c).pan_window = NULL;
  1
}
}

/// Update stored offsets for a window and redraw if necessary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_update_window_offset(w: *mut window ) {
unsafe {
  struct client *c;

  TAILQ_FOREACH(c, &clients, entry) {
    if ((*c).session != NULL && (*(*c).session).curw != NULL &&
        (*(*(*c).session).curw).window == w) {
      tty_update_client_offset(c);
    }
  }
}
}

/// Update stored offsets for a client and redraw if necessary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_update_client_offset(c: *mut client) {
unsafe {
  u_int ox, oy, sx, sy;

  if (~(*c).flags & CLIENT_TERMINAL) {
    return;
  }

  (*c).tty.oflag = tty_window_offset1(&(*c).tty, &ox, &oy, &sx, &sy);
  if (ox == (*c).tty.oox && oy == (*c).tty.ooy && sx == (*c).tty.osx &&
      sy == (*c).tty.osy) {
    return;
  }

  log_debug("%s: %s offset has changed (%u,%u %ux%u -> %u,%u %ux%u)", __func__,
            (*c).name, (*c).tty.oox, (*c).tty.ooy, (*c).tty.osx, (*c).tty.osy,
            ox, oy, sx, sy);

  (*c).tty.oox = ox;
  (*c).tty.ooy = oy;
  (*c).tty.osx = sx;
  (*c).tty.osy = sy;

  (*c).flags |= (CLIENT_REDRAWWINDOW | CLIENT_REDRAWSTATUS);
}
}

/*
 * Is the region large enough to be worth redrawing once later rather than
 * probably several times now? Currently yes if it is more than 50% of the
 * pane.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_large_region(_tty: *mut tty, ctx: *const tty_ctx) -> i32 {
unsafe {
  (*ctx).orlower - (*ctx).orupper >= (*ctx).sy / 2
}
}

/// Return if BCE is needed but the terminal doesn't have it - it'll need to be emulated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_fake_bce(tty: *const tty , gc: *const grid_cell, bg: u32) -> i32 {
unsafe {
  if (tty_term_flag((*tty).term, TTYC_BCE)) {
    return 0;
  }
  if (!COLOUR_DEFAULT(bg) || !COLOUR_DEFAULT((*gc).bg)) {
    return 1;
  }
  0
}
}

/*
 * Redraw scroll region using data from screen (already updated). Used when
 * CSR not supported, or window is a pane that doesn't take up the full
 * width of the terminal.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_redraw_region(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  let c = (*tty).client;
  u_int i;

  /*
   * If region is large, schedule a redraw. In most cases this is likely
   * to be followed by some more scrolling.
   */
  if (tty_large_region(tty, ctx)) {
    log_debug("%s: %s large redraw", __func__, (*c).name);
    (*ctx).redraw_cb(ctx);
    return;
  }

  for (i = (*ctx).orupper; i <= (*ctx).orlower; i++) {
    tty_draw_pane(tty, ctx, i);
  }
}
}

/// Is this position visible in the pane?
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_is_visible(_tty: *mut tty , ctx: *const tty_ctx, px: u32, py: u32, nx: u32, ny: u32) -> i32 {
unsafe {
  let xoff = (*ctx).rxoff + px, yoff = (*ctx).ryoff + py;

  if (!(*ctx).bigger) {
    return 1;
  }

  if (xoff + nx <= (*ctx).wox || xoff >= (*ctx).wox + (*ctx).wsx ||
      yoff + ny <= (*ctx).woy || yoff >= (*ctx).woy + (*ctx).wsy) {
    return 0;
  }

  1
}
}

/// Clamp line position to visible part of pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clamp_line(tty: *mut tty , ctx: *const tty_ctx, px: u32,
                          py: u32, nx: u32, i: *mut u32, x: *mut u32 , rx: *mut u32,
                          ry: *mut u32) -> i32 {
unsafe {
  let mut  xoff = (*ctx).rxoff + px;

  if (!tty_is_visible(tty, ctx, px, py, nx, 1)) {
    return 0;
  }
  *ry = (*ctx).yoff + py - (*ctx).woy;

  if (xoff >= (*ctx).wox && xoff + nx <= (*ctx).wox + (*ctx).wsx) {
    /* All visible. */
    *i = 0;
    *x = (*ctx).xoff + px - (*ctx).wox;
    *rx = nx;
  } else if (xoff < (*ctx).wox && xoff + nx > (*ctx).wox + (*ctx).wsx) {
    /* Both left and right not visible. */
    *i = (*ctx).wox;
    *x = 0;
    *rx = (*ctx).wsx;
  } else if (xoff < (*ctx).wox) {
    /* Left not visible. */
    *i = (*ctx).wox - ((*ctx).xoff + px);
    *x = 0;
    *rx = nx - *i;
  } else {
    /* Right not visible. */
    *i = 0;
    *x = ((*ctx).xoff + px) - (*ctx).wox;
    *rx = (*ctx).wsx - *x;
  }
  if (*rx > nx) {
    fatalx("%s: x too big, %u > %u", __func__, *rx, nx);
  }

  1
}
}

/// Clear a line.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clear_line(tty:*mut tty , defaults: *const grid_cell, py: u32, px: u32, nx: u32, bg: u32) {
unsafe {
  let c = (*tty).client;
  struct overlay_ranges r;
  u_int i;

  log_debug("%s: %s, %u at %u,%u", __func__, (*c).name, nx, px, py);

  /* Nothing to clear. */
  if (nx == 0) {
    return;
  }

  /* If genuine BCE is available, can try escape sequences. */
  if ((*c).overlay_check == NULL && !tty_fake_bce(tty, defaults, bg)) {
    /* Off the end of the line, use EL if available. */
    if (px + nx >= (*tty).sx && tty_term_has((*tty).term, TTYC_EL)) {
      tty_cursor(tty, px, py);
      tty_putcode(tty, TTYC_EL);
      return;
    }

    /* At the start of the line. Use EL1. */
    if (px == 0 && tty_term_has((*tty).term, TTYC_EL1)) {
      tty_cursor(tty, px + nx - 1, py);
      tty_putcode(tty, TTYC_EL1);
      return;
    }

    /* Section of line. Use ECH if possible. */
    if (tty_term_has((*tty).term, TTYC_ECH)) {
      tty_cursor(tty, px, py);
      tty_putcode_i(tty, TTYC_ECH, nx);
      return;
    }
  }

  /*
   * Couldn't use an escape sequence, use spaces. Clear only the visible
   * bit if there is an overlay.
   */
  tty_check_overlay_range(tty, px, py, nx, &r);
  for (i = 0; i < OVERLAY_MAX_RANGES; i++) {
    if (r.nx[i] == 0) {
      continue;
    }
    tty_cursor(tty, r.px[i], py);
    tty_repeat_space(tty, r.nx[i]);
  }
}
}

/// Clear a line, adjusting to visible part of pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clear_pane_line(tty: *mut tty , ctx: *const tty_ctx , py: u32, px: u32, nx: u32, bg: u32) {
unsafe {
  let c = (*tty).client;
  u_int i, x, rx, ry;

  log_debug("%s: %s, %u at %u,%u", __func__, (*c).name, nx, px, py);

  if (tty_clamp_line(tty, ctx, px, py, nx, &i, &x, &rx, &ry)) {
    tty_clear_line(tty, &(*ctx).defaults, ry, x, rx, bg);
  }
}
}

/// Clamp area position to visible part of pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clamp_area(tty: *mut tty , ctx: *const tty_ctx, px: u32,
                          py: u32, nx: u32, ny: u32, i: *mut u32, j: *mut u32,
                          x: *mut u32, y: *mut u32, rx: *mut u32, ry: *mut u32) -> i32 {
unsafe {
  let xoff = (*ctx).rxoff + px, yoff = (*ctx).ryoff + py;

  if (!tty_is_visible(tty, ctx, px, py, nx, ny)) {
    return 0;
  }

  if (xoff >= (*ctx).wox && xoff + nx <= (*ctx).wox + (*ctx).wsx) {
    /* All visible. */
    *i = 0;
    *x = (*ctx).xoff + px - (*ctx).wox;
    *rx = nx;
  } else if (xoff < (*ctx).wox && xoff + nx > (*ctx).wox + (*ctx).wsx) {
    /* Both left and right not visible. */
    *i = (*ctx).wox;
    *x = 0;
    *rx = (*ctx).wsx;
  } else if (xoff < (*ctx).wox) {
    /* Left not visible. */
    *i = (*ctx).wox - ((*ctx).xoff + px);
    *x = 0;
    *rx = nx - *i;
  } else {
    /* Right not visible. */
    *i = 0;
    *x = ((*ctx).xoff + px) - (*ctx).wox;
    *rx = (*ctx).wsx - *x;
  }
  if (*rx > nx) {
    fatalx("%s: x too big, %u > %u", __func__, *rx, nx);
  }

  if (yoff >= (*ctx).woy && yoff + ny <= (*ctx).woy + (*ctx).wsy) {
    /* All visible. */
    *j = 0;
    *y = (*ctx).yoff + py - (*ctx).woy;
    *ry = ny;
  } else if (yoff < (*ctx).woy && yoff + ny > (*ctx).woy + (*ctx).wsy) {
    /* Both top and bottom not visible. */
    *j = (*ctx).woy;
    *y = 0;
    *ry = (*ctx).wsy;
  } else if (yoff < (*ctx).woy) {
    /* Top not visible. */
    *j = (*ctx).woy - ((*ctx).yoff + py);
    *y = 0;
    *ry = ny - *j;
  } else {
    /* Bottom not visible. */
    *j = 0;
    *y = ((*ctx).yoff + py) - (*ctx).woy;
    *ry = (*ctx).wsy - *y;
  }
  if (*ry > ny) {
    fatalx("%s: y too big, %u > %u", __func__, *ry, ny);
  }

  1
}
}

/// Clear an area, adjusting to visible part of pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clear_area(tty: *mut tty , defaults: *const grid_cell, py: u32, ny: u32, px: u32, nx: u32, bg: u32) {
unsafe {
  let mut c = (*tty).client;
  u_int yy;
  char tmp[64];

  log_debug("%s: %s, %u,%u at %u,%u", __func__, (*c).name, nx, ny, px, py);

  /* Nothing to clear. */
  if (nx == 0 || ny == 0) {
    return;
  }

  /* If genuine BCE is available, can try escape sequences. */
  if ((*c).overlay_check == NULL && !tty_fake_bce(tty, defaults, bg)) {
    /* Use ED if clearing off the bottom of the terminal. */
    if (px == 0 && px + nx >= (*tty).sx && py + ny >= (*tty).sy &&
        tty_term_has((*tty).term, TTYC_ED)) {
      tty_cursor(tty, 0, py);
      tty_putcode(tty, TTYC_ED);
      return;
    }

    /*
     * On VT420 compatible terminals we can use DECFRA if the
     * background colour isn't default (because it doesn't work
     * after SGR 0).
     */
    if (((*(*tty).term).flags & TERM_DECFRA) && !COLOUR_DEFAULT(bg)) {
      xsnprintf(tmp, sizeof tmp, "\033[32;%u;%u;%u;%u$x", py + 1, px + 1,
                py + ny, px + nx);
      tty_puts(tty, tmp);
      return;
    }

    /* Full lines can be scrolled away to clear them. */
    if (px == 0 && px + nx >= (*tty).sx && ny > 2 &&
        tty_term_has((*tty).term, TTYC_CSR) &&
        tty_term_has((*tty).term, TTYC_INDN)) {
      tty_region(tty, py, py + ny - 1);
      tty_margin_off(tty);
      tty_putcode_i(tty, TTYC_INDN, ny);
      return;
    }

    /*
     * If margins are supported, can just scroll the area off to
     * clear it.
     */
    if (nx > 2 && ny > 2 && tty_term_has((*tty).term, TTYC_CSR) &&
        tty_use_margin(tty) && tty_term_has((*tty).term, TTYC_INDN)) {
      tty_region(tty, py, py + ny - 1);
      tty_margin(tty, px, px + nx - 1);
      tty_putcode_i(tty, TTYC_INDN, ny);
      return;
    }
  }

  /* Couldn't use an escape sequence, loop over the lines. */
  for (yy = py; yy < py + ny; yy++) {
    tty_clear_line(tty, defaults, yy, px, nx, bg);
  }
}
}

/// Clear an area in a pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clear_pane_area(tty: *mut tty , ctx: *const tty_ctx , py: u32, ny: u32, px: u32, nx: u32, bg: u32) {
unsafe {
let mut i: u32 = 0;
let mut j: u32 = 0;
let mut x: u32 = 0;
let mut y: u32 = 0;
let mut rx: u32 = 0;
let mut ry: u32 = 0;

  if (tty_clamp_area(tty, ctx, px, py, nx, ny, &raw mut i, &raw mut j, &raw mut x, &raw mut y, &raw mut rx, &raw mut ry)) {
    tty_clear_area(tty, &raw mut (*ctx).defaults, y, ry, x, rx, bg);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_draw_pane(tty: *mut tty , ctx: *const tty_ctx , py: u32) {
unsafe {
  let s = (*ctx).s;
  let nx = (*ctx).sx;
// i, x, rx, ry;

  log_debug("%s: %s %u %d", __func__, (*(*tty).client).name, py, (*ctx).bigger);

  if (!(*ctx).bigger) {
    tty_draw_line(tty, s, 0, py, nx, (*ctx).xoff, (*ctx).yoff + py,
                  &(*ctx).defaults, (*ctx).palette);
    return;
  }
  if (tty_clamp_line(tty, ctx, 0, py, nx, &i, &x, &rx, &ry)) {
    tty_draw_line(tty, s, i, py, rx, x, ry, &(*ctx).defaults, (*ctx).palette);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_check_codeset(tty: *mut tty, gc: *const grid_cell) -> *const grid_cell {
unsafe {
  static struct grid_cell new;
  int c;

  /* Characters less than 0x7f are always fine, no matter what. */
  if ((*gc).data.size == 1 && *(*gc).data.data < 0x7f) {
    return gc;
  }

  /* UTF-8 terminal and a UTF-8 character - fine. */
  if ((*(*tty).client).flags & CLIENT_UTF8) {
    return gc;
  }
  memcpy(&new, gc, sizeof new);

  /* See if this can be mapped to an ACS character. */
  c = tty_acs_reverse_get(tty, (*gc).data.data, (*gc).data.size);
  if (c != -1) {
    utf8_set(&new.data, c);
    new.attr |= GRID_ATTR_CHARSET;
    return &new;
  }

  /* Replace by the right number of underscores. */
  new.data.size = (*gc).data.width;
  if (new.data.size > UTF8_SIZE) {
    new.data.size = UTF8_SIZE;
  }
  memset(new.data.data, '_', new.data.size);
  return &new;
}
}

/// Check if a single character is obstructed by the overlay and return a boolean.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_check_overlay(tty: *mut tty , px: u32, py: u32) -> i32 {
unsafe {
  let mut overlay_ranges r = zeroed();

  /*
   * A unit width range will always return nx[2] == 0 from a check, even
   * with multiple overlays, so it's sufficient to check just the first
   * two entries.
   */
  tty_check_overlay_range(tty, px, py, 1, &raw mut r);
  if (r.nx[0] + r.nx[1] == 0) {
    return 0;
  }

  1
}
}

/// Return parts of the input range which are visible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_check_overlay_range(tty: *mut tty , px: u32, py: u32, nx: u32, r: *mut overlay_ranges) {
unsafe {
  let mut c = (*tty).client;

  if ((*c).overlay_check == NULL) {
    (*r).px[0] = px;
    (*r).nx[0] = nx;
    (*r).px[1] = 0;
    (*r).nx[1] = 0;
    (*r).px[2] = 0;
    (*r).nx[2] = 0;
    return;
  }

  (*c).overlay_check(c, (*c).overlay_data, px, py, nx, r);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_draw_line(tty: *mut tty , s: *mut screen , px: u32, py: u32,
                   nx: u32, atx: u32, aty: u32,
                   defaults: *const grid_cell,
                   palette: *mut colour_palette ) {
unsafe {
  let mut gd = (*s).grid;
  // struct grid_cell gc, last;
  // const struct grid_cell *gcp;
  // struct grid_line *gl;
  let mut c = (*tty).client;
  struct overlay_ranges r;
  u_int i, j, ux, sx, width, hidden, eux, nxx;
  u_int cellsize;
  int flags, cleared = 0, wrapped = 0;
  char buf[512];
  size_t len;

  log_debug("%s: px=%u py=%u nx=%u atx=%u aty=%u", __func__, px, py, nx, atx,
            aty);
  log_debug("%s: defaults: fg=%d, bg=%d", __func__, (*defaults).fg,
            (*defaults).bg);

  /*
   * py is the line in the screen to draw.
   * px is the start x and nx is the width to draw.
   * atx,aty is the line on the terminal to draw it.
   */

  flags = ((*tty).flags & TTY_NOCURSOR);
  (*tty).flags |= TTY_NOCURSOR;
  tty_update_mode(tty, (*tty).mode, s);

  tty_region_off(tty);
  tty_margin_off(tty);

  /*
   * Clamp the width to cellsize - note this is not cellused, because
   * there may be empty background cells after it (from BCE).
   */
  sx = screen_size_x(s);
  if (nx > sx) {
    nx = sx;
  }
  cellsize = (*grid_get_line(gd, (*gd).hsize + py)).cellsize;
  if (sx > cellsize) {
    sx = cellsize;
  }
  if (sx > (*tty).sx) {
    sx = (*tty).sx;
  }
  if (sx > nx) {
    sx = nx;
  }
  ux = 0;

  if (py == 0) {
    gl = NULL;
  } else {
    gl = grid_get_line(gd, (*gd).hsize + py - 1);
  }
  if (gl == NULL || (~(*gl).flags & GRID_LINE_WRAPPED) || atx != 0 ||
      (*tty).cx < (*tty).sx || nx < (*tty).sx) {
    if (nx < (*tty).sx && atx == 0 && px + sx != nx &&
        tty_term_has((*tty).term, TTYC_EL1) &&
        !tty_fake_bce(tty, defaults, 8) && (*c).overlay_check == NULL) {
      tty_default_attributes(tty, defaults, palette, 8, (*s).hyperlinks);
      tty_cursor(tty, nx - 1, aty);
      tty_putcode(tty, TTYC_EL1);
      cleared = 1;
    }
  } else {
    log_debug("%s: wrapped line %u", __func__, aty);
    wrapped = 1;
  }

  memcpy(&last, &grid_default_cell, sizeof last);
  len = 0;
  width = 0;

  for (i = 0; i < sx; i++) {
    grid_view_get_cell(gd, px + i, py, &gc);
    gcp = tty_check_codeset(tty, &gc);
    if (len != 0 &&
        (!tty_check_overlay(tty, atx + ux + width, aty) ||
         ((*gcp).attr & GRID_ATTR_CHARSET) || (*gcp).flags != last.flags ||
         (*gcp).attr != last.attr || (*gcp).fg != last.fg ||
         (*gcp).bg != last.bg || (*gcp).us != last.us ||
         (*gcp).link != last.link || ux + width + (*gcp).data.width > nx ||
         (sizeof buf) - len < (*gcp).data.size)) {
      tty_attributes(tty, &last, defaults, palette, (*s).hyperlinks);
      if (last.flags & GRID_FLAG_CLEARED) {
        log_debug("%s: %zu cleared", __func__, len);
        tty_clear_line(tty, defaults, aty, atx + ux, width, last.bg);
      } else {
        if (!wrapped || atx != 0 || ux != 0) {
          tty_cursor(tty, atx + ux, aty);
        }
        tty_putn(tty, buf, len, width);
      }
      ux += width;

      len = 0;
      width = 0;
      wrapped = 0;
    }

    if ((*gcp).flags & GRID_FLAG_SELECTED) {
      screen_select_cell(s, &last, gcp);
    } else {
      memcpy(&last, gcp, sizeof last);
    }

    tty_check_overlay_range(tty, atx + ux, aty, (*gcp).data.width, &r);
    hidden = 0;
    for (j = 0; j < OVERLAY_MAX_RANGES; j++) {
      hidden += r.nx[j];
    }
    hidden = (*gcp).data.width - hidden;
    if (hidden != 0 && hidden == (*gcp).data.width) {
      if (~(*gcp).flags & GRID_FLAG_PADDING) {
        ux += (*gcp).data.width;
      }
    } else if (hidden != 0 || ux + (*gcp).data.width > nx) {
      if (~(*gcp).flags & GRID_FLAG_PADDING) {
        tty_attributes(tty, &last, defaults, palette, (*s).hyperlinks);
        for (j = 0; j < OVERLAY_MAX_RANGES; j++) {
          if (r.nx[j] == 0) {
            continue;
          }
          /* Effective width drawn so far. */
          eux = r.px[j] - atx;
          if (eux < nx) {
            tty_cursor(tty, r.px[j], aty);
            nxx = nx - eux;
            if (r.nx[j] > nxx) {
              r.nx[j] = nxx;
            }
            tty_repeat_space(tty, r.nx[j]);
            ux = eux + r.nx[j];
          }
        }
      }
    } else if ((*gcp).attr & GRID_ATTR_CHARSET) {
      tty_attributes(tty, &last, defaults, palette, (*s).hyperlinks);
      tty_cursor(tty, atx + ux, aty);
      for (j = 0; j < (*gcp).data.size; j++) {
        tty_putc(tty, (*gcp).data.data[j]);
      }
      ux += (*gcp).data.width;
    } else if (~(*gcp).flags & GRID_FLAG_PADDING) {
      memcpy(buf + len, (*gcp).data.data, (*gcp).data.size);
      len += (*gcp).data.size;
      width += (*gcp).data.width;
    }
  }
  if (len != 0 && ((~last.flags & GRID_FLAG_CLEARED) || last.bg != 8)) {
    tty_attributes(tty, &last, defaults, palette, (*s).hyperlinks);
    if (last.flags & GRID_FLAG_CLEARED) {
      log_debug("%s: %zu cleared (end)", __func__, len);
      tty_clear_line(tty, defaults, aty, atx + ux, width, last.bg);
    } else {
      if (!wrapped || atx != 0 || ux != 0) {
        tty_cursor(tty, atx + ux, aty);
      }
      tty_putn(tty, buf, len, width);
    }
    ux += width;
  }

  if (!cleared && ux < nx) {
    log_debug("%s: %u to end of line (%zu cleared)", __func__, nx - ux, len);
    tty_default_attributes(tty, defaults, palette, 8, (*s).hyperlinks);
    tty_clear_line(tty, defaults, aty, atx + ux, nx - ux, 8);
  }

  (*tty).flags = ((*tty).flags & ~TTY_NOCURSOR) | flags;
  tty_update_mode(tty, (*tty).mode, s);
}
}

#ifdef ENABLE_SIXEL
/* Update context for client. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_set_client_cb(ttyctx: *mut tty_ctx , c: *mut client ) -> i32 {
unsafe {
  let mut wp = (*ttyctx).arg;

  if ((*(*(*c).session).curw).window != (*wp).window) {
    return 0;
  }
  if ((*wp).layout_cell == NULL) {
    return 0;
  }

  /* Set the properties relevant to the current client. */
  (*ttyctx).bigger =
      tty_window_offset(&(*c).tty, &(*ttyctx).wox, &(*ttyctx).woy,
                        &(*ttyctx).wsx, &(*ttyctx).wsy);

  (*ttyctx).yoff = (*ttyctx).ryoff = (*wp).yoff;
  if (status_at_line(c) == 0) {
    (*ttyctx).yoff += status_line_size(c);
  }

  1
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_draw_images(c: *mut client , wp: *mut window_pane , s: *mut screen ) {
unsafe {
  struct image *im;
  struct tty_ctx ttyctx;

  TAILQ_FOREACH(im, &(*s).images, entry) {
    memset(&ttyctx, 0, sizeof ttyctx);

    /* Set the client independent properties. */
    ttyctx.ocx = (*im).px;
    ttyctx.ocy = (*im).py;

    ttyctx.orlower = (*s).rlower;
    ttyctx.orupper = (*s).rupper;

    ttyctx.xoff = ttyctx.rxoff = (*wp).xoff;
    ttyctx.sx = (*wp).sx;
    ttyctx.sy = (*wp).sy;

    ttyctx.ptr = im;
    ttyctx.arg = wp;
    ttyctx.set_client_cb = tty_set_client_cb;
    ttyctx.allow_invisible_panes = 1;
    tty_write_one(tty_cmd_sixelimage, c, &ttyctx);
  }
}
}
#endif

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_sync_start(tty: *mut tty ) {
unsafe {
  if ((*tty).flags & TTY_BLOCK) {
    return;
  }
  if ((*tty).flags & TTY_SYNCING) {
    return;
  }
  (*tty).flags |= TTY_SYNCING;

  if (tty_term_has((*tty).term, TTYC_SYNC)) {
    log_debug("%s sync start", (*(*tty).client).name);
    tty_putcode_i(tty, TTYC_SYNC, 1);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_sync_end(tty: *mut tty ) {
unsafe {
  if ((*tty).flags & TTY_BLOCK) {
    return;
  }
  if (!(*tty).flags & TTY_SYNCING) {
    return;
  }
  (*tty).flags &= ~TTY_SYNCING;

  if (tty_term_has((*tty).term, TTYC_SYNC)) {
    log_debug("%s sync end", (*(*tty).client).name);
    tty_putcode_i(tty, TTYC_SYNC, 2);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_client_ready(ctx: *const tty_ctx , c: *mut client) -> i32 {
unsafe {
  if ((*c).session == NULL || (*c).tty.term == NULL) {
    return 0;
  }
  if ((*c).flags & CLIENT_SUSPENDED) {
    return 0;
  }

  /*
   * If invisible panes are allowed (used for passthrough), don't care if
   * redrawing or frozen.
   */
  if ((*ctx).allow_invisible_panes) {
    return 1;
  }

  if ((*c).flags & CLIENT_REDRAWWINDOW) {
    return 0;
  }
  if ((*c).tty.flags & TTY_FREEZE) {
    return 0;
  }
  1
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_write(void (*cmdfn)(struct tty *, const struct tty_ctx *), ctx: *mut tty_ctx ) {
unsafe {
  struct client *c;
  int state;

  if ((*ctx).set_client_cb == NULL) {
    return;
  }
  TAILQ_FOREACH(c, &clients, entry) {
    if (tty_client_ready(ctx, c)) {
      state = (*ctx).set_client_cb(ctx, c);
      if (state == -1) {
        break;
      }
      if (state == 0) {
        continue;
      }
      cmdfn(&(*c).tty, ctx);
    }
  }
}
}

#ifdef ENABLE_SIXEL
/* Only write to the incoming tty instead of every client. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_write_one(void (*cmdfn)(struct tty *, const struct tty_ctx *), c: *mut client , ctx: *mut tty_ctx ) {
  if ((*ctx).set_client_cb == NULL) {
    return;
  }
  if (((*ctx).set_client_cb(ctx, c)) == 1) {
    cmdfn(&(*c).tty, ctx);
  }
}
#endif

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_insertcharacter(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let c = (*tty).client;

  if ((*ctx).bigger || !tty_full_width(tty, ctx) ||
      tty_fake_bce(tty, &(*ctx).defaults, (*ctx).bg) ||
      (!tty_term_has((*tty).term, TTYC_ICH) &&
       !tty_term_has((*tty).term, TTYC_ICH1)) ||
      (*c).overlay_check != NULL) {
    tty_draw_pane(tty, ctx, (*ctx).ocy);
    return;
  }

  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

  tty_emulate_repeat(tty, TTYC_ICH, TTYC_ICH1, (*ctx).num);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_deletecharacter(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  let mut c = (*tty).client;

  if ((*ctx).bigger || !tty_full_width(tty, ctx) ||
      tty_fake_bce(tty, &(*ctx).defaults, (*ctx).bg) ||
      (!tty_term_has((*tty).term, TTYC_DCH) &&
       !tty_term_has((*tty).term, TTYC_DCH1)) ||
      (*c).overlay_check != NULL) {
    tty_draw_pane(tty, ctx, (*ctx).ocy);
    return;
  }

  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

  tty_emulate_repeat(tty, TTYC_DCH, TTYC_DCH1, (*ctx).num);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearcharacter(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_clear_pane_line(tty, ctx, (*ctx).ocy, (*ctx).ocx, (*ctx).num, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_insertline(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let c = (*tty).client;

  if ((*ctx).bigger || !tty_full_width(tty, ctx) ||
      tty_fake_bce(tty, &(*ctx).defaults, (*ctx).bg) ||
      !tty_term_has((*tty).term, TTYC_CSR) ||
      !tty_term_has((*tty).term, TTYC_IL1) || (*ctx).sx == 1 ||
      (*ctx).sy == 1 || (*c).overlay_check != NULL) {
    tty_redraw_region(tty, ctx);
    return;
  }

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  tty_margin_off(tty);
  tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

  tty_emulate_repeat(tty, TTYC_IL, TTYC_IL1, (*ctx).num);
  (*tty).cx = (*tty).cy = UINT_MAX;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_deleteline(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  let c = (*tty).client;

  if ((*ctx).bigger || !tty_full_width(tty, ctx) ||
      tty_fake_bce(tty, &(*ctx).defaults, (*ctx).bg) ||
      !tty_term_has((*tty).term, TTYC_CSR) ||
      !tty_term_has((*tty).term, TTYC_DL1) || (*ctx).sx == 1 ||
      (*ctx).sy == 1 || (*c).overlay_check != NULL) {
    tty_redraw_region(tty, ctx);
    return;
  }

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  tty_margin_off(tty);
  tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);

  tty_emulate_repeat(tty, TTYC_DL, TTYC_DL1, (*ctx).num);
  (*tty).cx = (*tty).cy = UINT_MAX;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearline(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_clear_pane_line(tty, ctx, (*ctx).ocy, 0, (*ctx).sx, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearendofline(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let nx = (*ctx).sx - (*ctx).ocx;

  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_clear_pane_line(tty, ctx, (*ctx).ocy, (*ctx).ocx, nx, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearstartofline(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_clear_pane_line(tty, ctx, (*ctx).ocy, 0, (*ctx).ocx + 1, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_reverseindex(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let c = (*tty).client;

  if ((*ctx).ocy != (*ctx).orupper) {
    return;
  }

  if ((*ctx).bigger || (!tty_full_width(tty, ctx) && !tty_use_margin(tty)) ||
      tty_fake_bce(tty, &(*ctx).defaults, 8) ||
      !tty_term_has((*tty).term, TTYC_CSR) ||
      (!tty_term_has((*tty).term, TTYC_RI) &&
       !tty_term_has((*tty).term, TTYC_RIN)) ||
      (*ctx).sx == 1 || (*ctx).sy == 1 || (*c).overlay_check != NULL) {
    tty_redraw_region(tty, ctx);
    return;
  }

  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  tty_margin_pane(tty, ctx);
  tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).orupper);

  if (tty_term_has((*tty).term, TTYC_RI)) {
    tty_putcode(tty, TTYC_RI);
  } else {
    tty_putcode_i(tty, TTYC_RIN, 1);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_linefeed(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let c = (*tty).client;

  if ((*ctx).ocy != (*ctx).orlower) {
    return;
  }

  if ((*ctx).bigger || (!tty_full_width(tty, ctx) && !tty_use_margin(tty)) ||
      tty_fake_bce(tty, &(*ctx).defaults, 8) ||
      !tty_term_has((*tty).term, TTYC_CSR) || (*ctx).sx == 1 ||
      (*ctx).sy == 1 || (*c).overlay_check != NULL) {
    tty_redraw_region(tty, ctx);
    return;
  }

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  tty_margin_pane(tty, ctx);

  /*
   * If we want to wrap a pane while using margins, the cursor needs to
   * be exactly on the right of the region. If the cursor is entirely off
   * the edge - move it back to the right. Some terminals are funny about
   * this and insert extra spaces, so only use the right if margins are
   * enabled.
   */
  if ((*ctx).xoff + (*ctx).ocx > (*tty).rright) {
    if (!tty_use_margin(tty)) {
      tty_cursor(tty, 0, (*ctx).yoff + (*ctx).ocy);
    } else {
      tty_cursor(tty, (*tty).rright, (*ctx).yoff + (*ctx).ocy);
    }
  } else {
    tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).ocy);
  }

  tty_putc(tty, '\n');
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_scrollup(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let mut c = (*tty).client;
  u_int i;

  if ((*ctx).bigger || (!tty_full_width(tty, ctx) && !tty_use_margin(tty)) ||
      tty_fake_bce(tty, &(*ctx).defaults, 8) ||
      !tty_term_has((*tty).term, TTYC_CSR) || (*ctx).sx == 1 ||
      (*ctx).sy == 1 || (*c).overlay_check != NULL) {
    tty_redraw_region(tty, ctx);
    return;
  }

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  tty_margin_pane(tty, ctx);

  if ((*ctx).num == 1 || !tty_term_has((*tty).term, TTYC_INDN)) {
    if (!tty_use_margin(tty)) {
      tty_cursor(tty, 0, (*tty).rlower);
    } else {
      tty_cursor(tty, (*tty).rright, (*tty).rlower);
    }
    for (i = 0; i < (*ctx).num; i++) {
      tty_putc(tty, '\n');
    }
  } else {
    if ((*tty).cy == UINT_MAX) {
      tty_cursor(tty, 0, 0);
    } else {
      tty_cursor(tty, 0, (*tty).cy);
    }
    tty_putcode_i(tty, TTYC_INDN, (*ctx).num);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_scrolldown(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  u_int i;
  let c = (*tty).client;

  if ((*ctx).bigger || (!tty_full_width(tty, ctx) && !tty_use_margin(tty)) ||
      tty_fake_bce(tty, &(*ctx).defaults, 8) ||
      !tty_term_has((*tty).term, TTYC_CSR) ||
      (!tty_term_has((*tty).term, TTYC_RI) &&
       !tty_term_has((*tty).term, TTYC_RIN)) ||
      (*ctx).sx == 1 || (*ctx).sy == 1 || (*c).overlay_check != NULL) {
    tty_redraw_region(tty, ctx);
    return;
  }

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  tty_margin_pane(tty, ctx);
  tty_cursor_pane(tty, ctx, (*ctx).ocx, (*ctx).orupper);

  if (tty_term_has((*tty).term, TTYC_RIN)) {
    tty_putcode_i(tty, TTYC_RIN, (*ctx).num);
  } else {
    for (i = 0; i < (*ctx).num; i++) {
      tty_putcode(tty, TTYC_RI);
    }
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearendofscreen(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  u_int px, py, nx, ny;

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
  tty_margin_off(tty);

  px = 0;
  nx = (*ctx).sx;
  py = (*ctx).ocy + 1;
  ny = (*ctx).sy - (*ctx).ocy - 1;

  tty_clear_pane_area(tty, ctx, py, ny, px, nx, (*ctx).bg);

  px = (*ctx).ocx;
  nx = (*ctx).sx - (*ctx).ocx;
  py = (*ctx).ocy;

  tty_clear_pane_line(tty, ctx, py, px, nx, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearstartofscreen(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  u_int px, py, nx, ny;

  tty_default_attributes(tty, &raw mut (*ctx).defaults, (*ctx).palette, (*ctx).bg, (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
  tty_margin_off(tty);

  px = 0;
  nx = (*ctx).sx;
  py = 0;
  ny = (*ctx).ocy;

  tty_clear_pane_area(tty, ctx, py, ny, px, nx, (*ctx).bg);

  px = 0;
  nx = (*ctx).ocx + 1;
  py = (*ctx).ocy;

  tty_clear_pane_line(tty, ctx, py, px, nx, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_clearscreen(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  u_int px, py, nx, ny;

  tty_default_attributes(tty, &(*ctx).defaults, (*ctx).palette, (*ctx).bg,
                         (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
  tty_margin_off(tty);

  px = 0;
  nx = (*ctx).sx;
  py = 0;
  ny = (*ctx).sy;

  tty_clear_pane_area(tty, ctx, py, ny, px, nx, (*ctx).bg);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_alignmenttest(tty: tty *, ctx: *const tty_ctx ) {
unsafe {
  u_int i, j;

  if ((*ctx).bigger) {
    (*ctx).redraw_cb(ctx);
    return;
  }

  tty_attributes(tty, &raw mut grid_default_cell, &(*ctx).defaults, (*ctx).palette, (*(*ctx).s).hyperlinks);

  tty_region_pane(tty, ctx, 0, (*ctx).sy - 1);
  tty_margin_off(tty);

  for (j = 0; j < (*ctx).sy; j++) {
    tty_cursor_pane(tty, ctx, 0, j);
    for (i = 0; i < (*ctx).sx; i++) {
      tty_putc(tty, 'E');
    }
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_cell(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  const struct grid_cell *gcp = (*ctx).cell;
  struct screen *s = (*ctx).s;
  struct overlay_ranges r;
  u_int px, py, i, vis = 0;

  px = (*ctx).xoff + (*ctx).ocx - (*ctx).wox;
  py = (*ctx).yoff + (*ctx).ocy - (*ctx).woy;
  if (!tty_is_visible(tty, ctx, (*ctx).ocx, (*ctx).ocy, 1, 1) ||
      ((*gcp).data.width == 1 && !tty_check_overlay(tty, px, py))) {
    return;
  }

  /* Handle partially obstructed wide characters. */
  if ((*gcp).data.width > 1) {
    tty_check_overlay_range(tty, px, py, (*gcp).data.width, &r);
    for (i = 0; i < OVERLAY_MAX_RANGES; i++) {
      vis += r.nx[i];
    }
    if (vis < (*gcp).data.width) {
      tty_draw_line(tty, s, (*s).cx, (*s).cy, (*gcp).data.width, px, py,
                    &(*ctx).defaults, (*ctx).palette);
      return;
    }
  }

  if ((*ctx).xoff + (*ctx).ocx - (*ctx).wox > (*tty).sx - 1 &&
      (*ctx).ocy == (*ctx).orlower && tty_full_width(tty, ctx)) {
    tty_region_pane(tty, ctx, (*ctx).orupper, (*ctx).orlower);
  }

  tty_margin_off(tty);
  tty_cursor_pane_unless_wrap(tty, ctx, (*ctx).ocx, (*ctx).ocy);

  tty_cell(tty, (*ctx).cell, &(*ctx).defaults, (*ctx).palette,
           (*(*ctx).s).hyperlinks);

  if ((*ctx).num == 1) {
    tty_invalidate(tty);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_cells(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  struct overlay_ranges r;
  u_int i, px, py, cx;
  char *cp = (*ctx).ptr;

  if (!tty_is_visible(tty, ctx, (*ctx).ocx, (*ctx).ocy, (*ctx).num, 1)) {
    return;
  }

  if ((*ctx).bigger &&
      ((*ctx).xoff + (*ctx).ocx < (*ctx).wox ||
       (*ctx).xoff + (*ctx).ocx + (*ctx).num > (*ctx).wox + (*ctx).wsx)) {
    if (!(*ctx).wrapped || !tty_full_width(tty, ctx) ||
        ((*(*tty).term).flags & TERM_NOAM) || (*ctx).xoff + (*ctx).ocx != 0 ||
        (*ctx).yoff + (*ctx).ocy != (*tty).cy + 1 || (*tty).cx < (*tty).sx ||
        (*tty).cy == (*tty).rlower) {
      tty_draw_pane(tty, ctx, (*ctx).ocy);
    } else {
      (*ctx).redraw_cb(ctx);
    }
    return;
  }

  tty_margin_off(tty);
  tty_cursor_pane_unless_wrap(tty, ctx, (*ctx).ocx, (*ctx).ocy);
  tty_attributes(tty, (*ctx).cell, &(*ctx).defaults, (*ctx).palette,
                 (*(*ctx).s).hyperlinks);

  /* Get tty position from pane position for overlay check. */
  px = (*ctx).xoff + (*ctx).ocx - (*ctx).wox;
  py = (*ctx).yoff + (*ctx).ocy - (*ctx).woy;

  tty_check_overlay_range(tty, px, py, (*ctx).num, &r);
  for (i = 0; i < OVERLAY_MAX_RANGES; i++) {
    if (r.nx[i] == 0) {
      continue;
    }
    /* Convert back to pane position for printing. */
    cx = r.px[i] - (*ctx).xoff + (*ctx).wox;
    tty_cursor_pane_unless_wrap(tty, ctx, cx, (*ctx).ocy);
    tty_putn(tty, cp + r.px[i] - px, r.nx[i], r.nx[i]);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_setselection(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  tty_set_selection(tty, (*ctx).ptr2, (*ctx).ptr, (*ctx).num);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_set_selection(tty: *mut tty , flags: *const c_char , buf: *const c_char , len: usize) {
unsafe {
  char *encoded;
  size_t size;

  if (!(*tty).flags & TTY_STARTED) {
    return;
  }
  if (!tty_term_has((*tty).term, TTYC_MS)) {
    return;
  }

  size = 4 * ((len + 2) / 3) + 1; /* storage for base64 */
  encoded = xmalloc(size);

  b64_ntop(buf, len, encoded, size);
  (*tty).flags |= TTY_NOBLOCK;
  tty_putcode_ss(tty, TTYC_MS, flags, encoded);

  free_(encoded);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_rawstring(tty: *mut tty, ctx: *const tty_ctx) {
unsafe {
  (*tty).flags |= TTY_NOBLOCK;
  tty_add(tty, (*ctx).ptr, (*ctx).num);
  tty_invalidate(tty);
}
}

#ifdef ENABLE_SIXEL
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_sixelimage(tty: *mut tty , ctx: *const tty_ctx ) {
unsafe {
  let mut im = (*ctx).ptr;
  let mut si : *sixel_image = (*im).data;
  struct sixel_image *new;
  char *data;
  size_t size;
  let cx = (*ctx).ocx;
  let cy = (*ctx).ocy;
// sx, sy;
  u_int i, j, x, y, rx, ry;
  int fallback = 0;

  if ((~(*(*tty).term).flags & TERM_SIXEL) &&
      !tty_term_has((*tty).term, TTYC_SXL)) {
    fallback = 1;
  }
  if ((*tty).xpixel == 0 || (*tty).ypixel == 0) {
    fallback = 1;
  }

  sixel_size_in_cells(si, &sx, &sy);
  log_debug("%s: image is %ux%u", __func__, sx, sy);
  if (!tty_clamp_area(tty, ctx, cx, cy, sx, sy, &i, &j, &x, &y, &rx, &ry)) {
    return;
  }
  log_debug("%s: clamping to %u,%u-%u,%u", __func__, i, j, rx, ry);

  if (fallback == 1) {
    data = xstrdup((*im).fallback);
    size = strlen(data);
  } else {
    new = sixel_scale(si, (*tty).xpixel, (*tty).ypixel, i, j, rx, ry, 0);
    if (new == NULL) {
      return;
    }

    data = sixel_print(new, si, &size);
  }
  if (data != NULL) {
    log_debug("%s: %zu bytes: %s", __func__, size, data);
    tty_region_off(tty);
    tty_margin_off(tty);
    tty_cursor(tty, x, y);

    (*tty).flags |= TTY_NOBLOCK;
    tty_add(tty, data, size);
    tty_invalidate(tty);
    free(data);
  }

  if (fallback == 0) {
    sixel_free(new);
  }
}
}
#endif

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cmd_syncstart(tty: *mut tty , ctx: *const tty_ctx) {
unsafe {
  if ((*ctx).num == 0x11) {
    /*
     * This is an overlay and a command that moves the cursor so
     * start synchronized updates.
     */
    tty_sync_start(tty);
  } else if (~(*ctx).num & 0x10) {
    /*
     * This is a pane. If there is an overlay, always start;
     * otherwise, only if requested.
     */
    if ((*ctx).num || (*(*tty).client).overlay_draw != NULL) {
      tty_sync_start(tty);
    }
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cell(tty: *mut tty , gc: *const grid_cell, defaults: *const grid_cell, palette: *mut colour_palette , hl: *mut hyperlinks) {
unsafe {
  const struct grid_cell *gcp;

  /* Skip last character if terminal is stupid. */
  if (((*(*tty).term).flags & TERM_NOAM) && (*tty).cy == (*tty).sy - 1 &&
      (*tty).cx == (*tty).sx - 1) {
    return;
  }

  /* If this is a padding character, do nothing. */
  if ((*gc).flags & GRID_FLAG_PADDING) {
    return;
  }

  /* Check the output codeset and apply attributes. */
  gcp = tty_check_codeset(tty, gc);
  tty_attributes(tty, gcp, defaults, palette, hl);

  /* If it is a single character, write with putc to handle ACS. */
  if ((*gcp).data.size == 1) {
    tty_attributes(tty, gcp, defaults, palette, hl);
    if (*(*gcp).data.data < 0x20 || *(*gcp).data.data == 0x7f) {
      return;
    }
    tty_putc(tty, *(*gcp).data.data);
    return;
  }

  /* Write the data. */
  tty_putn(tty, (*gcp).data.data, (*gcp).data.size, (*gcp).data.width);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_reset(tty: *mut tty ) {
unsafe {
  struct grid_cell *gc = &(*tty).cell;

  if (!grid_cells_equal(gc, &grid_default_cell)) {
    if ((*gc).link != 0) {
      tty_putcode_ss(tty, TTYC_HLS, "", "");
    }
    if (((*gc).attr & GRID_ATTR_CHARSET) && tty_acs_needed(tty)) {
      tty_putcode(tty, TTYC_RMACS);
    }
    tty_putcode(tty, TTYC_SGR0);
    memcpy(gc, &grid_default_cell, sizeof *gc);
  }
  memcpy(&(*tty).last_cell, &grid_default_cell, sizeof(*tty).last_cell);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_invalidate(tty: *mut tty ) {
unsafe {
  memcpy(&(*tty).cell, &grid_default_cell, sizeof(*tty).cell);
  memcpy(&(*tty).last_cell, &grid_default_cell, sizeof(*tty).last_cell);

  (*tty).cx = (*tty).cy = UINT_MAX;
  (*tty).rupper = (*tty).rleft = UINT_MAX;
  (*tty).rlower = (*tty).rright = UINT_MAX;

  if ((*tty).flags & TTY_STARTED) {
    if (tty_use_margin(tty)) {
      tty_putcode(tty, TTYC_ENMG);
    }
    tty_putcode(tty, TTYC_SGR0);

    (*tty).mode = ALL_MODES;
    tty_update_mode(tty, MODE_CURSOR, NULL);

    tty_cursor(tty, 0, 0);
    tty_region_off(tty);
    tty_margin_off(tty);
  } else {
    (*tty).mode = MODE_CURSOR;
  }
}
}

/* Turn off margin. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_region_off(tty: *mut tty ) { unsafe {tty_region(tty, 0, (*tty).sy - 1); }}

/* Set region inside pane. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_region_pane(tty: *mut tty, ctx: *const tty_ctx , rupper: u32, rlower: u32) {
unsafe {
  tty_region(tty, (*ctx).yoff + rupper - (*ctx).woy,
             (*ctx).yoff + rlower - (*ctx).woy);
}
}

/* Set region at absolute position. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_region(tty: *mut tty , rupper:u32, rlower: u32) {
unsafe {
  if ((*tty).rlower == rlower && (*tty).rupper == rupper) {
    return;
  }
  if (!tty_term_has((*tty).term, TTYC_CSR)) {
    return;
  }

  (*tty).rupper = rupper;
  (*tty).rlower = rlower;

  /*
   * Some terminals (such as PuTTY) do not correctly reset the cursor to
   * 0,0 if it is beyond the last column (they do not reset their wrap
   * flag so further output causes a line feed). As a workaround, do an
   * explicit move to 0 first.
   */
  if ((*tty).cx >= (*tty).sx) {
    if ((*tty).cy == UINT_MAX) {
      tty_cursor(tty, 0, 0);
    } else {
      tty_cursor(tty, 0, (*tty).cy);
    }
  }

  tty_putcode_ii(tty, TTYC_CSR, (*tty).rupper, (*tty).rlower);
  (*tty).cx = (*tty).cy = UINT_MAX;
}
}

/// Turn off margin.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_margin_off(tty: *mut tty) { unsafe {tty_margin(tty, 0, (*tty).sx - 1); }}

/// Set margin inside pane.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_margin_pane(tty: *mut tty, ctx: *const tty_ctx) {
unsafe {
  tty_margin(tty, (*ctx).xoff - (*ctx).wox, (*ctx).xoff + (*ctx).sx - 1 - (*ctx).wox);
}
}

/* Set margin at absolute position. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_margin(tty: *mut tty , rleft: u32, rright: u32) {
unsafe {
  if (!tty_use_margin(tty)) {
    return;
  }
  if ((*tty).rleft == rleft && (*tty).rright == rright) {
    return;
  }

  tty_putcode_ii(tty, TTYC_CSR, (*tty).rupper, (*tty).rlower);

  (*tty).rleft = rleft;
  (*tty).rright = rright;

  if (rleft == 0 && rright == (*tty).sx - 1) {
    tty_putcode(tty, TTYC_CLMG);
  } else {
    tty_putcode_ii(tty, TTYC_CMG, rleft, rright);
  }
  (*tty).cx = (*tty).cy = UINT_MAX;
}
}

/*
 * Move the cursor, unless it would wrap itself when the next character is
 * printed.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cursor_pane_unless_wrap(tty: *mut tty, ctx: *const tty_ctx, cx: u32, cy: u32) {
unsafe {
  if (!(*ctx).wrapped || !tty_full_width(tty, ctx) ||
      ((*(*tty).term).flags & TERM_NOAM) || (*ctx).xoff + cx != 0 ||
      (*ctx).yoff + cy != (*tty).cy + 1 || (*tty).cx < (*tty).sx ||
      (*tty).cy == (*tty).rlower) {
    tty_cursor_pane(tty, ctx, cx, cy);
  } else {
    log_debug("%s: will wrap at %u,%u", __func__, (*tty).cx, (*tty).cy);
  }
}
}

/* Move cursor inside pane. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cursor_pane(tty: *mut tty , ctx: *const tty_ctx , cx: u32, cy: u32) {
unsafe {
  tty_cursor(tty, (*ctx).xoff + cx - (*ctx).wox, (*ctx).yoff + cy - (*ctx).woy);
}
}

/* Move cursor to absolute position. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_cursor(tty: *mut tty , cx: u32, cy: u32) {
unsafe {
  struct tty_term *term = (*tty).term;
  u_int thisx, thisy;
  int change;

  if ((*tty).flags & TTY_BLOCK) {
    return;
  }

  thisx = (*tty).cx;
  thisy = (*tty).cy;

  /*
   * If in the automargin space, and want to be there, do not move.
   * Otherwise, force the cursor to be in range (and complain).
   */
  if (cx == thisx && cy == thisy && cx == (*tty).sx) {
    return;
  }
  if (cx > (*tty).sx - 1) {
    log_debug("%s: x too big %u > %u", __func__, cx, (*tty).sx - 1);
    cx = (*tty).sx - 1;
  }

  /* No change. */
  if (cx == thisx && cy == thisy) {
    return;
  }

  /* Currently at the very end of the line - use absolute movement. */
  if (thisx > (*tty).sx - 1) {
    goto absolute;
  }

  /* Move to home position (0, 0). */
  if (cx == 0 && cy == 0 && tty_term_has(term, TTYC_HOME)) {
    tty_putcode(tty, TTYC_HOME);
    goto out;
  }

  /* Zero on the next line. */
  if (cx == 0 && cy == thisy + 1 && thisy != (*tty).rlower &&
      (!tty_use_margin(tty) || (*tty).rleft == 0)) {
    tty_putc(tty, '\r');
    tty_putc(tty, '\n');
    goto out;
  }

  /* Moving column or row. */
  if (cy == thisy) {
    /*
     * Moving column only, row staying the same.
     */

    /* To left edge. */
    if (cx == 0 && (!tty_use_margin(tty) || (*tty).rleft == 0)) {
      tty_putc(tty, '\r');
      goto out;
    }

    /* One to the left. */
    if (cx == thisx - 1 && tty_term_has(term, TTYC_CUB1)) {
      tty_putcode(tty, TTYC_CUB1);
      goto out;
    }

    /* One to the right. */
    if (cx == thisx + 1 && tty_term_has(term, TTYC_CUF1)) {
      tty_putcode(tty, TTYC_CUF1);
      goto out;
    }

    /* Calculate difference. */
    change = thisx - cx; /* +ve left, -ve right */

    /*
     * Use HPA if change is larger than absolute, otherwise move
     * the cursor with CUB/CUF.
     */
    if ((u_int)abs(change) > cx && tty_term_has(term, TTYC_HPA)) {
      tty_putcode_i(tty, TTYC_HPA, cx);
      goto out;
    } else if (change > 0 && tty_term_has(term, TTYC_CUB) &&
               !tty_use_margin(tty)) {
      if (change == 2 && tty_term_has(term, TTYC_CUB1)) {
        tty_putcode(tty, TTYC_CUB1);
        tty_putcode(tty, TTYC_CUB1);
        goto out;
      }
      tty_putcode_i(tty, TTYC_CUB, change);
      goto out;
    } else if (change < 0 && tty_term_has(term, TTYC_CUF) &&
               !tty_use_margin(tty)) {
      tty_putcode_i(tty, TTYC_CUF, -change);
      goto out;
    }
  } else if (cx == thisx) {
    /*
     * Moving row only, column staying the same.
     */

    /* One above. */
    if (thisy != (*tty).rupper && cy == thisy - 1 &&
        tty_term_has(term, TTYC_CUU1)) {
      tty_putcode(tty, TTYC_CUU1);
      goto out;
    }

    /* One below. */
    if (thisy != (*tty).rlower && cy == thisy + 1 &&
        tty_term_has(term, TTYC_CUD1)) {
      tty_putcode(tty, TTYC_CUD1);
      goto out;
    }

    /* Calculate difference. */
    change = thisy - cy; /* +ve up, -ve down */

    /*
     * Try to use VPA if change is larger than absolute or if this
     * change would cross the scroll region, otherwise use CUU/CUD.
     */
    if ((u_int)abs(change) > cy ||
        (change < 0 && cy - change > (*tty).rlower) ||
        (change > 0 && cy - change < (*tty).rupper)) {
      if (tty_term_has(term, TTYC_VPA)) {
        tty_putcode_i(tty, TTYC_VPA, cy);
        goto out;
      }
    } else if (change > 0 && tty_term_has(term, TTYC_CUU)) {
      tty_putcode_i(tty, TTYC_CUU, change);
      goto out;
    } else if (change < 0 && tty_term_has(term, TTYC_CUD)) {
      tty_putcode_i(tty, TTYC_CUD, -change);
      goto out;
    }
  }

absolute:
  /* Absolute movement. */
  tty_putcode_ii(tty, TTYC_CUP, cy, cx);

out:
  (*tty).cx = cx;
  (*tty).cy = cy;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_hyperlink(tty *tty, gc: *const grid_cell , hl: *mut hyperlinks) {
unsafe {
  const char *uri, *id;

  if ((*gc).link == (*tty).cell.link) {
    return;
  }
  (*tty).cell.link = (*gc).link;

  if (hl == NULL) {
    return;
  }

  if ((*gc).link == 0 || !hyperlinks_get(hl, (*gc).link, &uri, NULL, &id)) {
    tty_putcode_ss(tty, TTYC_HLS, "", "");
  } else {
    tty_putcode_ss(tty, TTYC_HLS, id, uri);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_attributes(tty: *mut tty , gc: *const grid_cell , defaults: *const grid_cell, palette: *mut colour_palette , hl: *hyperlinks ) {
unsafe {
  let mut tc = &raw mut (*tty).cell;
// gc2;
  int changed;

  /* Copy cell and update default colours. */
  memcpy(&gc2, gc, sizeof gc2);
  if (~(*gc).flags & GRID_FLAG_NOPALETTE) {
    if (gc2.fg == 8) {
      gc2.fg = (*defaults).fg;
    }
    if (gc2.bg == 8) {
      gc2.bg = (*defaults).bg;
    }
  }

  /* Ignore cell if it is the same as the last one. */
  if (gc2.attr == (*tty).last_cell.attr && gc2.fg == (*tty).last_cell.fg &&
      gc2.bg == (*tty).last_cell.bg && gc2.us == (*tty).last_cell.us &&
      gc2.link == (*tty).last_cell.link) {
    return;
  }

  /*
   * If no setab, try to use the reverse attribute as a best-effort for a
   * non-default background. This is a bit of a hack but it doesn't do
   * any serious harm and makes a couple of applications happier.
   */
  if (!tty_term_has((*tty).term, TTYC_SETAB)) {
    if (gc2.attr & GRID_ATTR_REVERSE) {
      if (gc2.fg != 7 && !COLOUR_DEFAULT(gc2.fg)) {
        gc2.attr &= ~GRID_ATTR_REVERSE;
      }
    } else {
      if (gc2.bg != 0 && !COLOUR_DEFAULT(gc2.bg)) {
        gc2.attr |= GRID_ATTR_REVERSE;
      }
    }
  }

  /* Fix up the colours if necessary. */
  tty_check_fg(tty, palette, &gc2);
  tty_check_bg(tty, palette, &gc2);
  tty_check_us(tty, palette, &gc2);

  /*
   * If any bits are being cleared or the underline colour is now default,
   * reset everything.
   */
  if (((*tc).attr & ~gc2.attr) || ((*tc).us != gc2.us && gc2.us == 0)) {
    tty_reset(tty);
  }

  /*
   * Set the colours. This may call tty_reset() (so it comes next) and
   * may add to (NOT remove) the desired attributes.
   */
  tty_colours(tty, &gc2);

  /* Filter out attribute bits already set. */
  changed = gc2.attr & ~(*tc).attr;
  (*tc).attr = gc2.attr;

  /* Set the attributes. */
  if (changed & GRID_ATTR_BRIGHT) {
    tty_putcode(tty, TTYC_BOLD);
  }
  if (changed & GRID_ATTR_DIM) {
    tty_putcode(tty, TTYC_DIM);
  }
  if (changed & GRID_ATTR_ITALICS) {
    tty_set_italics(tty);
  }
  if (changed & GRID_ATTR_ALL_UNDERSCORE) {
    if ((changed & GRID_ATTR_UNDERSCORE) ||
        !tty_term_has((*tty).term, TTYC_SMULX)) {
      tty_putcode(tty, TTYC_SMUL);
    } else if (changed & GRID_ATTR_UNDERSCORE_2) {
      tty_putcode_i(tty, TTYC_SMULX, 2);
    } else if (changed & GRID_ATTR_UNDERSCORE_3) {
      tty_putcode_i(tty, TTYC_SMULX, 3);
    } else if (changed & GRID_ATTR_UNDERSCORE_4) {
      tty_putcode_i(tty, TTYC_SMULX, 4);
    } else if (changed & GRID_ATTR_UNDERSCORE_5) {
      tty_putcode_i(tty, TTYC_SMULX, 5);
    }
  }
  if (changed & GRID_ATTR_BLINK) {
    tty_putcode(tty, TTYC_BLINK);
  }
  if (changed & GRID_ATTR_REVERSE) {
    if (tty_term_has((*tty).term, TTYC_REV)) {
      tty_putcode(tty, TTYC_REV);
    } else if (tty_term_has((*tty).term, TTYC_SMSO)) {
      tty_putcode(tty, TTYC_SMSO);
    }
  }
  if (changed & GRID_ATTR_HIDDEN) {
    tty_putcode(tty, TTYC_INVIS);
  }
  if (changed & GRID_ATTR_STRIKETHROUGH) {
    tty_putcode(tty, TTYC_SMXX);
  }
  if (changed & GRID_ATTR_OVERLINE) {
    tty_putcode(tty, TTYC_SMOL);
  }
  if ((changed & GRID_ATTR_CHARSET) && tty_acs_needed(tty)) {
    tty_putcode(tty, TTYC_SMACS);
  }

  /* Set hyperlink if any. */
  tty_hyperlink(tty, gc, hl);

  memcpy(&(*tty).last_cell, &gc2, sizeof(*tty).last_cell);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_colours(tty: *mut tty , gc: *const grid_cell) {
unsafe {
  let mut tc = &raw mut (*tty).cell;

  /* No changes? Nothing is necessary. */
  if ((*gc).fg == (*tc).fg && (*gc).bg == (*tc).bg && (*gc).us == (*tc).us) {
    return;
  }

  /*
   * Is either the default colour? This is handled specially because the
   * best solution might be to reset both colours to default, in which
   * case if only one is default need to fall onward to set the other
   * colour.
   */
  if (COLOUR_DEFAULT((*gc).fg) || COLOUR_DEFAULT((*gc).bg)) {
    /*
     * If don't have AX, send sgr0. This resets both colours to default.
     * Otherwise, try to set the default colour only as needed.
     */
    if (!tty_term_flag((*tty).term, TTYC_AX)) {
      tty_reset(tty);
    } else {
      if (COLOUR_DEFAULT((*gc).fg) && !COLOUR_DEFAULT((*tc).fg)) {
        tty_puts(tty, "\033[39m");
        (*tc).fg = (*gc).fg;
      }
      if (COLOUR_DEFAULT((*gc).bg) && !COLOUR_DEFAULT((*tc).bg)) {
        tty_puts(tty, "\033[49m");
        (*tc).bg = (*gc).bg;
      }
    }
  }

  /* Set the foreground colour. */
  if (!COLOUR_DEFAULT((*gc).fg) && (*gc).fg != (*tc).fg) {
    tty_colours_fg(tty, gc);
  }

  /*
   * Set the background colour. This must come after the foreground as
   * tty_colours_fg() can call tty_reset().
   */
  if (!COLOUR_DEFAULT((*gc).bg) && (*gc).bg != (*tc).bg) {
    tty_colours_bg(tty, gc);
  }

  /* Set the underscore colour. */
  if ((*gc).us != (*tc).us) {
    tty_colours_us(tty, gc);
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_check_fg(tty: *mut tty , palette: *mut colour_palette , gc: *mut grid_cell ) {
unsafe {
  u_char r, g, b;
  u_int colours;
  int c;

  /*
   * Perform substitution if this pane has a palette. If the bright
   * attribute is set and Nobr is not present, use the bright entry in
   * the palette by changing to the aixterm colour
   */
  if (~(*gc).flags & GRID_FLAG_NOPALETTE) {
    c = (*gc).fg;
    if (c < 8 && (*gc).attr & GRID_ATTR_BRIGHT &&
        !tty_term_has((*tty).term, TTYC_NOBR)) {
      c += 90;
    }
    if ((c = colour_palette_get(palette, c)) != -1) {
      (*gc).fg = c;
    }
  }

  /* Is this a 24-bit colour? */
  if ((*gc).fg & COLOUR_FLAG_RGB) {
    /* Not a 24-bit terminal? Translate to 256-colour palette. */
    if ((*(*tty).term).flags & TERM_RGBCOLOURS) {
      return;
    }
    colour_split_rgb((*gc).fg, &r, &g, &b);
    (*gc).fg = colour_find_rgb(r, g, b);
  }

  /* How many colours does this terminal have? */
  if ((*(*tty).term).flags & TERM_256COLOURS) {
    colours = 256;
  } else {
    colours = tty_term_number((*tty).term, TTYC_COLORS);
  }

  /* Is this a 256-colour colour? */
  if ((*gc).fg & COLOUR_FLAG_256) {
    /* And not a 256 colour mode? */
    if (colours < 256) {
      (*gc).fg = colour_256to16((*gc).fg);
      if ((*gc).fg & 8) {
        (*gc).fg &= 7;
        if (colours >= 16) {
          (*gc).fg += 90;
        }
      }
    }
    return;
  }

  /* Is this an aixterm colour? */
  if ((*gc).fg >= 90 && (*gc).fg <= 97 && colours < 16) {
    (*gc).fg -= 90;
    (*gc).attr |= GRID_ATTR_BRIGHT;
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_check_bg(tty: *mut tty , palette: *mut colour_palette , gc: *mut grid_cell ) {
unsafe {
  u_char r, g, b;
  u_int colours;
  int c;

  /* Perform substitution if this pane has a palette. */
  if (~(*gc).flags & GRID_FLAG_NOPALETTE) {
    if ((c = colour_palette_get(palette, (*gc).bg)) != -1) {
      (*gc).bg = c;
    }
  }

  /* Is this a 24-bit colour? */
  if ((*gc).bg & COLOUR_FLAG_RGB) {
    /* Not a 24-bit terminal? Translate to 256-colour palette. */
    if ((*(*tty).term).flags & TERM_RGBCOLOURS) {
      return;
    }
    colour_split_rgb((*gc).bg, &r, &g, &b);
    (*gc).bg = colour_find_rgb(r, g, b);
  }

  /* How many colours does this terminal have? */
  if ((*(*tty).term).flags & TERM_256COLOURS) {
    colours = 256;
  } else {
    colours = tty_term_number((*tty).term, TTYC_COLORS);
  }

  /* Is this a 256-colour colour? */
  if ((*gc).bg & COLOUR_FLAG_256) {
    /*
     * And not a 256 colour mode? Translate to 16-colour
     * palette. Bold background doesn't exist portably, so just
     * discard the bold bit if set.
     */
    if (colours < 256) {
      (*gc).bg = colour_256to16((*gc).bg);
      if ((*gc).bg & 8) {
        (*gc).bg &= 7;
        if (colours >= 16) {
          (*gc).bg += 90;
        }
      }
    }
    return;
  }

  /* Is this an aixterm colour? */
  if ((*gc).bg >= 90 && (*gc).bg <= 97 && colours < 16) {
    (*gc).bg -= 90;
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_check_us(_tty: *mut tty, palette: *mut colour_palette , gc: *mut grid_cell) {
unsafe {
  int c;

  /* Perform substitution if this pane has a palette. */
  if (~(*gc).flags & GRID_FLAG_NOPALETTE) {
    if ((c = colour_palette_get(palette, (*gc).us)) != -1) {
      (*gc).us = c;
    }
  }

  /* Convert underscore colour if only RGB can be supported. */
  if (!tty_term_has((*tty).term, TTYC_SETULC1)) {
    if ((c = colour_force_rgb((*gc).us)) == -1) {
      (*gc).us = 8;
    } else {
      (*gc).us = c;
    }
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_colours_fg(tty: *mut tty , gc: *const grid_cell ) {
unsafe {
  let mut tc = &raw mut (*tty).cell;
  char s[32];

  /*
   * If the current colour is an aixterm bright colour and the new is not,
   * reset because some terminals do not clear bright correctly.
   */
  if ((*tty).cell.fg >= 90 && (*tty).cell.bg <= 97 &&
      ((*gc).fg < 90 || (*gc).fg > 97)) {
    tty_reset(tty);
  }

  /* Is this a 24-bit or 256-colour colour? */
  if ((*gc).fg & COLOUR_FLAG_RGB || (*gc).fg & COLOUR_FLAG_256) {
    if (tty_try_colour(tty, (*gc).fg, "38") == 0) {
      goto save;
    }
    /* Should not get here, already converted in tty_check_fg. */
    return;
  }

  /* Is this an aixterm bright colour? */
  if ((*gc).fg >= 90 && (*gc).fg <= 97) {
    if ((*(*tty).term).flags & TERM_256COLOURS) {
      xsnprintf(s, sizeof s, "\033[%dm", (*gc).fg);
      tty_puts(tty, s);
    } else {
      tty_putcode_i(tty, TTYC_SETAF, (*gc).fg - 90 + 8);
    }
    goto save;
  }

  /* Otherwise set the foreground colour. */
  tty_putcode_i(tty, TTYC_SETAF, (*gc).fg);

save:
  /* Save the new values in the terminal current cell. */
  (*tc).fg = (*gc).fg;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_colours_bg(tty: *mut tty , gc: *const grid_cell ) {
unsafe {
  let mut tc = &raw mut (*tty).cell;
  char s[32];

  /* Is this a 24-bit or 256-colour colour? */
  if ((*gc).bg & COLOUR_FLAG_RGB || (*gc).bg & COLOUR_FLAG_256) {
    if (tty_try_colour(tty, (*gc).bg, "48") == 0) {
      goto save;
    }
    /* Should not get here, already converted in tty_check_bg. */
    return;
  }

  /* Is this an aixterm bright colour? */
  if ((*gc).bg >= 90 && (*gc).bg <= 97) {
    if ((*(*tty).term).flags & TERM_256COLOURS) {
      xsnprintf(s, sizeof s, "\033[%dm", (*gc).bg + 10);
      tty_puts(tty, s);
    } else {
      tty_putcode_i(tty, TTYC_SETAB, (*gc).bg - 90 + 8);
    }
    goto save;
  }

  /* Otherwise set the background colour. */
  tty_putcode_i(tty, TTYC_SETAB, (*gc).bg);

save:
  /* Save the new values in the terminal current cell. */
  (*tc).bg = (*gc).bg;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_colours_us(tty: *mut tty , gc: *const grid_cell) {
unsafe {
  let mut tc = &raw mut (*tty).cell;
  u_int c;
  u_char r, g, b;

  /* Clear underline colour. */
  if (COLOUR_DEFAULT((*gc).us)) {
    tty_putcode(tty, TTYC_OL);
    goto save;
  }

  /*
   * If this is not an RGB colour, use Setulc1 if it exists, otherwise
   * convert.
   */
  if (~(*gc).us & COLOUR_FLAG_RGB) {
    c = (*gc).us;
    if ((~c & COLOUR_FLAG_256) && (c >= 90 && c <= 97)) {
      c -= 82;
    }
    tty_putcode_i(tty, TTYC_SETULC1, c & ~COLOUR_FLAG_256);
    return;
  }

  /*
   * Setulc and setal follows the ncurses(3) one argument "direct colour"
   * capability format. Calculate the colour value.
   */
  colour_split_rgb((*gc).us, &r, &g, &b);
  c = (65536 * r) + (256 * g) + b;

  /*
   * Write the colour. Only use setal if the RGB flag is set because the
   * non-RGB version may be wrong.
   */
  if (tty_term_has((*tty).term, TTYC_SETULC)) {
    tty_putcode_i(tty, TTYC_SETULC, c);
  } else if (tty_term_has((*tty).term, TTYC_SETAL) &&
             tty_term_has((*tty).term, TTYC_RGB)) {
    tty_putcode_i(tty, TTYC_SETAL, c);
  }

save:
  /* Save the new values in the terminal current cell. */
  (*tc).us = (*gc).us;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_try_colour(tty: *mut tty , colour: i32, type_: *const c_char) -> i32 {
unsafe {
  u_char r, g, b;

  if (colour & COLOUR_FLAG_256) {
    if (*type == '3' && tty_term_has((*tty).term, TTYC_SETAF)) {
      tty_putcode_i(tty, TTYC_SETAF, colour & 0xff);
    } else if (tty_term_has((*tty).term, TTYC_SETAB)) {
      tty_putcode_i(tty, TTYC_SETAB, colour & 0xff);
    }
    return 0;
  }

  if (colour & COLOUR_FLAG_RGB) {
    colour_split_rgb(colour & 0xffffff, &r, &g, &b);
    if (*type == '3' && tty_term_has((*tty).term, TTYC_SETRGBF)) {
      tty_putcode_iii(tty, TTYC_SETRGBF, r, g, b);
    } else if (tty_term_has((*tty).term, TTYC_SETRGBB)) {
      tty_putcode_iii(tty, TTYC_SETRGBB, r, g, b);
    }
    return 0;
  }

  -1
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_window_default_style(gc:*mut grid_cell, wp: *mut window_pane ) {
unsafe {
  memcpy__(gc, &raw const grid_default_cell);
  (*gc).fg = (*wp).palette.fg;
  (*gc).bg = (*wp).palette.bg;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_default_colours(gc: *mut grid_cell , wp: *mut window_pane) {
unsafe {
  let oo = (*wp).options;
  struct format_tree *ft;

  memcpy(gc, &grid_default_cell, sizeof *gc);

  if ((*wp).flags & PANE_STYLECHANGED) {
    log_debug("%%%u: style changed", (*wp).id);
    (*wp).flags &= ~PANE_STYLECHANGED;

    ft = format_create(NULL, NULL, FORMAT_PANE | (*wp).id, FORMAT_NOJOBS);
    format_defaults(ft, NULL, NULL, NULL, wp);
    tty_window_default_style(&(*wp).cached_active_gc, wp);
    style_add(&(*wp).cached_active_gc, oo, "window-active-style", ft);
    tty_window_default_style(&(*wp).cached_gc, wp);
    style_add(&(*wp).cached_gc, oo, "window-style", ft);
    format_free(ft);
  }

  if ((*gc).fg == 8) {
    if (wp == (*(*wp).window).active && (*wp).cached_active_gc.fg != 8) {
      (*gc).fg = (*wp).cached_active_gc.fg;
    } else {
      (*gc).fg = (*wp).cached_gc.fg;
    }
  }

  if ((*gc).bg == 8) {
    if (wp == (*(*wp).window).active && (*wp).cached_active_gc.bg != 8) {
      (*gc).bg = (*wp).cached_active_gc.bg;
    } else {
      (*gc).bg = (*wp).cached_gc.bg;
    }
  }
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_default_attributes(tty: *mut tty , defaults: *const grid_cell , palette: *mut colour_palette , bg: u32, hl: *mut hyperlinks) {
unsafe {
  struct grid_cell gc;

  memcpy(&gc, &grid_default_cell, sizeof gc);
  gc.bg = bg;
  tty_attributes(tty, &gc, defaults, palette, hl);
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clipboard_query_callback(_fd: i32, _events: i16, data: *mut c_void) {
unsafe {
  let mut tty : *mut tty = data.cast();
  let mut c = (*tty).client;

  (*c).flags &= ~CLIENT_CLIPBOARDBUFFER;
  free((*c).clipboard_panes);
  (*c).clipboard_panes = NULL;
  (*c).clipboard_npanes = 0;

  (*tty).flags &= ~TTY_OSC52QUERY;
}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn
tty_clipboard_query(tty: *mut tty) {
unsafe {
  let mut tv = libc::timeval {tv_sec : TTY_QUERY_TIMEOUT};

  if ((~(*tty).flags & TTY_STARTED) || ((*tty).flags & TTY_OSC52QUERY)) {
    return;
  }
  tty_putcode_ss(tty, TTYC_MS, "", "?");

  (*tty).flags |= TTY_OSC52QUERY;
  evtimer_set(&(*tty).clipboard_timer, tty_clipboard_query_callback, tty);
  evtimer_add(&(*tty).clipboard_timer, &tv);
}
}
*/
