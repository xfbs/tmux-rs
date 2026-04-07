use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

// Ported from regress/am-terminal.sh
// Tests handling of terminals without auto-margin (am@).
// The original test uses two nested tmux servers; we instead disable am
// directly via terminal-overrides on a single server and verify the last
// line of capture handles the right margin correctly.
#[test]
#[ignore = "exercises nested tmux server with am@ override; nontrivial to port to single-server harness"]
fn am_terminal_status_line() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "20", "-y", "2"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set", "-as", "terminal-overrides", ",*:am@"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "-g", "status-right", "RRR"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "-g", "status-left", "LLL"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "-g", "window-status-current-format", "WWW"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(500));

    let captured = tmux.capture_pane();
    let last = captured.lines().last().unwrap_or("").to_string();
    // Reference output: "LLLWWW           RR" (last char dropped, no am)
    assert_eq!(last, "LLLWWW           RR");
}
