use super::harness;

use std::time::Duration;

use harness::{PtyClient, TmuxTestHarness};

#[test]
fn send_keys_and_capture_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24", "--", "cat"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Send some text to cat
    tmux.send_keys(&["hello"]).assert_success();
    tmux.send_keys(&["Enter"]).assert_success();

    // Give cat time to echo back
    std::thread::sleep(Duration::from_millis(200));

    let content = tmux.capture_pane();
    assert!(
        content.contains("hello"),
        "pane should contain 'hello', got:\n{content}"
    );
}

#[test]
fn pane_ids_are_unique() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();
    tmux.cmd().args(["split-window"]).run().assert_success();

    let panes = tmux.cmd().args(["list-panes", "-F", "#{pane_id}"]).run();
    let output = panes.stdout_trimmed();
    let ids: Vec<&str> = output.lines().collect();
    assert_eq!(ids.len(), 3);

    // All IDs should be unique
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3, "pane IDs should be unique: {ids:?}");
}

#[test]
fn kill_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();

    let before = tmux.cmd().args(["list-panes"]).run().stdout_trimmed().lines().count();
    assert_eq!(before, 2);

    tmux.cmd().args(["kill-pane"]).run().assert_success();

    let after = tmux.cmd().args(["list-panes"]).run().stdout_trimmed().lines().count();
    assert_eq!(after, 1);
}

#[test]
fn select_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();

    // Get pane IDs
    let panes = tmux.cmd().args(["list-panes", "-F", "#{pane_id}"]).run();
    let output = panes.stdout_trimmed();
    let ids: Vec<&str> = output.lines().collect();

    // Select the first pane
    tmux.cmd()
        .args(["select-pane", "-t", ids[0]])
        .run()
        .assert_success();

    let active = tmux.query("#{pane_id}");
    assert_eq!(active, ids[0]);
}

#[test]
fn resize_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd().args(["split-window"]).run().assert_success();

    // Resize the active pane
    tmux.cmd()
        .args(["resize-pane", "-y", "5"])
        .run()
        .assert_success();

    let height = tmux.query("#{pane_height}");
    assert_eq!(height, "5");
}

#[test]
fn split_window_vertical_twice() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Split vertically (side-by-side panes)
    tmux.cmd()
        .args(["split-window", "-h"])
        .run()
        .assert_success();

    let panes = tmux.cmd().args(["list-panes"]).run().stdout_trimmed().lines().count();
    assert_eq!(panes, 2, "should have 2 panes after first vertical split");

    // Split the same pane vertically again
    tmux.cmd()
        .args(["split-window", "-h"])
        .run()
        .assert_success();

    let panes = tmux.cmd().args(["list-panes"]).run().stdout_trimmed().lines().count();
    assert_eq!(panes, 3, "should have 3 panes after second vertical split");
}

#[test]
fn split_window_vertical_twice_layout() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // First vertical split
    tmux.cmd()
        .args(["split-window", "-h"])
        .run()
        .assert_success();

    // After first split, active pane should be the new (right) one
    let active_after_first = tmux.query("#{pane_index}");
    let widths_first = tmux.cmd()
        .args(["list-panes", "-F", "#{pane_index}:#{pane_width}:#{pane_active}"])
        .run()
        .stdout_trimmed();
    eprintln!("After first split (active={}): {}", active_after_first, widths_first);

    // Second vertical split on the active (right) pane
    tmux.cmd()
        .args(["split-window", "-h"])
        .run()
        .assert_success();

    let active_after_second = tmux.query("#{pane_index}");
    let widths_second = tmux.cmd()
        .args(["list-panes", "-F", "#{pane_index}:#{pane_width}:#{pane_active}"])
        .run()
        .stdout_trimmed();
    eprintln!("After second split (active={}): {}", active_after_second, widths_second);

    // Parse pane widths
    let panes: Vec<(u32, u32, bool)> = widths_second.lines().map(|line| {
        let parts: Vec<&str> = line.split(':').collect();
        let idx: u32 = parts[0].parse().unwrap();
        let width: u32 = parts[1].parse().unwrap();
        let active = parts[2] == "1";
        (idx, width, active)
    }).collect();

    assert_eq!(panes.len(), 3, "should have 3 panes");

    // The first pane (left, original) should be the widest or same width
    // The active pane should NOT be the widest (it was just split)
    let active_pane = panes.iter().find(|p| p.2).unwrap();
    let first_pane = &panes[0];

    eprintln!("First pane: idx={} width={}", first_pane.0, first_pane.1);
    eprintln!("Active pane: idx={} width={}", active_pane.0, active_pane.1);

    // The original left pane should still be ~39-40 wide
    // The split panes should each be ~19-20 wide
    assert!(
        first_pane.1 > active_pane.1,
        "first pane (width {}) should be wider than active split pane (width {})",
        first_pane.1, active_pane.1
    );
}

