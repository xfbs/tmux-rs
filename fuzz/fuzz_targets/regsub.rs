#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

#[derive(arbitrary::Arbitrary, Debug)]
struct RegsubInput {
    pattern: Vec<u8>,
    replacement: Vec<u8>,
    text: Vec<u8>,
}

fuzz_target!(|input: RegsubInput| {
    sandbox::enable("regsub");

    // Limit input sizes to avoid POSIX regex OOM on pathological patterns.
    if input.pattern.len() > 32 || input.replacement.len() > 32 || input.text.len() > 32 {
        return;
    }

    tmux_rs_new::regsub::fuzz_regsub(&input.pattern, &input.replacement, &input.text);
});
