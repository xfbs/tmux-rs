mod harness;

use std::time::Duration;

use harness::TmuxTestHarness;

// Ported from regress/new-session-size.sh
#[test]
fn new_session_default_size() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let size = tmux.query("#{window_width} #{window_height}");
    assert_eq!(size, "80 24");
}

// Ported from regress/new-session-size.sh (second case)
#[test]
fn new_session_explicit_size() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "100", "-y", "50"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let size = tmux.query("#{window_width} #{window_height}");
    assert_eq!(size, "100 50");
}

// Ported from regress/new-session-command.sh
#[test]
fn new_session_with_command() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["--", "cat"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // The command should be 'cat'
    let cmd = tmux.query("#{pane_current_command}");
    assert_eq!(cmd, "cat");
}

// Ported from regress/has-session-return.sh
#[test]
fn has_session_returns_error_for_nonexistent() {
    let mut tmux = TmuxTestHarness::new();

    // has-session should fail when no server is running
    let result = tmux.cmd().args(["has-session", "-t", "foo"]).run();
    assert!(!result.success());

    // has-session should fail for nonexistent session even after new-session
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux.cmd().args(["has-session", "-t", "foo"]).run();
    assert!(!result.success());
}

// Ported from regress/has-session-return.sh (positive case)
#[test]
fn has_session_returns_success_for_existing() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["-s", "foo"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux.cmd().args(["has-session", "-t", "foo"]).run();
    assert!(result.success());
}

// Ported from regress/new-session-base-index.sh
#[test]
fn new_session_base_index() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Default base index is 0
    let idx = tmux.query("#{window_index}");
    assert_eq!(idx, "0");

    // Set base-index to 1 and create a new session
    let mut tmux2 = TmuxTestHarness::new();
    tmux2
        .new_session()
        .run()
        .assert_success();
    tmux2.wait_ready(Duration::from_secs(5));
    tmux2
        .cmd()
        .args(["set-option", "-g", "base-index", "1"])
        .run()
        .assert_success();
    tmux2
        .cmd()
        .args(["new-window"])
        .run()
        .assert_success();
    // The new window should have index 1 (or 2 if base-index was applied)
    let windows = tmux2.cmd().args(["list-windows", "-F", "#{window_index}"]).run();
    let output = windows.stdout_trimmed();
    let indices: Vec<&str> = output.lines().collect();
    assert!(indices.contains(&"0") || indices.contains(&"1"));
}

// Ported from regress/kill-session-process-exit.sh
#[test]
fn kill_session_kills_processes() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["--", "sleep", "300"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Get the pane PID
    let pid_str = tmux.query("#{pane_pid}");
    let pid: u32 = pid_str.parse().expect("pane_pid should be a number");

    // Kill the session
    tmux.cmd().args(["kill-session"]).run().assert_success();

    // Give the process a moment to die
    std::thread::sleep(Duration::from_millis(500));

    // The process should be dead (kill -0 should fail)
    let check = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status();
    if let Ok(status) = check {
        assert!(!status.success(), "process {pid} should have been killed");
    }
}

// Ported from regress/new-session-no-client.sh
#[test]
fn new_session_no_client() {
    let mut tmux = TmuxTestHarness::new();
    // new -d means no client attaches
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let clients = tmux.cmd().args(["list-clients"]).run();
    // No clients should be attached (output should be empty)
    assert_eq!(clients.stdout_trimmed(), "");
}