#[test]
fn split_window_fullsize_vertical_twice() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // First full-size vertical split (matches: split-window -fh)
    tmux.cmd()
        .args(["split-window", "-fh"])
        .run()
        .assert_success();

    let widths_first = tmux.cmd()
        .args(["list-panes", "-F", "#{pane_index}:#{pane_width}:#{pane_active}"])
        .run()
        .stdout_trimmed();
    eprintln!("After first -fh split: {}", widths_first);

    // Second full-size vertical split
    tmux.cmd()
        .args(["split-window", "-fh"])
        .run()
        .assert_success();

    let widths_second = tmux.cmd()
        .args(["list-panes", "-F", "#{pane_index}:#{pane_width}:#{pane_active}"])
        .run()
        .stdout_trimmed();
    eprintln!("After second -fh split: {}", widths_second);

    let panes: Vec<(u32, u32, bool)> = widths_second.lines().map(|line| {
        let parts: Vec<&str> = line.split(':').collect();
        (parts[0].parse().unwrap(), parts[1].parse().unwrap(), parts[2] == "1")
    }).collect();

    assert_eq!(panes.len(), 3, "should have 3 panes");

    // With -f (full-size), each split takes half the WHOLE window.
    // First split: [40, 39]. Second: existing panes squeezed to 40 → [20, 19], new pane 39.
    // The active pane should be the newly created one (rightmost).
    let active = panes.iter().find(|p| p.2).unwrap();
    let widths: Vec<u32> = panes.iter().map(|p| p.1).collect();
    eprintln!("Active pane: idx={} width={}", active.0, active.1);

    // New pane (rightmost) gets ~half the window, existing panes share the other half
    assert_eq!(widths, vec![20, 19, 39], "full-size split layout should match C tmux");
}

#[test]
fn split_window_vertical_twice_attached() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Attach a PTY client to simulate interactive use
    let _client = PtyClient::attach(&tmux, 80, 24);
    std::thread::sleep(Duration::from_millis(200));

    // Split vertically (side-by-side panes)
    tmux.cmd()
        .args(["split-window", "-h"])
        .run()
        .assert_success();
    std::thread::sleep(Duration::from_millis(200));

    let panes = tmux.cmd().args(["list-panes"]).run().stdout_trimmed().lines().count();
    assert_eq!(panes, 2, "should have 2 panes after first vertical split");

    // Split the same pane vertically again
    tmux.cmd()
        .args(["split-window", "-h"])
        .run()
        .assert_success();
    std::thread::sleep(Duration::from_millis(200));

    let panes = tmux.cmd().args(["list-panes"]).run().stdout_trimmed().lines().count();
    assert_eq!(panes, 3, "should have 3 panes after second vertical split");
}

// Ported from regress/copy-mode-test-vi.sh (basic operations only)
#[test]
fn copy_mode_vi_basic() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "40", "-y", "10"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-window-option", "-g", "mode-keys", "vi"])
        .run()
        .assert_success();

    // Enter copy mode
    tmux.cmd().args(["copy-mode"]).run().assert_success();

    // Verify we're in copy mode
    let in_mode = tmux.query("#{pane_in_mode}");
    assert_eq!(in_mode, "1");

    // Exit copy mode
    tmux.send_keys(&["-X", "cancel"]).assert_success();

    let in_mode = tmux.query("#{pane_in_mode}");
    assert_eq!(in_mode, "0");
}
