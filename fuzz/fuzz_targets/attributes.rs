#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("attributes");

    if let Ok(s) = std::str::from_utf8(data) {
        tmux_rs_new::attributes::fuzz_attributes(s);
    }
});
