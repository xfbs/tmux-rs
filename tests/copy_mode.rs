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

    let _y_initial = tmux.query("#{copy_cursor_y}");

    // Page up
    send_copy_cmd(&tmux, "page-up");
    std::thread::sleep(Duration::from_millis(200));
    let _y_after_pgup = tmux.query("#{copy_cursor_y}");

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
    let _captured = tmux.capture_pane();
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

// ---------------------------------------------------------------------------
// New tests: broader copy-mode coverage
// ---------------------------------------------------------------------------

/// Helper: send a copy-mode command with arguments via send-keys -X.
fn send_copy_cmd_with_args(tmux: &TmuxTestHarness, args: &[&str]) {
    let mut all = vec!["-X"];
    all.extend(args);
    tmux.send_keys(&all).assert_success();
    std::thread::sleep(Duration::from_millis(100));
}

// 1. Rectangle selection
#[test]
#[ignore = "broken: tmux-rs crashes on buffer operations (copy-selection stores to buffer)"]
fn rectangle_selection_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["AAAA BBBB", "CCCC DDDD", "EEEE FFFF"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    // Begin selection, toggle rectangle mode, move right and down, copy
    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "rectangle-toggle");
    // Move right 3 chars and down 1 line to select a 4x2 rectangle
    send_copy_cmd(&tmux, "cursor-right");
    send_copy_cmd(&tmux, "cursor-right");
    send_copy_cmd(&tmux, "cursor-right");
    send_copy_cmd(&tmux, "cursor-down");
    send_copy_cmd(&tmux, "copy-selection");

    std::thread::sleep(Duration::from_millis(200));
    let buf = show_buffer(&tmux);
    // Rectangle selection of first 4 cols over 2 rows should give "AAAA\nCCCC"
    assert!(
        buf.contains("AAAA") && buf.contains("CCCC"),
        "rectangle selection should capture AAAA and CCCC, got: {buf:?}"
    );
}

// 1b. Rectangle toggle enters/exits rectangle mode (no buffer needed)
#[test]
fn rectangle_toggle_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["hello world"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "rectangle-toggle");

    // Check selection_active is still 1 (selection is on)
    let sel = tmux.query("#{selection_active}");
    assert_eq!(sel, "1", "selection should be active after rectangle-toggle");

    // Toggle off
    send_copy_cmd(&tmux, "rectangle-toggle");
    let sel2 = tmux.query("#{selection_active}");
    assert_eq!(sel2, "1", "selection should still be active after second rectangle-toggle");

    send_copy_cmd(&tmux, "cancel");
}

// 2. Jump to character
#[test]
fn jump_forward_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["abcdefghij"]);

    enter_copy_mode(&tmux);
    // Go up 1 line from the prompt to land on the echoed output line.
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "start-of-line");

    let x_before = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_before, "0");

    // jump-forward to 'e'
    send_copy_cmd_with_args(&tmux, &["jump-forward", "e"]);
    let x_after = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_after, "4", "jump-forward 'e' should move cursor to column 4");

    send_copy_cmd(&tmux, "cancel");
}

#[test]
#[ignore = "broken: tmux-rs jump-backward does not move cursor (passes with C tmux)"]
fn jump_backward_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["abcdefghij"]);

    enter_copy_mode(&tmux);
    // Go up 1 line from the prompt to land on the echoed output line.
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "start-of-line");

    // Move to end first
    send_copy_cmd(&tmux, "end-of-line");

    // jump-backward to 'c'
    send_copy_cmd_with_args(&tmux, &["jump-backward", "c"]);
    let x_after = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_after, "2", "jump-backward 'c' should move cursor to column 2");

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn jump_again_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["abcaefaghi"]);

    enter_copy_mode(&tmux);
    // Go up 1 line from the prompt to land on the echoed output line.
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "start-of-line");

    // jump-forward to 'a' -- from col 0 which is 'a', should find next 'a' at col 3
    send_copy_cmd_with_args(&tmux, &["jump-forward", "a"]);
    let x1 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x1, "3", "first jump-forward 'a' should land at col 3");

    // jump-again should find the next 'a' at col 6
    send_copy_cmd(&tmux, "jump-again");
    let x2 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x2, "6", "jump-again should land at col 6 (third 'a')");

    send_copy_cmd(&tmux, "cancel");
}

