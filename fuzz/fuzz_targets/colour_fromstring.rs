#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("colour_fromstring");

    // colour_fromstring takes &str, so we need valid UTF-8.
    if let Ok(s) = std::str::from_utf8(data) {
        // Returns a colour value or -1 on error. Either is fine.
        let _ = tmux_rs_new::colour::colour_fromstring(s);
    }
});
