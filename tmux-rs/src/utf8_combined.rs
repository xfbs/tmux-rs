use crate::{utf8_data, utf8_in_table, utf8_state, utf8_towc, wchar_t};

use core::ffi::c_void;
use libc::memcmp;

unsafe extern "C" {
    // pub unsafe fn utf8_has_zwj(_: *const utf8_data) -> c_int;
    // pub unsafe fn utf8_is_zwj(_: *const utf8_data) -> c_int;
    // pub unsafe fn utf8_is_vs(_: *const utf8_data) -> c_int;
    // pub unsafe fn utf8_is_modifier(_: *const utf8_data) -> c_int;
}

static utf8_modifier_table: [wchar_t; 31] = [
    0x1F1E6, 0x1F1E7, 0x1F1E8, 0x1F1E9, 0x1F1EA, 0x1F1EB, 0x1F1EC, 0x1F1ED, 0x1F1EE, 0x1F1EF,
    0x1F1F0, 0x1F1F1, 0x1F1F2, 0x1F1F3, 0x1F1F4, 0x1F1F5, 0x1F1F6, 0x1F1F7, 0x1F1F8, 0x1F1F9,
    0x1F1FA, 0x1F1FB, 0x1F1FC, 0x1F1FD, 0x1F1FE, 0x1F1FF, 0x1F3FB, 0x1F3FC, 0x1F3FD, 0x1F3FE,
    0x1F3FF,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_has_zwj(ud: *const utf8_data) -> i32 {
    unsafe {
        if (*ud).size < 3 {
            return 0;
        }
        (memcmp(
            &raw const (*ud).data[((*ud).size - 3) as usize] as *const c_void,
            b"\xe2\x80\x8d\x00" as *const u8 as *const c_void,
            3,
        ) == 0) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_is_zwj(ud: *const utf8_data) -> i32 {
    unsafe {
        if ((*ud).size != 3) {
            return 0;
        }
        (memcmp(
            &raw const (*ud).data as *const u8 as *const c_void,
            b"\xe2\x80\x8d\x00" as *const u8 as *const c_void,
            3,
        ) == 0) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_is_vs(ud: *const utf8_data) -> i32 {
    unsafe {
        if ((*ud).size != 3) {
            return 0;
        }
        (memcmp(
            &raw const (*ud).data as *const u8 as *const c_void,
            b"\xef\xbf\x8f\x00" as *const u8 as *const c_void,
            3,
        ) == 0) as i32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn utf8_is_modifier(ud: *const utf8_data) -> i32 {
    let mut wc: wchar_t = 0;
    unsafe {
        if utf8_towc(ud, &raw mut wc) != utf8_state::UTF8_DONE {
            return 0;
        }
        if utf8_in_table(
            wc,
            &raw const utf8_modifier_table as *const wchar_t,
            utf8_modifier_table.len() as u32,
        ) == 0
        {
            return 0;
        }
        1
    }
}