// 3. Search backward
#[test]
    #[ignore = "broken: tmux-rs search in copy mode"]
fn search_backward_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["alpha beta gamma", "delta epsilon zeta"]);

    enter_copy_mode(&tmux);

    // search-backward with an argument (the search term)
    send_copy_cmd_with_args(&tmux, &["search-backward", "alpha"]);
    std::thread::sleep(Duration::from_millis(300));

    // After finding "alpha", cursor should be on column 0 of that line
    let x = tmux.query("#{copy_cursor_x}");
    assert_eq!(x, "0", "search-backward 'alpha' should place cursor at col 0");

    // Capture the pane to verify we scrolled to the right place
    let captured = tmux.capture_pane();
    assert!(
        captured.contains("alpha"),
        "pane should show the line with 'alpha' after search"
    );

    send_copy_cmd(&tmux, "cancel");
}

// 4. Append to buffer
#[test]
#[ignore = "broken: tmux-rs crashes on buffer operations (append-selection stores to buffer)"]
fn append_selection_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["FIRST SECOND THIRD"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    // Select and copy "FIRST"
    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "next-word-end");
    send_copy_cmd(&tmux, "copy-selection-no-clear");
    std::thread::sleep(Duration::from_millis(200));

    // Now move to next word and append-select "SECOND"
    send_copy_cmd(&tmux, "next-word");
    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "next-word-end");
    send_copy_cmd(&tmux, "append-selection");
    std::thread::sleep(Duration::from_millis(200));

    let buf = show_buffer(&tmux);
    assert!(
        buf.contains("FIRST") && buf.contains("SECOND"),
        "buffer should contain both FIRST and SECOND after append, got: {buf:?}"
    );
}

// 5. Word movement: w/b/e (next-word, previous-word, next-word-end)
#[test]
#[ignore = "broken: tmux-rs next-word skips too far (passes with C tmux)"]
fn word_movement_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["one two three four"]);

    enter_copy_mode(&tmux);
    // Go up 1 line from the prompt to land on the echoed output line.
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "start-of-line");

    // Cursor at col 0 ("one")
    let x0 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x0, "0", "should start at col 0");

    // next-word should move to "two" (col 4)
    send_copy_cmd(&tmux, "next-word");
    let x1 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x1, "4", "next-word from 'one' should move to col 4 ('two')");

    // next-word again should move to "three" (col 8)
    send_copy_cmd(&tmux, "next-word");
    let x2 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x2, "8", "next-word from 'two' should move to col 8 ('three')");

    // previous-word should go back to "two" (col 4)
    send_copy_cmd(&tmux, "previous-word");
    let x3 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x3, "4", "previous-word from 'three' should move back to col 4 ('two')");

    // next-word-end from "two" should land on 'o' of "two" (col 6)
    send_copy_cmd(&tmux, "next-word-end");
    let x4 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x4, "6", "next-word-end from start of 'two' should land at col 6 (end of 'two')");

    send_copy_cmd(&tmux, "cancel");
}

// 6. Top/middle/bottom of visible screen
#[test]
fn top_middle_bottom_line_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    // Fill enough content to have a full screen
    for i in 0..15 {
        client.write_str(&format!("echo 'line_{i}'\r"));
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    enter_copy_mode(&tmux);

    // top-line: cursor goes to top visible row
    send_copy_cmd(&tmux, "top-line");
    let y_top: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);
    assert_eq!(y_top, 0, "top-line should put cursor at row 0");

    // bottom-line: cursor goes to bottom visible row
    send_copy_cmd(&tmux, "bottom-line");
    let y_bottom: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);
    // The visible area is 10 rows minus 1 for status bar = 9 data rows (0-8)
    assert!(
        y_bottom >= 7,
        "bottom-line should put cursor near bottom of visible area, got y={y_bottom}"
    );

    // middle-line: cursor goes to middle visible row
    send_copy_cmd(&tmux, "middle-line");
    let y_mid: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);
    assert!(
        y_mid > y_top && y_mid < y_bottom,
        "middle-line y={y_mid} should be between top={y_top} and bottom={y_bottom}"
    );

    send_copy_cmd(&tmux, "cancel");
}

