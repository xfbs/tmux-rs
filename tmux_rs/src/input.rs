use super::*;

unsafe extern "C" {
    pub fn input_init(_: *mut window_pane, _: *mut bufferevent, _: *mut colour_palette) -> *mut input_ctx;
    pub fn input_free(_: *mut input_ctx);
    pub fn input_reset(_: *mut input_ctx, _: c_int);
    pub fn input_pending(_: *mut input_ctx) -> *mut evbuffer;
    pub fn input_parse_pane(_: *mut window_pane);
    pub fn input_parse_buffer(_: *mut window_pane, _: *mut c_uchar, _: usize);
    pub fn input_parse_screen(
        _: *mut input_ctx,
        _: *mut screen,
        _: screen_write_init_ctx_cb,
        _: *mut c_void,
        _: *mut c_uchar,
        _: usize,
    );
    pub fn input_reply_clipboard(_: *mut bufferevent, _: *const c_char, _: usize, _: *const c_char);
}
