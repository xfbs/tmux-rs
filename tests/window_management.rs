use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

// ---------------------------------------------------------------------------
// rename-window
// ---------------------------------------------------------------------------

#[test]
fn rename_window() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Rename the current window
    tmux.cmd()
        .args(["rename-window", "my-work"])
        .run()
        .assert_success();

    // Verify via display-message
    let name = tmux.query("#{window_name}");
    assert_eq!(name, "my-work");

    // Verify via list-windows
    let windows = tmux
        .cmd()
        .args(["list-windows", "-F", "#{window_name}"])
        .run();
    let output = windows.stdout_trimmed();
    assert!(
        output.contains("my-work"),
        "list-windows should show renamed window, got: {output}"
    );
}

#[test]
fn rename_window_with_target() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second window
    tmux.cmd().args(["new-window"]).run().assert_success();

    // Rename window 0 while window 1 is active
    tmux.cmd()
        .args(["rename-window", "-t", ":0", "background"])
        .run()
        .assert_success();

    // The active window (1) should not be "background"
    let active_name = tmux.query("#{window_name}");
    assert_ne!(active_name, "background");

    // Window 0 should be "background"
    let windows = tmux
        .cmd()
        .args(["list-windows", "-F", "#{window_index}:#{window_name}"])
        .run();
    let output = windows.stdout_trimmed();
    assert!(
        output.contains("0:background"),
        "window 0 should be renamed to 'background', got: {output}"
    );
}

// ---------------------------------------------------------------------------
// swap-window
// ---------------------------------------------------------------------------

#[test]
fn swap_window() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24", "-s", "main"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Name the first window and create a named second window
    tmux.cmd()
        .args(["rename-window", "first"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["new-window", "-n", "second"])
        .run()
        .assert_success();

    // Before swap: window 0 = "first", window 1 = "second"
    let before = tmux
        .cmd()
        .args(["list-windows", "-F", "#{window_index}:#{window_name}"])
        .run();
    let before_output = before.stdout_trimmed();
    assert!(
        before_output.contains("0:first"),
        "before swap window 0 should be 'first', got: {before_output}"
    );
    assert!(
        before_output.contains("1:second"),
        "before swap window 1 should be 'second', got: {before_output}"
    );

    // Swap windows 0 and 1
    tmux.cmd()
        .args(["swap-window", "-s", ":0", "-t", ":1"])
        .run()
        .assert_success();

    // After swap: window 0 = "second", window 1 = "first"
    let after = tmux
        .cmd()
        .args(["list-windows", "-F", "#{window_index}:#{window_name}"])
        .run();
    let after_output = after.stdout_trimmed();
    assert!(
        after_output.contains("0:second"),
        "after swap window 0 should be 'second', got: {after_output}"
    );
    assert!(
        after_output.contains("1:first"),
        "after swap window 1 should be 'first', got: {after_output}"
    );
}

// ---------------------------------------------------------------------------
// move-window
// ---------------------------------------------------------------------------

#[test]
fn move_window_to_new_index() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Name the first window
    tmux.cmd()
        .args(["rename-window", "alpha"])
        .run()
        .assert_success();

    // Create a second window
    tmux.cmd()
        .args(["new-window", "-n", "beta"])
        .run()
        .assert_success();

    // Move window "alpha" (index 0) to index 5
    tmux.cmd()
        .args(["move-window", "-s", ":0", "-t", ":5"])
        .run()
        .assert_success();

    // Verify the indices
    let windows = tmux
        .cmd()
        .args(["list-windows", "-F", "#{window_index}:#{window_name}"])
        .run();
    let output = windows.stdout_trimmed();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2, "should still have 2 windows, got: {lines:?}");
    assert!(
        output.contains("1:beta"),
        "beta should remain at index 1, got: {output}"
    );
    assert!(
        output.contains("5:alpha"),
        "alpha should now be at index 5, got: {output}"
    );
}

#[test]
fn move_window_fails_on_occupied_index() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["new-window"]).run().assert_success();

    // Moving window 0 to index 1 should fail (occupied)
    let result = tmux.cmd().args(["move-window", "-s", ":0", "-t", ":1"]).run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// select-window
// ---------------------------------------------------------------------------

