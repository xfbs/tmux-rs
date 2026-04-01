use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

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
