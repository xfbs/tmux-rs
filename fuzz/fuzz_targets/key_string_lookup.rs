#![no_main]

use libfuzzer_sys::fuzz_target;
use tmux_rs_new::key_string::key_string_lookup_string;

fuzz_target!(|data: &[u8]| {
    // key_string_lookup_string expects a NUL-terminated C string.
    if data.contains(&0) {
        return;
    }

    let mut cstr = Vec::with_capacity(data.len() + 1);
    cstr.extend_from_slice(data);
    cstr.push(0);

    unsafe {
        // Returns KEYC_UNKNOWN or a valid key_code. Either is fine.
        let _ = key_string_lookup_string(cstr.as_ptr());
    }
});