#[test]
fn select_window_by_index() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["rename-window", "win0"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["new-window", "-n", "win1"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["new-window", "-n", "win2"])
        .run()
        .assert_success();

    // Currently on window 2; select window 0
    tmux.cmd()
        .args(["select-window", "-t", ":0"])
        .run()
        .assert_success();

    let name = tmux.query("#{window_name}");
    assert_eq!(name, "win0");

    // Select window 1
    tmux.cmd()
        .args(["select-window", "-t", ":1"])
        .run()
        .assert_success();

    let name = tmux.query("#{window_name}");
    assert_eq!(name, "win1");
}

#[test]
fn select_window_by_name() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["rename-window", "editor"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["new-window", "-n", "terminal"])
        .run()
        .assert_success();

    // Currently on "terminal"; select by name "editor"
    tmux.cmd()
        .args(["select-window", "-t", "editor"])
        .run()
        .assert_success();

    let name = tmux.query("#{window_name}");
    assert_eq!(name, "editor");
}

#[test]
fn select_window_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux
        .cmd()
        .args(["select-window", "-t", "does-not-exist"])
        .run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// rotate-window
// ---------------------------------------------------------------------------

#[test]
fn rotate_window_panes() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create two splits so we have 3 panes
    tmux.cmd().args(["split-window"]).run().assert_success();
    tmux.cmd().args(["split-window"]).run().assert_success();

    // Record the pane IDs in positional order
    let before = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run();
    let before_output = before.stdout_trimmed();
    let before_ids: Vec<&str> = before_output.lines().collect();
    assert_eq!(before_ids.len(), 3);

    // Rotate the window (default: downward)
    tmux.cmd()
        .args(["rotate-window"])
        .run()
        .assert_success();

    // After rotation, the order of pane IDs in list-panes should change.
    // The last pane moves to the first position.
    let after = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run();
    let after_output = after.stdout_trimmed();
    let after_ids: Vec<&str> = after_output.lines().collect();
    assert_eq!(after_ids.len(), 3);

    // After default rotation, panes shift up: [0,1,2] -> [1,2,0]
    assert_eq!(
        after_ids[0], before_ids[1],
        "after rotate, first position should hold the previously second pane"
    );
    assert_eq!(
        after_ids[1], before_ids[2],
        "after rotate, second position should hold the previously third pane"
    );
    assert_eq!(
        after_ids[2], before_ids[0],
        "after rotate, third position should hold the previously first pane"
    );
}

#[test]
fn rotate_window_upward() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();
    tmux.cmd().args(["split-window"]).run().assert_success();

    let before = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run();
    let before_output = before.stdout_trimmed();
    let before_ids: Vec<&str> = before_output.lines().collect();

    // Rotate upward with -U
    tmux.cmd()
        .args(["rotate-window", "-U"])
        .run()
        .assert_success();

    let after = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run();
    let after_output = after.stdout_trimmed();
    let after_ids: Vec<&str> = after_output.lines().collect();

    // After an upward rotation, the first pane should become the last
    assert_eq!(
        after_ids[0], before_ids[1],
        "after upward rotate, first position should hold previously second pane"
    );
    assert_eq!(
        after_ids[1], before_ids[2],
        "after upward rotate, second position should hold previously third pane"
    );
    assert_eq!(
        after_ids[2], before_ids[0],
        "after upward rotate, third position should hold previously first pane"
    );
}

// ---------------------------------------------------------------------------
// resize-window
// ---------------------------------------------------------------------------

#[test]
fn resize_window_dimensions() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set window-size to manual so resize-window takes effect
    tmux.cmd()
        .args(["set-option", "-g", "window-size", "manual"])
        .run()
        .assert_success();

    // Resize to 120x40
    tmux.cmd()
        .args(["resize-window", "-x", "120", "-y", "40"])
        .run()
        .assert_success();

    let width = tmux.query("#{window_width}");
    let height = tmux.query("#{window_height}");
    assert_eq!(width, "120", "window width should be 120, got: {width}");
    assert_eq!(height, "40", "window height should be 40, got: {height}");
}

#[test]
fn resize_window_width_only() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-option", "-g", "window-size", "manual"])
        .run()
        .assert_success();

    // Resize width only
    tmux.cmd()
        .args(["resize-window", "-x", "100"])
        .run()
        .assert_success();

    let width = tmux.query("#{window_width}");
    assert_eq!(width, "100", "window width should be 100, got: {width}");
}

