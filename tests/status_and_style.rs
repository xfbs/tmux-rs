use super::harness;

use std::time::Duration;

use harness::{PtyClient, TmuxTestHarness};

// ---------------------------------------------------------------------------
// Helper: create a harness with a ready session
// ---------------------------------------------------------------------------

fn setup() -> TmuxTestHarness {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    tmux
}

/// Helper to set a global option and read it back with show-options -gv.
fn set_and_get_global(tmux: &TmuxTestHarness, option: &str, value: &str) -> String {
    tmux.cmd()
        .args(["set-option", "-g", option, value])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["show-options", "-gv", option])
        .run()
        .stdout_trimmed()
}

/// Helper to set a session option and read it back with show-options -v.
fn set_and_get_session(tmux: &TmuxTestHarness, option: &str, value: &str) -> String {
    tmux.cmd()
        .args(["set-option", option, value])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["show-options", "-v", option])
        .run()
        .stdout_trimmed()
}

/// Helper to set a window option and read it back with show-window-options -gv.
fn set_and_get_window(tmux: &TmuxTestHarness, option: &str, value: &str) -> String {
    tmux.cmd()
        .args(["set-window-option", "-g", option, value])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["show-window-options", "-gv", option])
        .run()
        .stdout_trimmed()
}

// ===========================================================================
// Status bar tests
// ===========================================================================

#[test]
fn status_on_off() {
    let tmux = setup();

    // Turn status off
    let val = set_and_get_session(&tmux, "status", "off");
    assert_eq!(val, "off");

    // Turn status back on
    let val = set_and_get_session(&tmux, "status", "on");
    assert_eq!(val, "on");
}

#[test]
fn status_position_top_and_bottom() {
    let tmux = setup();

    let val = set_and_get_session(&tmux, "status-position", "top");
    assert_eq!(val, "top");

    let val = set_and_get_session(&tmux, "status-position", "bottom");
    assert_eq!(val, "bottom");
}

#[test]
fn status_left_custom_format() {
    let tmux = setup();

    let val = set_and_get_session(&tmux, "status-left", "HELLO");
    assert_eq!(val, "HELLO");

    let val = set_and_get_session(&tmux, "status-left", "[#S]");
    assert_eq!(val, "[#S]");
}

#[test]
fn status_right_custom_format() {
    let tmux = setup();

    let val = set_and_get_session(&tmux, "status-right", "world");
    assert_eq!(val, "world");
}

#[test]
fn status_left_length() {
    let tmux = setup();

    let val = set_and_get_session(&tmux, "status-left-length", "50");
    assert_eq!(val, "50");
}

#[test]
fn status_right_length() {
    let tmux = setup();

    let val = set_and_get_session(&tmux, "status-right-length", "80");
    assert_eq!(val, "80");
}

#[test]
fn status_style_bg_fg() {
    let tmux = setup();

    tmux.cmd()
        .args(["set-option", "-g", "status-style", "bg=red,fg=white"])
        .run()
        .assert_success();

    let val = tmux
        .cmd()
        .args(["show-options", "-gv", "status-style"])
        .run()
        .stdout_trimmed();

    // tmux may normalize the order; just check both attributes are present
    assert!(val.contains("bg=red"), "expected bg=red in '{val}'");
    assert!(val.contains("fg=white"), "expected fg=white in '{val}'");
}

#[test]
fn status_interval() {
    let tmux = setup();

    let val = set_and_get_session(&tmux, "status-interval", "5");
    assert_eq!(val, "5");

    let val = set_and_get_session(&tmux, "status-interval", "30");
    assert_eq!(val, "30");
}

#[test]
fn status_multiple_lines() {
    let tmux = setup();

    // status 2 means two status lines
    let val = set_and_get_session(&tmux, "status", "2");
    assert_eq!(val, "2");
}

#[test]
fn window_status_format() {
    let tmux = setup();

    let val = set_and_get_window(&tmux, "window-status-format", "#I:#W");
    assert_eq!(val, "#I:#W");
}

#[test]
fn window_status_current_format() {
    let tmux = setup();

    let val = set_and_get_window(&tmux, "window-status-current-format", "[#I:#W]");
    assert_eq!(val, "[#I:#W]");
}

