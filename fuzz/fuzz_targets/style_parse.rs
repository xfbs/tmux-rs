#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("style_parse");

    // Only feed valid UTF-8 to avoid known cstr_to_str panics on non-UTF-8.
    if std::str::from_utf8(data).is_err() {
        return;
    }
    if data.contains(&0) {
        return;
    }

    let mut cstr = Vec::with_capacity(data.len() + 1);
    cstr.extend_from_slice(data);
    cstr.push(0);

    let _ = tmux_rs_new::style_::fuzz_style_parse(&cstr);
});