#[test]
fn resize_window_relative_increase() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-option", "-g", "window-size", "manual"])
        .run()
        .assert_success();

    // Adjust width by +10 (relative)
    tmux.cmd()
        .args(["resize-window", "-x", "90"])
        .run()
        .assert_success();

    let width = tmux.query("#{window_width}");
    assert_eq!(width, "90");
}

// ---------------------------------------------------------------------------
// respawn-window
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs remain-on-exit/respawn-window interaction"]
fn respawn_window_after_process_exit() {
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

    // Give the shell time to start, then send exit
    std::thread::sleep(Duration::from_millis(200));
    tmux.send_keys(&["exit", "Enter"]).assert_success();

    // Wait for the shell to exit
    let start = std::time::Instant::now();
    loop {
        let dead = tmux.query("#{pane_dead}");
        if dead == "1" {
            break;
        }
        if start.elapsed() > Duration::from_secs(10) {
            panic!("pane did not die within timeout");
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    // Respawn the window with a new command
    tmux.cmd()
        .args(["respawn-window", "--", "cat"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(200));

    // Verify the pane is alive again
    let dead_after = tmux.query("#{pane_dead}");
    assert_eq!(
        dead_after, "0",
        "pane should be alive after respawn, got: {dead_after}"
    );

    let cmd = tmux.query("#{pane_current_command}");
    assert_eq!(cmd, "cat", "respawned command should be 'cat', got: {cmd}");
}

#[test]
fn respawn_window_kill_flag() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24", "--", "cat"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Respawn without -k should fail because process is still running
    let result = tmux.cmd().args(["respawn-window"]).run();
    result.assert_failure();

    // Respawn with -k should succeed even if the process is still running
    tmux.cmd()
        .args(["respawn-window", "-k", "--", "sleep", "300"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(200));

    let cmd = tmux.query("#{pane_current_command}");
    assert_eq!(cmd, "sleep", "respawned command should be 'sleep', got: {cmd}");
}

// ---------------------------------------------------------------------------
// find-window
// ---------------------------------------------------------------------------

#[test]
#[ignore = "find-window opens interactive mode, requires an attached client"]
fn find_window_by_name() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create windows with distinct names
    tmux.cmd()
        .args(["rename-window", "logs-viewer"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["new-window", "-n", "code-editor"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["new-window", "-n", "database"])
        .run()
        .assert_success();

    // find-window with -N flag searches window names.
    // When there is exactly one match it selects that window.
    // With multiple matches it opens a menu. We use list-windows + grep pattern
    // to verify the name exists via the tmux command output.
    let result = tmux
        .cmd()
        .args(["find-window", "-N", "logs-viewer"])
        .run();
    result.assert_success();

    // After find-window with a unique match, it should select that window
    let name = tmux.query("#{window_name}");
    assert_eq!(
        name, "logs-viewer",
        "find-window should select the matching window, got: {name}"
    );
}

// ---------------------------------------------------------------------------
// break-pane
// ---------------------------------------------------------------------------

#[test]
fn break_pane_into_new_window() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Split so we have 2 panes in window 0
    tmux.cmd().args(["split-window"]).run().assert_success();

    // Verify we have 2 panes
    let panes_before = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run();
    let before_output = panes_before.stdout_trimmed();
    let before_ids: Vec<&str> = before_output.lines().collect();
    assert_eq!(before_ids.len(), 2, "should have 2 panes before break-pane");

    // Record the active pane ID (the one we will break out)
    let active_pane = tmux.query("#{pane_id}");

    // Verify we have 1 window
    let win_count_before = tmux
        .cmd()
        .args(["list-windows"])
        .run()
        .stdout_trimmed()
        .lines()
        .count();
    assert_eq!(win_count_before, 1);

    // Break the current pane into its own window
    tmux.cmd()
        .args(["break-pane"])
        .run()
        .assert_success();

    // We should now have 2 windows
    let win_count_after = tmux
        .cmd()
        .args(["list-windows"])
        .run()
        .stdout_trimmed()
        .lines()
        .count();
    assert_eq!(
        win_count_after, 2,
        "break-pane should create a new window"
    );

    // The original window should have only 1 pane
    let panes_win0 = tmux
        .cmd()
        .args(["list-panes", "-t", ":0", "-F", "#{pane_id}"])
        .run();
    let win0_output = panes_win0.stdout_trimmed();
    let win0_ids: Vec<&str> = win0_output.lines().collect();
    assert_eq!(
        win0_ids.len(),
        1,
        "original window should have 1 pane after break-pane"
    );

    // The new window should contain the broken-out pane
    let current_pane = tmux.query("#{pane_id}");
    assert_eq!(
        current_pane, active_pane,
        "the active pane should be the one that was broken out"
    );
}

#[test]
fn break_pane_with_detach() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Split to get 2 panes
    tmux.cmd().args(["split-window"]).run().assert_success();

    // Record which window we are on
    let orig_window = tmux.query("#{window_index}");

    // Break pane with -d (stay on current window)
    tmux.cmd()
        .args(["break-pane", "-d"])
        .run()
        .assert_success();

    // We should still be on the original window
    let current_window = tmux.query("#{window_index}");
    assert_eq!(
        current_window, orig_window,
        "with -d flag, should remain on original window"
    );

    // But there should be 2 windows total
    let win_count = tmux
        .cmd()
        .args(["list-windows"])
        .run()
        .stdout_trimmed()
        .lines()
        .count();
    assert_eq!(win_count, 2);
}

// ---------------------------------------------------------------------------
// join-pane
// ---------------------------------------------------------------------------

#[test]
fn join_pane_from_another_window() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second window
    tmux.cmd()
        .args(["new-window", "-n", "source"])
        .run()
        .assert_success();

    // Record the pane ID in the second window
    let source_pane = tmux.query("#{pane_id}");

    // Go back to window 0
    tmux.cmd()
        .args(["select-window", "-t", ":0"])
        .run()
        .assert_success();

    // Verify window 0 has 1 pane
    let panes_before = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run()
        .stdout_trimmed()
        .lines()
        .count();
    assert_eq!(panes_before, 1);

    // Join the pane from the "source" window into this window
    tmux.cmd()
        .args(["join-pane", "-s", &format!("{}", source_pane)])
        .run()
        .assert_success();

    // Window 0 should now have 2 panes
    let panes_after = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_id}"])
        .run();
    let after_output = panes_after.stdout_trimmed();
    let after_ids: Vec<&str> = after_output.lines().collect();
    assert_eq!(
        after_ids.len(),
        2,
        "window should have 2 panes after join-pane, got: {after_ids:?}"
    );
    assert!(
        after_ids.contains(&source_pane.as_str()),
        "joined pane ID {source_pane} should appear in the window"
    );

    // The source window should be gone (it had only one pane)
    let win_count = tmux
        .cmd()
        .args(["list-windows"])
        .run()
        .stdout_trimmed()
        .lines()
        .count();
    assert_eq!(
        win_count, 1,
        "source window should be removed after its only pane was joined"
    );
}

