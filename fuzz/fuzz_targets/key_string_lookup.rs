#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("key_string_lookup");

    // key_string_lookup_string expects a NUL-terminated C string.
    if data.contains(&0) {
        return;
    }

    let mut cstr = Vec::with_capacity(data.len() + 1);
    cstr.extend_from_slice(data);
    cstr.push(0);

    unsafe {
        // Returns KEYC_UNKNOWN or a valid key_code. Either is fine.
        let _ = tmux_rs_new::key_string::key_string_lookup_string(cstr.as_ptr());
    }
});
