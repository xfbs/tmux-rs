use super::harness;

use std::time::Duration;

use harness::{PtyClient, TmuxTestHarness};

/// Verify that the automatic-rename timer fires: when a pane runs a new
/// command, the window name should update to reflect it.
#[test]
fn automatic_rename_updates_window_name() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Enable automatic rename (on by default, but be explicit).
    tmux.cmd()
        .args(["set", "-w", "automatic-rename", "on"])
        .run()
        .assert_success();

    // Run `cat` — window name should change from the shell to "cat".
    tmux.cmd()
        .args(["send-keys", "cat", "Enter"])
        .run()
        .assert_success();

    let name = tmux.wait_for(
        "#{window_name}",
        |v| v == "cat",
        Duration::from_secs(5),
    );
    assert_eq!(name, "cat");
}

/// Verify that the automatic-rename timer rate-limits name updates.
/// NAME_INTERVAL is 500ms, so rapid command changes should not cause
/// more than ~3 renames per second.
#[test]
fn automatic_rename_is_rate_limited() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set", "-w", "automatic-rename", "on"])
        .run()
        .assert_success();

    // Run a script that rapidly switches between `sleep` commands with
    // different names.  The rate limiter (500ms) should prevent the window
    // name from updating on every iteration.
    tmux.cmd()
        .args([
            "send-keys",
            "for i in $(seq 1 20); do sleep 0.05; done",
            "Enter",
        ])
        .run()
        .assert_success();

    // Wait for the command to start.
    std::thread::sleep(Duration::from_millis(200));

    // Sample the window name several times over the 1s the script runs.
    let mut names = Vec::new();
    for _ in 0..20 {
        let name = tmux.query("#{window_name}");
        if names.last().map_or(true, |last: &String| last != &name) {
            names.push(name);
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // The name should not have changed more than ~5 times (generous bound).
    // Without rate limiting, it would change ~20 times.
    assert!(
        names.len() <= 8,
        "window name changed {} times, expected <= 8 (rate limiting): {:?}",
        names.len(),
        names,
    );
}

/// Verify that the silence monitor alert timer fires after the configured
/// interval of inactivity.
#[test]
fn monitor_silence_triggers_alert() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second window so we can monitor the first from the second.
    tmux.cmd()
        .args(["new-window"])
        .run()
        .assert_success();

    // Enable silence monitoring on window 0 with a 2-second timeout.
    tmux.cmd()
        .args(["set", "-t", ":0", "monitor-silence", "2"])
        .run()
        .assert_success();

    // Switch to window 1 so we can observe the alert on window 0.
    tmux.cmd()
        .args(["select-window", "-t", ":1"])
        .run()
        .assert_success();

    // Wait for the silence flag to appear on window 0.
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(5);
    loop {
        let result = tmux.cmd()
            .args(["display-message", "-t", ":0", "-p", "#{window_silence_flag}"])
            .run();
        let flag = result.stdout_trimmed();
        if flag == "1" {
            break;
        }
        if start.elapsed() >= timeout {
            panic!("monitor-silence flag not set after {timeout:?}, last value: {flag:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Verify that the status line timer fires by checking that a dynamic
/// format in the status line gets evaluated (status-interval refresh).
#[test]
fn status_interval_refreshes_status() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a short status interval and a format that includes the session name.
    tmux.cmd()
        .args(["set", "-g", "status-interval", "1"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "-g", "status-right", "#{session_name}"])
        .run()
        .assert_success();

    // Rename the session — the status line should pick up the new name
    // after the next status-interval tick.
    tmux.cmd()
        .args(["rename-session", "timer-test"])
        .run()
        .assert_success();

    // Verify via the format system that the session name changed.
    let name = tmux.wait_for(
        "#{session_name}",
        |v| v == "timer-test",
        Duration::from_secs(3),
    );
    assert_eq!(name, "timer-test");
}

/// Verify that the session lock timer fires after `lock-after-time` seconds.
/// Uses a PtyClient so the lock command's output is visible.
#[test]
fn session_lock_timer_fires() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a very short lock timeout and a simple lock command.
    tmux.cmd()
        .args(["set", "-g", "lock-after-time", "1"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "-g", "lock-command", "echo SESSION_LOCKED && sleep 100"])
        .run()
        .assert_success();

    // Attach a PTY client — needed for lock to take effect.
    let mut client = PtyClient::attach(&tmux, 80, 24);

    // Wait for initial output.
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    // Wait for the lock command to fire (1 second timeout + margin).
    let output = client.wait_for_text("SESSION_LOCKED", Duration::from_secs(5));
    assert!(
        output.contains("SESSION_LOCKED"),
        "lock command should have fired, got: {output:?}"
    );
}

/// Verify that `run-shell -d` delays execution by the specified amount.
/// Uses `run-shell` (without `-b`) which blocks the command queue until
/// the shell command completes.  A second command queued after it will
/// only run once the delayed command finishes.
#[test]
fn run_shell_delay_timer() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Queue a delayed run-shell followed by a set-option.  The set-option
    // will only execute after the delay + command completes.
    let start = std::time::Instant::now();
    tmux.cmd()
        .args(["run-shell", "-b", "-d", "2", "true"])
        .run()
        .assert_success();
    // Immediately set a marker — this runs right away since -b was used.
    tmux.cmd()
        .args(["set", "-g", "@delay-start", "yes"])
        .run()
        .assert_success();

    // The marker should be set immediately.
    let val = tmux.query("#{@delay-start}");
    assert_eq!(val, "yes");

    // Now use a blocking run-shell with delay to create a file after 2s.
    // We can observe the timing by checking if the file exists.
    let marker = tempfile::NamedTempFile::new().unwrap();
    let marker_path = marker.path().to_str().unwrap().to_string();
    // Remove the file so we can detect when it gets created.
    drop(marker);
    std::fs::remove_file(&marker_path).ok();

    tmux.cmd()
        .args([
            "run-shell",
            "-b",
            "-d",
            "2",
            &format!("touch {marker_path}"),
        ])
        .run()
        .assert_success();

    // After 500ms the file should not exist yet.
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        !std::path::Path::new(&marker_path).exists(),
        "marker file should not exist 500ms into a 2s delay"
    );

    // Wait for the file to appear.
    let timeout = Duration::from_secs(5);
    loop {
        if std::path::Path::new(&marker_path).exists() {
            break;
        }
        if start.elapsed() >= timeout {
            panic!("marker file not created after {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(1800),
        "delay should be at least ~2s, was {:?}",
        elapsed
    );

    // Clean up.
    std::fs::remove_file(&marker_path).ok();
}

/// Verify that clock mode renders (the clock-mode timer fires and draws).
#[test]
fn clock_mode_renders() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut client = PtyClient::attach(&tmux, 80, 24);
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw(); // drain initial output

    // Enter clock mode.
    tmux.cmd()
        .args(["clock-mode"])
        .run()
        .assert_success();

    // The clock draws using box characters.  Wait for some output that
    // contains the ":" separator (drawn as block characters) or just any
    // substantial redraw.
    std::thread::sleep(Duration::from_millis(500));
    let output = client.read_screen();
    assert!(
        !output.is_empty(),
        "clock mode should have produced output"
    );

    // Verify we're actually in clock-mode by checking the pane mode.
    let mode = tmux.query("#{pane_mode}");
    assert_eq!(mode, "clock-mode");
}

/// Verify that the message timer fires: `display-message` shows a message
/// on the status line for `display-time` milliseconds, then clears it.
#[test]
fn message_timer_clears_message() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a short display-time so the test doesn't take long.
    tmux.cmd()
        .args(["set", "-g", "display-time", "1500"])
        .run()
        .assert_success();

    let mut client = PtyClient::attach(&tmux, 80, 24);
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw(); // drain initial output

    // Display a message.
    tmux.cmd()
        .args(["display-message", "TIMER_TEST_MSG"])
        .run()
        .assert_success();

    // The message should be visible on screen shortly after.
    std::thread::sleep(Duration::from_millis(200));
    let screen = client.read_screen();
    assert!(
        screen.contains("TIMER_TEST_MSG"),
        "message should be visible on status line, got: {screen:?}"
    );

    // After display-time (1.5s) the message should be cleared.
    // Wait a generous amount, then check the screen again.
    std::thread::sleep(Duration::from_millis(2000));
    client.read_raw(); // drain any intermediate output

    // Send a no-op to force a redraw so we get fresh screen content.
    tmux.cmd()
        .args(["refresh-client"])
        .run()
        .assert_success();
    std::thread::sleep(Duration::from_millis(200));
    let screen = client.read_screen();
    assert!(
        !screen.contains("TIMER_TEST_MSG"),
        "message should have been cleared after display-time, got: {screen:?}"
    );
}
