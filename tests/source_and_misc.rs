mod harness;

use std::io::Write;
use std::time::Duration;

use harness::TmuxTestHarness;

// ---------------------------------------------------------------------------
// source-file
// ---------------------------------------------------------------------------

#[test]
fn source_file_sets_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Write a config snippet that sets a user option
    let mut tmpfile = tempfile::NamedTempFile::new().expect("failed to create temp file");
    writeln!(tmpfile, "set -g @sourced_var sourced_ok").unwrap();

    tmux.cmd()
        .args(["source-file", &tmpfile.path().display().to_string()])
        .run()
        .assert_success();

    let val = tmux.query("#{@sourced_var}");
    assert_eq!(val, "sourced_ok");
}

#[test]
fn source_file_multiple_commands() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut tmpfile = tempfile::NamedTempFile::new().expect("failed to create temp file");
    writeln!(tmpfile, "set -g @multi_a first").unwrap();
    writeln!(tmpfile, "set -g @multi_b second").unwrap();

    tmux.cmd()
        .args(["source-file", &tmpfile.path().display().to_string()])
        .run()
        .assert_success();

    assert_eq!(tmux.query("#{@multi_a}"), "first");
    assert_eq!(tmux.query("#{@multi_b}"), "second");
}

#[test]
fn source_file_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux
        .cmd()
        .args(["source-file", "/tmp/tmux_rs_nonexistent_config_file_12345.conf"])
        .run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// pipe-pane
// ---------------------------------------------------------------------------

#[test]
fn pipe_pane_captures_output() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let outfile = tempfile::NamedTempFile::new().expect("failed to create temp file");
    let outpath = outfile.path().display().to_string();

    // Pipe pane output to cat, writing to our temp file
    tmux.cmd()
        .args(["pipe-pane", &format!("cat > {outpath}")])
        .run()
        .assert_success();

    // Generate some output in the pane
    tmux.send_keys(&["echo pipe_test_marker", "Enter"])
        .assert_success();

    // Give pipe time to flush
    std::thread::sleep(Duration::from_millis(1000));

    // Turn off pipe-pane to flush
    tmux.cmd().args(["pipe-pane"]).run().assert_success();

    std::thread::sleep(Duration::from_millis(500));

    let content = std::fs::read_to_string(&outpath).unwrap_or_default();
    assert!(
        content.contains("pipe_test_marker"),
        "pipe output should contain marker, got:\n{content}"
    );
}

// ---------------------------------------------------------------------------
// display-panes
// ---------------------------------------------------------------------------

#[test]
#[ignore = "display-panes requires an attached client"]
fn display_panes_smoke_test() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // display-panes in detached mode should not error
    let result = tmux.cmd().args(["display-panes"]).run();
    result.assert_success();
}

#[test]
#[ignore = "display-panes requires an attached client"]
fn display_panes_with_duration() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // display-panes with a short duration
    let result = tmux.cmd().args(["display-panes", "-d", "100"]).run();
    result.assert_success();
}

// ---------------------------------------------------------------------------
// respawn-pane
// ---------------------------------------------------------------------------

#[test]
fn respawn_pane_after_exit() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set remain-on-exit so pane stays around after process dies
    tmux.cmd()
        .args(["set-option", "-g", "remain-on-exit", "on"])
        .run()
        .assert_success();

    // Send exit to the shell
    tmux.send_keys(&["exit", "Enter"]).assert_success();

    // Wait for the shell to exit
    let start = std::time::Instant::now();
    loop {
        let dead = tmux.query("#{pane_dead}");
        if dead == "1" {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(5) {
            panic!("pane did not die within timeout");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Respawn the pane with a new command
    tmux.cmd()
        .args(["respawn-pane", "cat"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(300));

    let dead = tmux.query("#{pane_dead}");
    assert_eq!(dead, "0", "pane should be alive after respawn");

    let cmd = tmux.query("#{pane_current_command}");
    assert_eq!(cmd, "cat");
}

#[test]
fn respawn_pane_kill_flag() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24", "--", "sleep", "300"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Pane is alive (running sleep)
    let dead = tmux.query("#{pane_dead}");
    assert_eq!(dead, "0");

    // respawn-pane -k kills the existing process and starts a new one
    tmux.cmd()
        .args(["respawn-pane", "-k", "cat"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(300));

    let dead = tmux.query("#{pane_dead}");
    assert_eq!(dead, "0", "pane should be alive after respawn -k");

    let cmd = tmux.query("#{pane_current_command}");
    assert_eq!(cmd, "cat");
}

// ---------------------------------------------------------------------------
// attach-session
// ---------------------------------------------------------------------------

#[test]
fn attach_session_nonexistent_session_fails() {
    let tmux = TmuxTestHarness::new();

    // No server running at all -- attach should fail
    let result = tmux
        .cmd()
        .args(["attach-session", "-t", "nonexistent_session_xyz"])
        .run();
    result.assert_failure();
}

#[test]
fn attach_session_no_sessions_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["-s", "temp"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Kill all sessions
    tmux.cmd().args(["kill-session", "-t", "temp"]).run().assert_success();

    // Now attach should fail -- no sessions remain
    let result = tmux.cmd().args(["attach-session"]).run();
    result.assert_failure();
}

#[test]
fn attach_session_wrong_target_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["-s", "real"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Attaching to a nonexistent session should fail
    let result = tmux
        .cmd()
        .args(["attach-session", "-t", "does_not_exist"])
        .run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// wait-for
// ---------------------------------------------------------------------------

#[test]
fn wait_for_signal() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Signal a channel (lock it first, then unlock)
    // -L locks, -U unlocks
    tmux.cmd()
        .args(["wait-for", "-L", "test_channel"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["wait-for", "-U", "test_channel"])
        .run()
        .assert_success();
}

#[test]
fn wait_for_signal_and_wake() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Use -S to signal a named channel.
    // When no one is waiting, this should still succeed without blocking.
    let result = tmux
        .cmd()
        .args(["wait-for", "-S", "my_signal"])
        .run();
    result.assert_success();
}

#[test]
fn wait_for_lock_unlock_cycle() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Lock, then unlock -- should not deadlock since we do it sequentially
    // from outside the tmux session
    tmux.cmd()
        .args(["wait-for", "-L", "lock_ch"])
        .run()
        .assert_success();

    // Set a user option while holding the lock to prove we still have control
    tmux.cmd()
        .args(["set", "-g", "@lock_held", "yes"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["wait-for", "-U", "lock_ch"])
        .run()
        .assert_success();

    let val = tmux.query("#{@lock_held}");
    assert_eq!(val, "yes");
}
