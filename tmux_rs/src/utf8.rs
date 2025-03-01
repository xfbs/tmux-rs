use compat_rs::tree::rb_initializer;

use crate::*;
unsafe extern "C" {
    pub unsafe fn utf8_towc(_: *const utf8_data, _: *mut wchar_t) -> utf8_state;
    pub unsafe fn utf8_fromwc(wc: wchar_t, _: *mut utf8_data) -> utf8_state;
    pub unsafe fn utf8_in_table(_: wchar_t, _: *const wchar_t, _: c_uint) -> c_int;
    pub unsafe fn utf8_build_one(_: c_uchar) -> utf8_char;
    pub unsafe fn utf8_from_data(_: *const utf8_data, _: *mut utf8_char) -> utf8_state;
    pub unsafe fn utf8_to_data(_: utf8_char, _: *mut utf8_data);
    pub unsafe fn utf8_set(_: *mut utf8_data, _: c_uchar);
    pub unsafe fn utf8_copy(_: *mut utf8_data, _: *const utf8_data);
    pub unsafe fn utf8_open(_: *mut utf8_data, _: c_uchar) -> utf8_state;
    pub unsafe fn utf8_append(_: *mut utf8_data, _: c_uchar) -> utf8_state;
    pub unsafe fn utf8_isvalid(_: *const c_char) -> c_int;
    pub unsafe fn utf8_strvis(_: *mut c_char, _: *const c_char, _: usize, _: c_int) -> c_int;
    pub unsafe fn utf8_stravis(_: *mut *mut c_char, _: *const c_char, _: c_int) -> c_int;
    pub unsafe fn utf8_stravisx(_: *mut *mut c_char, _: *const c_char, _: usize, _: c_int) -> c_int;
    pub unsafe fn utf8_sanitize(_: *const c_char) -> *mut c_char;
    pub unsafe fn utf8_strlen(_: *const utf8_data) -> usize;
    pub unsafe fn utf8_strwidth(_: *const utf8_data, _: isize) -> c_uint;
    pub unsafe fn utf8_fromcstr(_: *const c_char) -> *mut utf8_data;
    pub unsafe fn utf8_tocstr(_: *mut utf8_data) -> *mut c_char;
    pub unsafe fn utf8_cstrwidth(_: *const c_char) -> c_uint;
    pub unsafe fn utf8_padcstr(_: *const c_char, _: c_uint) -> *mut c_char;
    pub unsafe fn utf8_rpadcstr(_: *const c_char, _: c_uint) -> *mut c_char;
    pub unsafe fn utf8_cstrhas(_: *const c_char, _: *const utf8_data) -> c_int;
}

