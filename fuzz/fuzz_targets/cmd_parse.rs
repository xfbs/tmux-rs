#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("cmd_parse");

    // cmd_parse_from_buffer handles arbitrary bytes — it lexes, parses, and
    // builds a cmd_list data structure, but does NOT execute any commands.
    // Landlock sandbox ensures no filesystem writes or command execution
    // even if format_expand reaches an unexpected code path.
    tmux_rs_new::cmd_parse::fuzz_cmd_parse(data);
});