#[test]
fn status_left_with_session_name_variable() {
    let tmux = setup();

    // Set status-left to include session name format
    tmux.cmd()
        .args(["set-option", "status-left", "#{session_name}"])
        .run()
        .assert_success();

    // Query the format to see the expanded value
    let session_name = tmux.query("#{session_name}");
    assert!(
        !session_name.is_empty(),
        "session_name should not be empty"
    );
}

// ===========================================================================
// Style tests
// ===========================================================================

#[test]
fn style_fg_red() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "status-style", "fg=red");
    assert!(val.contains("fg=red"), "expected fg=red in '{val}'");
}

#[test]
fn style_bg_blue() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "status-style", "bg=blue");
    assert!(val.contains("bg=blue"), "expected bg=blue in '{val}'");
}

#[test]
fn style_bold() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "status-style", "bold");
    assert!(val.contains("bold"), "expected bold in '{val}'");
}

#[test]
fn style_underscore() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "status-style", "underscore");
    assert!(val.contains("underscore"), "expected underscore in '{val}'");
}

#[test]
fn style_colour_number() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "status-style", "fg=colour123");
    assert!(
        val.contains("colour123"),
        "expected colour123 in '{val}'"
    );
}

#[test]
fn style_hex_color() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "status-style", "fg=#ff0000");
    assert!(val.contains("#ff0000"), "expected #ff0000 in '{val}'");
}

#[test]
fn window_style_and_active_style() {
    let tmux = setup();

    let val = set_and_get_window(&tmux, "window-style", "bg=black");
    assert!(val.contains("bg=black"), "expected bg=black in '{val}'");

    let val = set_and_get_window(&tmux, "window-active-style", "bg=white");
    assert!(val.contains("bg=white"), "expected bg=white in '{val}'");
}

#[test]
fn pane_border_style() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "pane-border-style", "fg=green");
    assert!(val.contains("fg=green"), "expected fg=green in '{val}'");
}

#[test]
fn pane_active_border_style() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "pane-active-border-style", "fg=yellow");
    assert!(
        val.contains("fg=yellow"),
        "expected fg=yellow in '{val}'"
    );
}

#[test]
fn message_style() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "message-style", "fg=cyan,bg=black");
    assert!(val.contains("fg=cyan"), "expected fg=cyan in '{val}'");
    assert!(val.contains("bg=black"), "expected bg=black in '{val}'");
}

#[test]
fn mode_style() {
    let tmux = setup();

    let val = set_and_get_global(&tmux, "mode-style", "bg=yellow,fg=black");
    assert!(
        val.contains("bg=yellow"),
        "expected bg=yellow in '{val}'"
    );
    assert!(
        val.contains("fg=black"),
        "expected fg=black in '{val}'"
    );
}

// ===========================================================================
// Capture-pane tests
// ===========================================================================

#[test]
fn capture_pane_with_escapes() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Attach a PTY client so there is an attached client for rendering
    let mut pty = PtyClient::attach(&tmux, 80, 24);

    // Send a command that produces colored output
    tmux.send_keys(&["printf '\\033[31mRED\\033[0m'", "Enter"]);
    std::thread::sleep(Duration::from_millis(500));

    // Drain PTY output so the client stays in sync
    let _ = pty.read_raw();

    // Capture with -e (include escape sequences)
    let result = tmux
        .cmd()
        .args(["capture-pane", "-p", "-e"])
        .run();
    let output = result.stdout_str();

    // The output should contain ANSI escape sequences (ESC = \x1b = \033)
    assert!(
        output.contains('\x1b'),
        "capture-pane -e should include ANSI escapes, got: {:?}",
        &output[..output.len().min(200)]
    );
}

#[test]
fn capture_pane_target() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Create a second pane via split-window
    tmux.cmd()
        .args(["split-window", "-d"])
        .run()
        .assert_success();

    // Send some text to pane 1 (the second pane, index starts at 0)
    tmux.send_keys(&["-t", ":.1", "echo PANE_ONE_TEXT", "Enter"]);
    std::thread::sleep(Duration::from_millis(500));

    // Capture pane 1 specifically
    let output = tmux.capture_pane_target(":.1");
    assert!(
        output.contains("PANE_ONE_TEXT"),
        "capture-pane -t should capture the targeted pane, got: {output}"
    );
}
