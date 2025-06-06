use super::*;

unsafe extern "C" {
    pub fn tty_create_log();
    pub fn tty_window_bigger(_: *mut tty) -> c_int;
    pub fn tty_window_offset(
        _: *mut tty,
        _: *mut c_uint,
        _: *mut c_uint,
        _: *mut c_uint,
        _: *mut c_uint,
    ) -> c_int;
    pub fn tty_update_window_offset(_: *mut window);
    pub fn tty_update_client_offset(_: *mut client);
    pub fn tty_raw(_: *mut tty, _: *const c_char);
    pub fn tty_attributes(
        _: *mut tty,
        _: *const grid_cell,
        _: *const grid_cell,
        _: *mut colour_palette,
        _: *mut hyperlinks,
    );
    pub fn tty_reset(_: *mut tty);
    pub fn tty_region_off(_: *mut tty);
    pub fn tty_m_in_off(_: *mut tty);
    pub fn tty_cursor(_: *mut tty, _: c_uint, _: c_uint);
    pub fn tty_clipboard_query(_: *mut tty);
    pub fn tty_putcode(_: *mut tty, _: tty_code_code);
    pub fn tty_putcode_i(_: *mut tty, _: tty_code_code, _: c_int);
    pub fn tty_putcode_ii(_: *mut tty, _: tty_code_code, _: c_int, _: c_int);
    pub fn tty_putcode_iii(_: *mut tty, _: tty_code_code, _: c_int, _: c_int, _: c_int);
    pub fn tty_putcode_s(_: *mut tty, _: tty_code_code, _: *const c_char);
    pub fn tty_putcode_ss(_: *mut tty, _: tty_code_code, _: *const c_char, _: *const c_char);
    pub fn tty_puts(_: *mut tty, _: *const c_char);
    pub fn tty_putc(_: *mut tty, _: c_uchar);
    pub fn tty_putn(_: *mut tty, _: *const c_void, _: usize, _: c_uint);
    pub fn tty_cell(
        _: *mut tty,
        _: *const grid_cell,
        _: *const grid_cell,
        _: *mut colour_palette,
        _: *mut hyperlinks,
    );
    pub fn tty_init(_: *mut tty, _: *mut client) -> c_int;
    pub fn tty_resize(_: *mut tty);
    pub fn tty_set_size(_: *mut tty, _: c_uint, _: c_uint, _: c_uint, _: c_uint);
    pub fn tty_start_tty(_: *mut tty);
    pub fn tty_send_requests(_: *mut tty);
    pub fn tty_repeat_requests(_: *mut tty);
    pub fn tty_stop_tty(_: *mut tty);
    pub fn tty_set_title(_: *mut tty, _: *const c_char);
    pub fn tty_set_path(_: *mut tty, _: *const c_char);
    pub fn tty_update_mode(_: *mut tty, _: c_int, _: *mut screen);
    pub fn tty_draw_line(
        _: *mut tty,
        _: *mut screen,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: c_uint,
        _: *const grid_cell,
        _: *mut colour_palette,
    );
    pub fn tty_sync_start(_: *mut tty);
    pub fn tty_sync_end(_: *mut tty);
    pub fn tty_open(_: *mut tty, _: *mut *mut c_char) -> c_int;
    pub fn tty_close(_: *mut tty);
    pub fn tty_free(_: *mut tty);
    pub fn tty_update_features(_: *mut tty);
    pub fn tty_set_selection(_: *mut tty, _: *const c_char, _: *const c_char, _: usize);
    pub fn tty_write(
        _: Option<unsafe extern "C" fn(_: *mut tty, _: *const tty_ctx)>,
        _: *mut tty_ctx,
    );
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
}
