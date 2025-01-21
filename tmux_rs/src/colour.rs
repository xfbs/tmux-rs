use super::*;
unsafe extern "C" {
    pub fn colour_find_rgb(_: c_uchar, _: c_uchar, _: c_uchar) -> c_int;
    pub fn colour_join_rgb(_: c_uchar, _: c_uchar, _: c_uchar) -> c_int;
    pub fn colour_split_rgb(_: c_int, _: *mut c_uchar, _: *mut c_uchar, _: *mut c_uchar);
    pub fn colour_force_rgb(_: c_int) -> c_int;
    pub fn colour_tostring(_: c_int) -> *const c_char;
    pub fn colour_fromstring(s: *const c_char) -> c_int;
    pub fn colour_256toRGB(_: c_int) -> c_int;
    pub fn colour_256to16(_: c_int) -> c_int;
    pub fn colour_byname(_: *const c_char) -> c_int;
    pub fn colour_parseX11(_: *const c_char) -> c_int;
    pub fn colour_palette_init(_: *mut colour_palette);
    pub fn colour_palette_clear(_: *mut colour_palette);
    pub fn colour_palette_free(_: *mut colour_palette);
    pub fn colour_palette_get(_: *mut colour_palette, _: c_int) -> c_int;
    pub fn colour_palette_set(_: *mut colour_palette, _: c_int, _: c_int) -> c_int;
    pub fn colour_palette_from_option(_: *mut colour_palette, _: *mut options);
}
