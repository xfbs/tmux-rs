use super::*;

unsafe extern "C" {
    pub static mut status_prompt_hlist: [*mut *mut c_char; 0usize];
    pub static mut status_prompt_hsize: [c_uint; 0usize];
    pub fn status_timer_start(_: *mut client);
    pub fn status_timer_start_all();
    pub fn status_update_cache(_: *mut session);
    pub fn status_at_line(_: *mut client) -> c_int;
    pub fn status_line_size(_: *mut client) -> c_uint;
    pub fn status_get_range(_: *mut client, _: c_uint, _: c_uint) -> *mut style_range;
    pub fn status_init(_: *mut client);
    pub fn status_free(_: *mut client);
    pub fn status_redraw(_: *mut client) -> c_int;
    pub fn status_message_set(_: *mut client, _: c_int, _: c_int, _: c_int, _: *const c_char, ...);
    pub fn status_message_clear(_: *mut client);
    pub fn status_message_redraw(_: *mut client) -> c_int;
    pub fn status_prompt_set(
        _: *mut client,
        _: *mut cmd_find_state,
        _: *const c_char,
        _: *const c_char,
        _: prompt_input_cb,
        _: prompt_free_cb,
        _: *mut c_void,
        _: c_int,
        _: prompt_type,
    );
    pub fn status_prompt_clear(_: *mut client);
    pub fn status_prompt_redraw(_: *mut client) -> c_int;
    pub fn status_prompt_key(_: *mut client, _: key_code) -> c_int;
    pub fn status_prompt_update(_: *mut client, _: *const c_char, _: *const c_char);
    pub fn status_prompt_load_history();
    pub fn status_prompt_save_history();
    pub fn status_prompt_type_string(_: c_uint) -> *const c_char;
    pub fn status_prompt_type(type_: *const c_char) -> prompt_type;
}
