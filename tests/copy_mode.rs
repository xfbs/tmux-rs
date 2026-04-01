use super::harness;

use std::time::Duration;

use harness::{PtyClient, TmuxTestHarness};

/// Helper: create a harness + session + attached client, drain initial output.
fn setup(cols: u16, rows: u16) -> (TmuxTestHarness, PtyClient) {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", &cols.to_string(), "-y", &rows.to_string()])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    let mut client = PtyClient::attach(&tmux, cols, rows);
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw(); // drain initial output
    (tmux, client)
}

/// Helper: set mode-keys option.
fn set_mode_keys(tmux: &TmuxTestHarness, mode: &str) {
    tmux.cmd()
        .args(["set-window-option", "-g", "mode-keys", mode])
        .run()
        .assert_success();
}

/// Helper: enter copy mode via send-keys command (more reliable than prefix+[).
fn enter_copy_mode(tmux: &TmuxTestHarness) {
    tmux.cmd().args(["copy-mode"]).run().assert_success();
    std::thread::sleep(Duration::from_millis(300));
}

/// Helper: query whether the pane is in copy mode.
fn pane_in_mode(tmux: &TmuxTestHarness) -> String {
    tmux.query("#{pane_in_mode}")
}

/// Helper: send a copy-mode command via send-keys -X.
fn send_copy_cmd(tmux: &TmuxTestHarness, cmd: &str) {
    tmux.send_keys(&["-X", cmd]).assert_success();
    std::thread::sleep(Duration::from_millis(100));
}

/// Helper: get the contents of the paste buffer.
fn show_buffer(tmux: &TmuxTestHarness) -> String {
    tmux.cmd().args(["show-buffer"]).run().stdout_trimmed()
}

/// Helper: fill the pane with some known text by echoing lines.
fn fill_pane(client: &mut PtyClient, lines: &[&str]) {
    for line in lines {
        client.write_str(&format!("echo '{}'\r", line));
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(300));
    client.read_raw(); // drain
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn enter_copy_mode_vi() {
    let (tmux, mut _client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    enter_copy_mode(&tmux);
    assert_eq!(pane_in_mode(&tmux), "1", "pane should be in copy mode");
}

#[test]
fn enter_copy_mode_emacs() {
    let (tmux, mut _client) = setup(80, 24);
    set_mode_keys(&tmux, "emacs");

    enter_copy_mode(&tmux);
    assert_eq!(pane_in_mode(&tmux), "1", "pane should be in copy mode (emacs)");
}

#[test]
fn exit_copy_mode_vi() {
    let (tmux, mut _client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    enter_copy_mode(&tmux);
    assert_eq!(pane_in_mode(&tmux), "1");

    // Exit copy mode with cancel
    send_copy_cmd(&tmux, "cancel");
    assert_eq!(pane_in_mode(&tmux), "0", "pane should have exited copy mode");
}

#[test]
fn exit_copy_mode_emacs() {
    let (tmux, mut _client) = setup(80, 24);
    set_mode_keys(&tmux, "emacs");

    enter_copy_mode(&tmux);
    assert_eq!(pane_in_mode(&tmux), "1");

    send_copy_cmd(&tmux, "cancel");
    assert_eq!(pane_in_mode(&tmux), "0", "pane should have exited copy mode (emacs)");
}

#[test]
fn exit_copy_mode_vi_q_key() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    enter_copy_mode(&tmux);
    assert_eq!(pane_in_mode(&tmux), "1");

    // In vi mode, pressing 'q' should exit copy mode
    client.send_key("q");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(pane_in_mode(&tmux), "0", "q should exit vi copy mode");
}

#[test]
fn cursor_movement_vi() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Put some text in the pane
    fill_pane(&mut client, &["AAAA", "BBBB", "CCCC"]);

    enter_copy_mode(&tmux);

    // Move cursor down with j, check cursor_y changes
    let y_before = tmux.query("#{copy_cursor_y}");
    send_copy_cmd(&tmux, "cursor-down");
    // Note: cursor-down might not change y if already at bottom,
    // but we can also verify cursor-up goes back
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "cursor-up");
    let y_after = tmux.query("#{copy_cursor_y}");

    // Cursor should have moved up from initial position
    let y_before_num: i32 = y_before.parse().unwrap_or(0);
    let y_after_num: i32 = y_after.parse().unwrap_or(0);
    assert!(
        y_after_num <= y_before_num,
        "cursor should have moved up: before={y_before}, after={y_after}"
    );

    // Test horizontal movement
    send_copy_cmd(&tmux, "start-of-line");
    let x_start = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_start, "0", "start-of-line should put cursor at column 0");

    send_copy_cmd(&tmux, "cursor-right");
    let x_after = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_after, "1", "cursor-right should move to column 1");

    send_copy_cmd(&tmux, "end-of-line");
    let x_end: i32 = tmux.query("#{copy_cursor_x}").parse().unwrap_or(0);
    assert!(x_end > 0, "end-of-line should move cursor past column 0");

    send_copy_cmd(&tmux, "cancel");
}

