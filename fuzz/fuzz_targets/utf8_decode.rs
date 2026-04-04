#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("utf8_decode");

    // Feeds bytes through the UTF-8 decoder state machine.
    // Pure computation, no side effects.
    tmux_rs_new::utf8::fuzz_utf8_decode(data);
});