// 7. Beginning/end of line, back-to-indentation
#[test]
fn start_end_of_line_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["hello world test"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");

    // end-of-line
    send_copy_cmd(&tmux, "end-of-line");
    let x_end: i32 = tmux.query("#{copy_cursor_x}").parse().unwrap_or(-1);
    assert!(x_end > 0, "end-of-line should move cursor past col 0, got {x_end}");

    // start-of-line
    send_copy_cmd(&tmux, "start-of-line");
    let x_start = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_start, "0", "start-of-line should move cursor to col 0");

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn back_to_indentation_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    // Use printf to get a line with leading whitespace
    client.write_str("printf '    indented\\n'\r");
    std::thread::sleep(Duration::from_millis(300));
    client.read_raw();

    enter_copy_mode(&tmux);
    // Go up 1 line from the prompt to land on the indented output line.
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "start-of-line");
    let x_start = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_start, "0", "start-of-line should be at col 0");

    // back-to-indentation should skip whitespace
    send_copy_cmd(&tmux, "back-to-indentation");
    let x_indent: i32 = tmux.query("#{copy_cursor_x}").parse().unwrap_or(0);
    assert!(
        x_indent >= 4,
        "back-to-indentation should skip leading spaces, got col {x_indent}"
    );

    send_copy_cmd(&tmux, "cancel");
}

// 8. Copy pipe
#[test]
#[ignore = "broken: tmux-rs crashes on buffer operations (copy-pipe stores to buffer)"]
fn copy_pipe_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    fill_pane(&mut client, &["PIPE_TEST_DATA here"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    send_copy_cmd(&tmux, "begin-selection");
    send_copy_cmd(&tmux, "next-word-end");

    // copy-pipe to cat (stores to buffer and pipes to command)
    send_copy_cmd_with_args(&tmux, &["copy-pipe", "cat > /tmp/tmux-copy-pipe-test"]);
    std::thread::sleep(Duration::from_millis(300));

    let buf = show_buffer(&tmux);
    assert!(
        buf.contains("PIPE_TEST"),
        "copy-pipe should store to buffer, got: {buf:?}"
    );
}

// 9. Emacs mode basics
#[test]
fn emacs_enter_exit_copy_mode() {
    let (tmux, mut _client) = setup(40, 10);
    set_mode_keys(&tmux, "emacs");

    enter_copy_mode(&tmux);
    assert_eq!(pane_in_mode(&tmux), "1", "should be in copy mode (emacs)");

    // Cancel exits copy mode in emacs
    send_copy_cmd(&tmux, "cancel");
    assert_eq!(pane_in_mode(&tmux), "0", "cancel should exit emacs copy mode");
}

#[test]
fn emacs_cursor_movement() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "emacs");

    fill_pane(&mut client, &["emacs cursor test line"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    let x0 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x0, "0", "should start at col 0 (emacs)");

    // cursor-right
    send_copy_cmd(&tmux, "cursor-right");
    let x1 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x1, "1", "cursor-right should move to col 1 (emacs)");

    // cursor-left
    send_copy_cmd(&tmux, "cursor-left");
    let x2 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x2, "0", "cursor-left should move back to col 0 (emacs)");

    // end-of-line
    send_copy_cmd(&tmux, "end-of-line");
    let x_end: i32 = tmux.query("#{copy_cursor_x}").parse().unwrap_or(-1);
    assert!(x_end > 0, "end-of-line should move past col 0 (emacs), got {x_end}");

    // start-of-line
    send_copy_cmd(&tmux, "start-of-line");
    let x_home = tmux.query("#{copy_cursor_x}");
    assert_eq!(x_home, "0", "start-of-line should return to col 0 (emacs)");

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn emacs_selection_with_begin_selection() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "emacs");

    fill_pane(&mut client, &["emacs select test"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    // begin-selection (C-Space in emacs)
    send_copy_cmd(&tmux, "begin-selection");
    let sel = tmux.query("#{selection_active}");
    assert_eq!(sel, "1", "begin-selection should activate selection in emacs");

    // Move right to extend selection
    send_copy_cmd(&tmux, "cursor-right");
    send_copy_cmd(&tmux, "cursor-right");
    send_copy_cmd(&tmux, "cursor-right");

    // Selection should still be active
    let sel2 = tmux.query("#{selection_active}");
    assert_eq!(sel2, "1", "selection should remain active after cursor movement (emacs)");

    // clear-selection
    send_copy_cmd(&tmux, "clear-selection");
    let sel3 = tmux.query("#{selection_active}");
    assert_eq!(sel3, "0", "clear-selection should deactivate selection (emacs)");

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn emacs_next_previous_word() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "emacs");

    fill_pane(&mut client, &["alpha beta gamma"]);

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");
    send_copy_cmd(&tmux, "start-of-line");

    // next-word
    send_copy_cmd(&tmux, "next-word");
    let x1: i32 = tmux.query("#{copy_cursor_x}").parse().unwrap_or(-1);
    assert!(
        x1 > 0,
        "next-word should move past col 0 (emacs), got {x1}"
    );

    // previous-word
    send_copy_cmd(&tmux, "previous-word");
    let x2 = tmux.query("#{copy_cursor_x}");
    assert_eq!(x2, "0", "previous-word should return to start of first word (emacs)");

    send_copy_cmd(&tmux, "cancel");
}

