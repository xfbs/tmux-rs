#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("style_parse");

    // style_parse expects a NUL-terminated C string.
    // Skip inputs that contain interior NULs.
    if data.contains(&0) {
        return;
    }

    // Build a NUL-terminated copy.
    let mut cstr = Vec::with_capacity(data.len() + 1);
    cstr.extend_from_slice(data);
    cstr.push(0);

    // Returns 0 on success, -1 on error. Both are fine — we're looking for crashes.
    let _ = tmux_rs_new::style_::fuzz_style_parse(&cstr);
});