static utf8_force_wide: [wchar_t; 162] = [
    0x0261D, 0x026F9, 0x0270A, 0x0270B, 0x0270C, 0x0270D, 0x1F1E6, 0x1F1E7, 0x1F1E8, 0x1F1E9, 0x1F1EA, 0x1F1EB,
    0x1F1EC, 0x1F1ED, 0x1F1EE, 0x1F1EF, 0x1F1F0, 0x1F1F1, 0x1F1F2, 0x1F1F3, 0x1F1F4, 0x1F1F5, 0x1F1F6, 0x1F1F7,
    0x1F1F8, 0x1F1F9, 0x1F1FA, 0x1F1FB, 0x1F1FC, 0x1F1FD, 0x1F1FE, 0x1F1FF, 0x1F385, 0x1F3C2, 0x1F3C3, 0x1F3C4,
    0x1F3C7, 0x1F3CA, 0x1F3CB, 0x1F3CC, 0x1F3FB, 0x1F3FC, 0x1F3FD, 0x1F3FE, 0x1F3FF, 0x1F442, 0x1F443, 0x1F446,
    0x1F447, 0x1F448, 0x1F449, 0x1F44A, 0x1F44B, 0x1F44C, 0x1F44D, 0x1F44E, 0x1F44F, 0x1F450, 0x1F466, 0x1F467,
    0x1F468, 0x1F469, 0x1F46B, 0x1F46C, 0x1F46D, 0x1F46E, 0x1F470, 0x1F471, 0x1F472, 0x1F473, 0x1F474, 0x1F475,
    0x1F476, 0x1F477, 0x1F478, 0x1F47C, 0x1F481, 0x1F482, 0x1F483, 0x1F485, 0x1F486, 0x1F487, 0x1F48F, 0x1F491,
    0x1F4AA, 0x1F574, 0x1F575, 0x1F57A, 0x1F590, 0x1F595, 0x1F596, 0x1F645, 0x1F646, 0x1F647, 0x1F64B, 0x1F64C,
    0x1F64D, 0x1F64E, 0x1F64F, 0x1F6A3, 0x1F6B4, 0x1F6B5, 0x1F6B6, 0x1F6C0, 0x1F6CC, 0x1F90C, 0x1F90F, 0x1F918,
    0x1F919, 0x1F91A, 0x1F91B, 0x1F91C, 0x1F91D, 0x1F91E, 0x1F91F, 0x1F926, 0x1F930, 0x1F931, 0x1F932, 0x1F933,
    0x1F934, 0x1F935, 0x1F936, 0x1F937, 0x1F938, 0x1F939, 0x1F93D, 0x1F93E, 0x1F977, 0x1F9B5, 0x1F9B6, 0x1F9B8,
    0x1F9B9, 0x1F9BB, 0x1F9CD, 0x1F9CE, 0x1F9CF, 0x1F9D1, 0x1F9D2, 0x1F9D3, 0x1F9D4, 0x1F9D5, 0x1F9D6, 0x1F9D7,
    0x1F9D8, 0x1F9D9, 0x1F9DA, 0x1F9DB, 0x1F9DC, 0x1F9DD, 0x1FAC3, 0x1FAC4, 0x1FAC5, 0x1FAF0, 0x1FAF1, 0x1FAF2,
    0x1FAF3, 0x1FAF4, 0x1FAF5, 0x1FAF6, 0x1FAF7, 0x1FAF8,
];

#[repr(C)]
pub struct utf8_item {
    pub index_entry: rb_entry<utf8_item>,
    pub index: u32,

    pub data_entry: rb_entry<utf8_item>,
    pub data: [c_char; UTF8_SIZE],
    pub size: c_uchar,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_data_cmp(ui1: *const utf8_item, ui2: *const utf8_item) -> i32 {
    unsafe {
        if ((*ui1).size < (*ui2).size) {
            return -1;
        }
        if ((*ui1).size > (*ui2).size) {
            return 1;
        }
        memcmp(
            (*ui1).data.as_ptr().cast(),
            (*ui2).data.as_ptr().cast(),
            (*ui1).size as usize,
        )
    }
}

pub type utf8_data_tree = rb_head<utf8_item>;
RB_GENERATE!(utf8_data_tree, utf8_item, data_entry, utf8_data_cmp);
static mut utf8_data_tree: utf8_data_tree = rb_initializer();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_index_cmp(ui1: *const utf8_item, ui2: *const utf8_item) -> i32 {
    unsafe {
        if ((*ui1).index < (*ui2).index) {
            return -1;
        }
        if ((*ui1).index > (*ui2).index) {
            return 1;
        }
    }
    0
}

pub type utf8_index_tree = rb_head<utf8_item>;
// RB_GENERATE!(utf8_index_tree, utf8_item, index_entry, utf8_index_cmp);
static mut utf8_index_tree: utf8_index_tree = rb_initializer();

static mut utf8_next_index: u32 = 0;

fn UTF8_GET_SIZE(uc: utf8_char) -> utf8_char { (((uc) >> 24) & 0x1f) }
fn UTF8_GET_WIDTH(uc: utf8_char) -> utf8_char { (((uc) >> 29) - 1) }

fn UTF8_SET_SIZE(size: utf8_char) -> utf8_char { size << 24 }
fn UTF8_SET_WIDTH(width: utf8_char) -> utf8_char { (width + 1) << 29 }
