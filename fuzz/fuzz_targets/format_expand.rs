#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("format_expand");

    // Only feed valid UTF-8 to avoid known cstr_to_str panics on non-UTF-8.
    if std::str::from_utf8(data).is_err() {
        return;
    }

    tmux_rs_new::format::fuzz_format_expand(data);
});
