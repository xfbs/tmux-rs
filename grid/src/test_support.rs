//! Test-only helpers. Provides a minimal [`Utf8Codec`] implementation for
//! the crate's self-contained test suite (the real codec lives in
//! `tmux-rs::utf8` and isn't available from a standalone `cargo test
//! -p tmux-grid` run).
//!
//! Use [`install_test_codec`] from each test that might reach the codec
//! (any grid operation that round-trips cells through the extended-cell
//! side-table, or any reader call that uses `cstr_has`). Calling it more
//! than once is safe — the underlying `set_codec` is idempotent.

#![cfg(test)]

use crate::{Utf8Codec, Utf8State, set_codec};
use tmux_types::{UTF8_SIZE, utf8_char, utf8_data};

/// Trivial codec: rejects >3-byte inputs (no intern table), performs the
/// same bit-packing as tmux-rs's real codec for <=3-byte inputs, and
/// bytes-compare NUL-terminated byte sets for `cstr_has`.
pub struct TestCodec;

impl Utf8Codec for TestCodec {
    unsafe fn from_data(&self, ud: *const utf8_data, uc: *mut utf8_char) -> Utf8State {
        let size = unsafe { (*ud).size };
        let width = unsafe { (*ud).width };
        if size as usize > UTF8_SIZE || size > 3 {
            // Tests don't exercise multi-byte interning; substitute an
            // error cell matching the behavior of the real codec.
            unsafe {
                *uc = (size as u32) << 24 | (width as u32 + 1) << 29;
            }
            return Utf8State::Error;
        }
        let index = unsafe {
            ((*ud).data[2] as u32) << 16 | ((*ud).data[1] as u32) << 8 | (*ud).data[0] as u32
        };
        unsafe {
            *uc = (size as u32) << 24 | (width as u32 + 1) << 29 | index;
        }
        Utf8State::Done
    }

    fn to_data(&self, uc: utf8_char) -> utf8_data {
        let size = ((uc >> 24) & 0x1f) as u8;
        let width = ((uc >> 29) - 1) as u8;
        let mut data = [0u8; UTF8_SIZE];
        if size <= 3 {
            data[0] = (uc & 0xff) as u8;
            data[1] = ((uc >> 8) & 0xff) as u8;
            data[2] = ((uc >> 16) & 0xff) as u8;
        }
        utf8_data { data, have: size, size, width }
    }

    unsafe fn cstr_has(&self, set: *const u8, ud: *const utf8_data) -> bool {
        // Linear scan of the NUL-terminated `set` for the first byte of
        // `ud`. Matches tmux-rs's `utf8_cstrhas` semantics for the ASCII
        // cases the test suite exercises.
        if set.is_null() {
            return false;
        }
        let first = unsafe { (*ud).data[0] };
        let mut p = set;
        unsafe {
            while *p != 0 {
                if *p == first {
                    return true;
                }
                p = p.add(1);
            }
        }
        false
    }
}

static TEST_CODEC: TestCodec = TestCodec;

/// Register the test codec. Idempotent across threads.
pub fn install_test_codec() {
    set_codec(&TEST_CODEC);
}