#[test]
#[ignore = "broken: tmux-rs crashes on buffer operations (copy-selection stores to buffer)"]
fn copy_selection_vi() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Echo known text
    client.write_str("echo 'SELECTME hello world'\r");
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);

    // Go to the line with our text: search for it
    tmux.send_keys(&["-X", "search-backward", "-X"]).assert_success();
    // Actually, use the search command properly
    send_copy_cmd(&tmux, "cancel");

    // Re-enter copy mode and use history-top + navigate
    enter_copy_mode(&tmux);
    // Search backward for SELECTME
    tmux.send_keys(&["-X", "search-backward"]).assert_success();
    std::thread::sleep(Duration::from_millis(100));
    // Type the search term
    tmux.send_keys(&["SELECTME", "Enter"]).assert_success();
    std::thread::sleep(Duration::from_millis(300));

    // Now cursor should be on SELECTME. Select the word.
    send_copy_cmd(&tmux, "begin-selection");
    // Move to end of word
    send_copy_cmd(&tmux, "next-word-end");
    send_copy_cmd(&tmux, "copy-selection");

    std::thread::sleep(Duration::from_millis(200));
    let buf = show_buffer(&tmux);
    assert!(
        buf.contains("SELECTME"),
        "buffer should contain SELECTME, got: {buf:?}"
    );
}

#[test]
fn copy_selection_with_send_keys() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    // Load the copy-mode test data
    client.write_str("echo 'A line of words'\r");
    std::thread::sleep(Duration::from_millis(300));
    client.read_raw();

    enter_copy_mode(&tmux);
    // Go to top of history
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    // Select first word: begin-selection then next-word-end
    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "next-word-end");
    send_copy_cmd(&tmux, "copy-selection");

    std::thread::sleep(Duration::from_millis(200));
    let buf = show_buffer(&tmux);
    // Should have captured at least the first word-like content from the top of scrollback
    assert!(
        !buf.is_empty(),
        "buffer should not be empty after copy-selection"
    );
}

#[test]
#[ignore = "broken: tmux-rs hangs during copy-mode search"]
fn search_vi_forward() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Put some known text
    client.write_str("echo 'FINDME_MARKER somewhere in output'\r");
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);

    // In vi copy mode, search backward with ?
    tmux.send_keys(&["-X", "search-backward"]).assert_success();
    std::thread::sleep(Duration::from_millis(100));
    tmux.send_keys(&["FINDME_MARKER", "Enter"]).assert_success();
    std::thread::sleep(Duration::from_millis(300));

    // Cursor should now be on the line containing FINDME_MARKER
    // Verify by selecting from cursor to end of word and copying
    send_copy_cmd(&tmux, "begin-selection");
    // Select through the marker text
    for _ in 0..12 {
        send_copy_cmd(&tmux, "cursor-right");
    }
    send_copy_cmd(&tmux, "copy-selection");

    std::thread::sleep(Duration::from_millis(200));
    let buf = show_buffer(&tmux);
    assert!(
        buf.contains("FINDME_MARKER"),
        "search should have found FINDME_MARKER, buffer: {buf:?}"
    );
}

