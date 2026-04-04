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

    // Regex substitution: pattern + replacement + text → result string.
    // Pure computation using POSIX regcomp/regexec. No side effects.
    tmux_rs_new::regsub::fuzz_regsub(&input.pattern, &input.replacement, &input.text);
});
