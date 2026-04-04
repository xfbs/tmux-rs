#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("format_expand");

    // Expands a format string (#{...}) with FORMAT_NOJOBS and null context.
    // Cannot execute shell commands. Landlock sandbox provides additional
    // protection against unexpected code paths.
    tmux_rs_new::format::fuzz_format_expand(data);
});
