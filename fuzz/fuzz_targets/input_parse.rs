#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("input_parse");

    // Feeds bytes through the terminal escape sequence state machine.
    // Writes to a screen/grid but does not execute shell commands or
    // perform any I/O. The input_ctx has a null window_pane.
    tmux_rs_new::input::fuzz_input_parse(data);
});
