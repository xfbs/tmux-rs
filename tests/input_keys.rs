use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

/// Set up a tmux session and open a window running `cat -tv` in raw mode,
/// mirroring the upstream regress/input-keys.sh pattern.
///
/// Returns `(harness, window_target)`.
fn new_harness_with_cat() -> (TmuxTestHarness, String) {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    tmux.cmd()
        .args(["set", "-g", "escape-time", "0"])
        .run()
        .assert_success();

    let result = tmux
        .cmd()
        .args([
            "new-window",
            "-P",
            "-F",
            "#{window_id}",
            "--",
            "sh",
            "-c",
            "stty raw -echo && cat -tv",
        ])
        .run();
    result.assert_success();
    let window = result.stdout_trimmed();
    // Give cat -tv a moment to start.
    std::thread::sleep(Duration::from_millis(300));
    (tmux, window)
}

/// Send a key to the window, then capture what `cat -tv` printed.
/// Reuses an existing cat -tv window, clearing the terminal between keys.
fn assert_key(tmux: &TmuxTestHarness, window: &str, key: &str, expected: &str) {
    // Reset terminal and clear history so we get a clean capture.
    tmux.cmd()
        .args(["send-keys", "-t", window, "-R"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["clear-history", "-t", window])
        .run()
        .assert_success();
    std::thread::sleep(Duration::from_millis(50));

    tmux.send_keys(&["-t", window, key, "EOL"]).assert_success();
    std::thread::sleep(Duration::from_millis(100));

    let captured = tmux.capture_pane_target(window);
    // The first line should contain <expected>EOL...
    let first_line = captured.lines().next().unwrap_or("");
    let actual = if let Some(pos) = first_line.find("EOL") {
        &first_line[..pos]
    } else {
        first_line
    };

    assert_eq!(
        actual, expected,
        "key={key}: expected {expected:?}, got {actual:?} (full line: {first_line:?})"
    );
}

// ---------------------------------------------------------------------------
// 1. Control keys (C-a through C-z)
// ---------------------------------------------------------------------------
#[test]
fn input_keys_control() {
    let (tmux, window) = new_harness_with_cat();

    // C-a (^A) through C-z (^Z), skipping C-j which produces newline.
    let cases: Vec<(&str, &str)> = vec![
        ("C-Space", "^@"),
        ("C-a", "^A"),
        ("C-b", "^B"),
        ("C-c", "^C"),
        ("C-d", "^D"),
        ("C-e", "^E"),
        ("C-f", "^F"),
        ("C-g", "^G"),
        ("C-h", "^H"),
        ("C-i", "^I"),
        // C-j produces newline — cat -tv shows empty first line
        ("C-k", "^K"),
        ("C-l", "^L"),
        ("C-m", "^M"),
        ("C-n", "^N"),
        ("C-o", "^O"),
        ("C-p", "^P"),
        ("C-q", "^Q"),
        ("C-r", "^R"),
        ("C-s", "^S"),
        ("C-t", "^T"),
        ("C-u", "^U"),
        ("C-v", "^V"),
        ("C-w", "^W"),
        ("C-x", "^X"),
        ("C-y", "^Y"),
        ("C-z", "^Z"),
    ];

    for (key, expected) in &cases {
        assert_key(&tmux, &window, key, expected);
    }
}

// ---------------------------------------------------------------------------
// 2. Arrow keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_arrows() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "Up", "^[[A");
    assert_key(&tmux, &window, "Down", "^[[B");
    assert_key(&tmux, &window, "Right", "^[[C");
    assert_key(&tmux, &window, "Left", "^[[D");
}

// ---------------------------------------------------------------------------
// 3. Function keys (F1-F12)
// ---------------------------------------------------------------------------
#[test]
fn input_keys_function() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "F1", "^[OP");
    assert_key(&tmux, &window, "F2", "^[OQ");
    assert_key(&tmux, &window, "F3", "^[OR");
    assert_key(&tmux, &window, "F4", "^[OS");
    assert_key(&tmux, &window, "F5", "^[[15~");
    assert_key(&tmux, &window, "F6", "^[[17~");
    assert_key(&tmux, &window, "F8", "^[[19~");
    assert_key(&tmux, &window, "F9", "^[[20~");
    assert_key(&tmux, &window, "F10", "^[[21~");
    assert_key(&tmux, &window, "F11", "^[[23~");
    assert_key(&tmux, &window, "F12", "^[[24~");
}

