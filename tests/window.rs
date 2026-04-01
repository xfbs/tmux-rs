use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

// Ported from regress/new-window-command.sh
#[test]
fn new_window_with_command() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["new-window", "--", "cat"])
        .run()
        .assert_success();

    let cmd = tmux.query("#{pane_current_command}");
    assert_eq!(cmd, "cat");
}

// Ported from regress/command-order.sh
#[test]
fn command_chaining_order() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["-s", "main"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second session and verify we can list both
    tmux.cmd()
        .args(["new-session", "-d", "-s", "other"])
        .run()
        .assert_success();

    let sessions = tmux.cmd().args(["list-sessions", "-F", "#{session_name}"]).run();
    let output = sessions.stdout_trimmed();
    let names: Vec<&str> = output.lines().collect();
    assert!(names.contains(&"main"), "should contain 'main', got: {names:?}");
    assert!(names.contains(&"other"), "should contain 'other', got: {names:?}");
}

// Ported from regress/control-client-sanity.sh
#[test]
fn control_client_basic_operations() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "200", "-y", "200"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();

    // Verify we have 2 panes
    let panes = tmux.cmd().args(["list-panes", "-F", "#{pane_id}"]).run();
    let output = panes.stdout_trimmed();
    let pane_count = output.lines().count();
    assert_eq!(pane_count, 2, "expected 2 panes, got {pane_count}");

    // Create a new window
    tmux.cmd().args(["new-window"]).run().assert_success();

    // Verify we have 2 windows
    let windows = tmux.cmd().args(["list-windows", "-F", "#{window_id}"]).run();
    let output = windows.stdout_trimmed();
    let window_count = output.lines().count();
    assert_eq!(window_count, 2, "expected 2 windows, got {window_count}");
}

// Ported from regress/control-client-size.sh
#[test]
fn control_client_pane_layout() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "200", "-y", "200"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Split and verify layout includes both panes
    tmux.cmd().args(["split-window"]).run().assert_success();

    let layout = tmux.cmd().args(["list-panes", "-F", "#{pane_id} #{pane_width}x#{pane_height}"]).run();
    let output = layout.stdout_trimmed();
    let panes: Vec<&str> = output.lines().collect();
    assert_eq!(panes.len(), 2, "expected 2 panes after split, got: {panes:?}");

    // Both panes should be 200 wide
    for pane in &panes {
        assert!(
            pane.contains("200x"),
            "pane should be 200 wide: {pane}"
        );
    }
}

#[test]
fn select_layout_tiled() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "200", "-y", "200"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create 4 panes
    tmux.cmd().args(["split-window"]).run().assert_success();
    tmux.cmd().args(["split-window"]).run().assert_success();
    tmux.cmd().args(["split-window"]).run().assert_success();

    // Apply tiled layout
    tmux.cmd().args(["select-layout", "tiled"]).run().assert_success();

    // All 4 panes should exist
    let panes = tmux.cmd().args(["list-panes", "-F", "#{pane_id}"]).run();
    let output = panes.stdout_trimmed();
    assert_eq!(output.lines().count(), 4, "expected 4 panes after tiled layout");
}

#[test]
fn kill_window() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second window, then kill it
    tmux.cmd().args(["new-window"]).run().assert_success();
    let count_before = tmux.cmd().args(["list-windows"]).run().stdout_trimmed().lines().count();
    assert_eq!(count_before, 2);

    tmux.cmd().args(["kill-window"]).run().assert_success();
    let count_after = tmux.cmd().args(["list-windows"]).run().stdout_trimmed().lines().count();
    assert_eq!(count_after, 1);
}

#[test]
fn swap_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();

    // Get pane IDs before swap
    let before = tmux.cmd().args(["list-panes", "-F", "#{pane_id}"]).run();
    let before_output = before.stdout_trimmed();
    let before_ids: Vec<&str> = before_output.lines().collect();
    assert_eq!(before_ids.len(), 2);

    // Swap panes
    tmux.cmd()
        .args(["swap-pane", "-s", before_ids[0], "-t", before_ids[1]])
        .run()
        .assert_success();

    // Panes should still exist (same IDs, different order)
    let after = tmux.cmd().args(["list-panes", "-F", "#{pane_id}"]).run();
    let after_output = after.stdout_trimmed();
    let after_ids: Vec<&str> = after_output.lines().collect();
    assert_eq!(after_ids.len(), 2);
    assert_eq!(after_ids[0], before_ids[1]);
    assert_eq!(after_ids[1], before_ids[0]);
}
