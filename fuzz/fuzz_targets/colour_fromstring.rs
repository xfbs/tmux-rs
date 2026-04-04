#![no_main]

use libfuzzer_sys::fuzz_target;
use tmux_rs_new::colour::colour_fromstring;

fuzz_target!(|data: &[u8]| {
    // colour_fromstring takes &str, so we need valid UTF-8.
    if let Ok(s) = std::str::from_utf8(data) {
        // Returns a colour value or -1 on error. Either is fine.
        let _ = colour_fromstring(s);
    }
});