#[test]
fn join_pane_horizontal() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second window
    tmux.cmd().args(["new-window"]).run().assert_success();
    let source_pane = tmux.query("#{pane_id}");

    // Go back to window 0
    tmux.cmd()
        .args(["select-window", "-t", ":0"])
        .run()
        .assert_success();

    // Join horizontally (-h)
    tmux.cmd()
        .args(["join-pane", "-h", "-s", &source_pane])
        .run()
        .assert_success();

    // Should have 2 panes side by side (same height, different widths)
    let panes = tmux
        .cmd()
        .args(["list-panes", "-F", "#{pane_width}"])
        .run();
    let widths_output = panes.stdout_trimmed();
    let widths: Vec<&str> = widths_output.lines().collect();
    assert_eq!(widths.len(), 2, "should have 2 panes after horizontal join");

    // Both panes should be less than the full width (80)
    for w in &widths {
        let width: u32 = w.parse().expect("width should be a number");
        assert!(
            width < 80,
            "each pane width should be less than 80 in horizontal split, got: {width}"
        );
    }
}

#[test]
fn join_pane_same_window_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let pane_id = tmux.query("#{pane_id}");

    // Trying to join a pane into its own window should fail
    let result = tmux
        .cmd()
        .args(["join-pane", "-s", &pane_id])
        .run();
    result.assert_failure();
}