// ---------------------------------------------------------------------------
// 4. Meta/Alt keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_meta() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "M-C-a", "^[^A");
    assert_key(&tmux, &window, "M-C-b", "^[^B");
    assert_key(&tmux, &window, "M-C-c", "^[^C");
    assert_key(&tmux, &window, "M-C-z", "^[^Z");
    assert_key(&tmux, &window, "M-a", "^[a");
    assert_key(&tmux, &window, "M-z", "^[z");
    assert_key(&tmux, &window, "M-Space", "^[ ");
    assert_key(&tmux, &window, "M-Tab", "^[^I");
    assert_key(&tmux, &window, "M-BSpace", "^[^?");
}

// ---------------------------------------------------------------------------
// 5. Special keys (Home, End, Insert, Delete, PageUp, PageDown)
// ---------------------------------------------------------------------------
#[test]
fn input_keys_special() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "IC", "^[[2~");
    assert_key(&tmux, &window, "Insert", "^[[2~");
    assert_key(&tmux, &window, "DC", "^[[3~");
    assert_key(&tmux, &window, "Delete", "^[[3~");
    assert_key(&tmux, &window, "Home", "^[[1~");
    assert_key(&tmux, &window, "End", "^[[4~");
    assert_key(&tmux, &window, "NPage", "^[[6~");
    assert_key(&tmux, &window, "PageDown", "^[[6~");
    assert_key(&tmux, &window, "PgDn", "^[[6~");
    assert_key(&tmux, &window, "PPage", "^[[5~");
    assert_key(&tmux, &window, "PageUp", "^[[5~");
    assert_key(&tmux, &window, "PgUp", "^[[5~");
    assert_key(&tmux, &window, "BTab", "^[[Z");
}

// ---------------------------------------------------------------------------
// 6. Escape key
// ---------------------------------------------------------------------------
#[test]
fn input_keys_escape() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "Escape", "^[");
    assert_key(&tmux, &window, "M-Escape", "^[^[");
}

// ---------------------------------------------------------------------------
// 7. Tab and Backspace
// ---------------------------------------------------------------------------
#[test]
fn input_keys_tab_bspace() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "Tab", "^I");
    assert_key(&tmux, &window, "BSpace", "^?");
}

// ---------------------------------------------------------------------------
// 8. Printable / literal keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_printable() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "Space", " ");
    assert_key(&tmux, &window, "a", "a");
    assert_key(&tmux, &window, "z", "z");
    assert_key(&tmux, &window, "A", "A");
    assert_key(&tmux, &window, "Z", "Z");
    assert_key(&tmux, &window, "0", "0");
    assert_key(&tmux, &window, "9", "9");
    assert_key(&tmux, &window, "!", "!");
    assert_key(&tmux, &window, "@", "@");
    assert_key(&tmux, &window, "#", "#");
}

// ---------------------------------------------------------------------------
// 9. Keypad keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_keypad() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "KP*", "*");
    assert_key(&tmux, &window, "KP+", "+");
    assert_key(&tmux, &window, "KP-", "-");
    assert_key(&tmux, &window, "KP.", ".");
    assert_key(&tmux, &window, "KP/", "/");
    assert_key(&tmux, &window, "KP0", "0");
    assert_key(&tmux, &window, "KP1", "1");
    assert_key(&tmux, &window, "KP9", "9");
}

// ---------------------------------------------------------------------------
// 10. Misc control punctuation keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_control_punct() {
    let (tmux, window) = new_harness_with_cat();

    assert_key(&tmux, &window, "C-]", "^]");
    assert_key(&tmux, &window, "C-^", "^^");
    assert_key(&tmux, &window, "C-_", "^_");
}
