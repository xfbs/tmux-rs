#![no_main]

mod sandbox;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    sandbox::enable("attributes");

    // Round-trip: parse attributes, format back to string, parse again.
    // Verifies idempotency of the parse/format cycle.
    if let Ok(s) = std::str::from_utf8(data) {
        tmux_rs_new::attributes::fuzz_attributes_round_trip(s);
    }
});
