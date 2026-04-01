use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

/// Set up a tmux session and open a window running `cat -tv` in raw mode,
/// mirroring the upstream regress/input-keys.sh pattern.
///
/// Returns `(tmux, window_target)`.
fn setup_cat_window(tmux: &TmuxTestHarness) -> String {
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
    window
}

/// Send a key to the window, then capture what `cat -tv` printed.
fn assert_key(tmux: &TmuxTestHarness, key: &str, expected: &str) {
    let window = setup_cat_window(tmux);

    tmux.send_keys(&["-t", &window, key, "EOL"]).assert_success();
    std::thread::sleep(Duration::from_millis(300));

    let captured = tmux.capture_pane_target(&window);
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

    tmux.cmd()
        .args(["kill-window", "-t", &window])
        .run()
        .assert_success();
}

fn new_harness() -> TmuxTestHarness {
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
    tmux
}

// ---------------------------------------------------------------------------
// 1. Control keys (C-a through C-z)
// ---------------------------------------------------------------------------
#[test]
fn input_keys_control() {
    let tmux = new_harness();

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
        assert_key(&tmux, key, expected);
    }
}

// ---------------------------------------------------------------------------
// 2. Arrow keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_arrows() {
    let tmux = new_harness();

    assert_key(&tmux, "Up", "^[[A");
    assert_key(&tmux, "Down", "^[[B");
    assert_key(&tmux, "Right", "^[[C");
    assert_key(&tmux, "Left", "^[[D");
}

// ---------------------------------------------------------------------------
// 3. Function keys (F1-F12)
// ---------------------------------------------------------------------------
#[test]
fn input_keys_function() {
    let tmux = new_harness();

    assert_key(&tmux, "F1", "^[OP");
    assert_key(&tmux, "F2", "^[OQ");
    assert_key(&tmux, "F3", "^[OR");
    assert_key(&tmux, "F4", "^[OS");
    assert_key(&tmux, "F5", "^[[15~");
    assert_key(&tmux, "F6", "^[[17~");
    assert_key(&tmux, "F8", "^[[19~");
    assert_key(&tmux, "F9", "^[[20~");
    assert_key(&tmux, "F10", "^[[21~");
    assert_key(&tmux, "F11", "^[[23~");
    assert_key(&tmux, "F12", "^[[24~");
}

// ---------------------------------------------------------------------------
// 4. Meta/Alt keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_meta() {
    let tmux = new_harness();

    assert_key(&tmux, "M-C-a", "^[^A");
    assert_key(&tmux, "M-C-b", "^[^B");
    assert_key(&tmux, "M-C-c", "^[^C");
    assert_key(&tmux, "M-C-z", "^[^Z");
    assert_key(&tmux, "M-a", "^[a");
    assert_key(&tmux, "M-z", "^[z");
    assert_key(&tmux, "M-Space", "^[ ");
    assert_key(&tmux, "M-Tab", "^[^I");
    assert_key(&tmux, "M-BSpace", "^[^?");
}

// ---------------------------------------------------------------------------
// 5. Special keys (Home, End, Insert, Delete, PageUp, PageDown)
// ---------------------------------------------------------------------------
#[test]
fn input_keys_special() {
    let tmux = new_harness();

    assert_key(&tmux, "IC", "^[[2~");
    assert_key(&tmux, "Insert", "^[[2~");
    assert_key(&tmux, "DC", "^[[3~");
    assert_key(&tmux, "Delete", "^[[3~");
    assert_key(&tmux, "Home", "^[[1~");
    assert_key(&tmux, "End", "^[[4~");
    assert_key(&tmux, "NPage", "^[[6~");
    assert_key(&tmux, "PageDown", "^[[6~");
    assert_key(&tmux, "PgDn", "^[[6~");
    assert_key(&tmux, "PPage", "^[[5~");
    assert_key(&tmux, "PageUp", "^[[5~");
    assert_key(&tmux, "PgUp", "^[[5~");
    assert_key(&tmux, "BTab", "^[[Z");
}

// ---------------------------------------------------------------------------
// 6. Escape key
// ---------------------------------------------------------------------------
#[test]
fn input_keys_escape() {
    let tmux = new_harness();

    assert_key(&tmux, "Escape", "^[");
    assert_key(&tmux, "M-Escape", "^[^[");
}

// ---------------------------------------------------------------------------
// 7. Tab and Backspace
// ---------------------------------------------------------------------------
#[test]
fn input_keys_tab_bspace() {
    let tmux = new_harness();

    assert_key(&tmux, "Tab", "^I");
    assert_key(&tmux, "BSpace", "^?");
}

// ---------------------------------------------------------------------------
// 8. Printable / literal keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_printable() {
    let tmux = new_harness();

    assert_key(&tmux, "Space", " ");
    assert_key(&tmux, "a", "a");
    assert_key(&tmux, "z", "z");
    assert_key(&tmux, "A", "A");
    assert_key(&tmux, "Z", "Z");
    assert_key(&tmux, "0", "0");
    assert_key(&tmux, "9", "9");
    assert_key(&tmux, "!", "!");
    assert_key(&tmux, "@", "@");
    assert_key(&tmux, "#", "#");
}

// ---------------------------------------------------------------------------
// 9. Keypad keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_keypad() {
    let tmux = new_harness();

    assert_key(&tmux, "KP*", "*");
    assert_key(&tmux, "KP+", "+");
    assert_key(&tmux, "KP-", "-");
    assert_key(&tmux, "KP.", ".");
    assert_key(&tmux, "KP/", "/");
    assert_key(&tmux, "KP0", "0");
    assert_key(&tmux, "KP1", "1");
    assert_key(&tmux, "KP9", "9");
}

// ---------------------------------------------------------------------------
// 10. Misc control punctuation keys
// ---------------------------------------------------------------------------
#[test]
fn input_keys_control_punct() {
    let tmux = new_harness();

    assert_key(&tmux, "C-]", "^]");
    assert_key(&tmux, "C-^", "^^");
    assert_key(&tmux, "C-_", "^_");
}