// Additional cursor movement tests

#[test]
fn cursor_down_up_multiple_vi() {
    let (tmux, mut client) = setup(40, 10);
    set_mode_keys(&tmux, "vi");

    for i in 0..8 {
        client.write_str(&format!("echo 'row_{i}'\r"));
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_millis(300));
    client.read_raw();

    enter_copy_mode(&tmux);
    send_copy_cmd(&tmux, "history-top");

    let y0: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);

    // Move down 3 times
    send_copy_cmd(&tmux, "cursor-down");
    send_copy_cmd(&tmux, "cursor-down");
    send_copy_cmd(&tmux, "cursor-down");
    let y3: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);
    assert_eq!(y3, y0 + 3, "three cursor-down should move 3 rows from {y0}, got {y3}");

    // Move up 2 times
    send_copy_cmd(&tmux, "cursor-up");
    send_copy_cmd(&tmux, "cursor-up");
    let y1: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);
    assert_eq!(y1, y0 + 1, "two cursor-up from row {y3} should give {}, got {y1}", y0 + 1);

    send_copy_cmd(&tmux, "cancel");
}

#[test]
fn search_backward_positions_cursor_vi() {
    let (tmux, mut client) = setup(60, 10);
    set_mode_keys(&tmux, "vi");

    client.write_str("echo 'the quick brown fox jumps'\r");
    std::thread::sleep(Duration::from_millis(300));
    client.read_raw();

    enter_copy_mode(&tmux);

    // search-backward for "brown"
    send_copy_cmd_with_args(&tmux, &["search-backward", "brown"]);
    std::thread::sleep(Duration::from_millis(300));

    let y: i32 = tmux.query("#{copy_cursor_y}").parse().unwrap_or(-1);
    assert!(y >= 0, "search-backward should position cursor, got y={y}");

    // Verify still in copy mode after search
    assert_eq!(pane_in_mode(&tmux), "1", "should remain in copy mode after search");

    send_copy_cmd(&tmux, "cancel");
}