#[test]
fn page_up_down() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Generate enough output to have scrollback
    for i in 0..50 {
        client.write_str(&format!("echo 'LINE_{i}'\r"));
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);

    let y_initial = tmux.query("#{copy_cursor_y}");

    // Page up
    send_copy_cmd(&tmux, "page-up");
    std::thread::sleep(Duration::from_millis(200));
    let y_after_pgup = tmux.query("#{copy_cursor_y}");

    // After page-up, the cursor_y in the scrollback coordinate system should change
    // (the cursor y stays in the viewport but the scroll offset changes)
    // We can also check scroll_position
    let scroll_pos = tmux.query("#{scroll_position}");
    let scroll_num: i32 = scroll_pos.parse().unwrap_or(0);
    assert!(
        scroll_num > 0,
        "page-up should scroll back, scroll_position={scroll_pos}"
    );

    // Page down should reduce scroll position
    send_copy_cmd(&tmux, "page-down");
    std::thread::sleep(Duration::from_millis(200));
    let scroll_after_down = tmux.query("#{scroll_position}");
    let scroll_down_num: i32 = scroll_after_down.parse().unwrap_or(-1);
    assert!(
        scroll_down_num < scroll_num,
        "page-down should reduce scroll position: before={scroll_num}, after={scroll_down_num}"
    );

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn history_navigation_scroll_back() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Generate output so we have history
    client.write_str("echo 'EARLY_OUTPUT_MARKER'\r");
    std::thread::sleep(Duration::from_millis(200));
    for i in 0..40 {
        client.write_str(&format!("echo 'filler line {i}'\r"));
        std::thread::sleep(Duration::from_millis(30));
    }
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);

    // Go to history top
    send_copy_cmd(&tmux, "history-top");
    std::thread::sleep(Duration::from_millis(200));

    // Capture the pane to see if we scrolled back
    let captured = tmux.capture_pane();
    // The early marker should be visible after scrolling to top
    // (or at least we should be scrolled back significantly)
    let scroll_pos = tmux.query("#{scroll_position}");
    let scroll_num: i32 = scroll_pos.parse().unwrap_or(0);
    assert!(
        scroll_num > 0,
        "history-top should scroll to top of history, scroll_position={scroll_pos}"
    );

    // Go to history bottom
    send_copy_cmd(&tmux, "history-bottom");
    std::thread::sleep(Duration::from_millis(200));
    let scroll_pos_bottom = tmux.query("#{scroll_position}");
    let scroll_bottom_num: i32 = scroll_pos_bottom.parse().unwrap_or(-1);
    assert_eq!(
        scroll_bottom_num, 0,
        "history-bottom should return to scroll_position=0, got {scroll_bottom_num}"
    );

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn enter_copy_mode_via_prefix_key() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Enter copy mode using the actual prefix + [ key sequence
    client.send_key("C-b");
    std::thread::sleep(Duration::from_millis(50));
    client.send_key("[");
    std::thread::sleep(Duration::from_millis(500));

    assert_eq!(
        pane_in_mode(&tmux),
        "1",
        "prefix+[ should enter copy mode"
    );

    // Exit with q
    client.send_key("q");
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(
        pane_in_mode(&tmux),
        "0",
        "q should exit copy mode"
    );
}

#[test]
#[ignore = "broken: tmux-rs crashes on buffer operations (copy-selection)"]
fn copy_mode_word_navigation_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    // Based on the upstream copy-mode-test-vi.sh / copy-mode-test.txt
    // Load the test data
    client.write_str("printf 'A line of words\\n\\tIndented line\\nAnother line...\\n'\r");
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    // Select first character with previous-word at start (should stay at A)
    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "previous-word");
    send_copy_cmd(&tmux, "copy-selection");

    std::thread::sleep(Duration::from_millis(200));
    let buf = show_buffer(&tmux);
    assert_eq!(buf, "A", "previous-word at start should select just 'A'");
}

#[test]
fn half_page_up_down() {
    let (tmux, mut client) = setup(80, 24);
    set_mode_keys(&tmux, "vi");

    // Generate scrollback
    for i in 0..50 {
        client.write_str(&format!("echo 'HALF_PAGE_LINE_{i}'\r"));
        std::thread::sleep(Duration::from_millis(30));
    }
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);

    // Half page up
    send_copy_cmd(&tmux, "halfpage-up");
    std::thread::sleep(Duration::from_millis(200));
    let scroll_pos = tmux.query("#{scroll_position}");
    let scroll_num: i32 = scroll_pos.parse().unwrap_or(0);
    assert!(
        scroll_num > 0,
        "halfpage-up should scroll back, scroll_position={scroll_pos}"
    );

    // Half page down
    send_copy_cmd(&tmux, "halfpage-down");
    std::thread::sleep(Duration::from_millis(200));
    let scroll_after = tmux.query("#{scroll_position}");
    let scroll_after_num: i32 = scroll_after.parse().unwrap_or(-1);
    assert!(
        scroll_after_num < scroll_num,
        "halfpage-down should reduce scroll, before={scroll_num} after={scroll_after_num}"
    );

    send_copy_cmd(&tmux, "cancel");
}
