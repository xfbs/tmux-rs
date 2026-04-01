use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

#[test]
fn set_and_get_global_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a global option
    tmux.cmd()
        .args(["set-option", "-g", "base-index", "1"])
        .run()
        .assert_success();

    // Read it back
    let val = tmux.cmd().args(["show-options", "-gv", "base-index"]).run();
    assert_eq!(val.stdout_trimmed(), "1");
}

#[test]
fn set_and_get_session_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-option", "status-position", "top"])
        .run()
        .assert_success();

    let val = tmux
        .cmd()
        .args(["show-options", "-v", "status-position"])
        .run();
    assert_eq!(val.stdout_trimmed(), "top");
}

#[test]
fn set_and_get_user_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set", "@myvar", "hello world"])
        .run()
        .assert_success();

    let val = tmux.query("#{@myvar}");
    assert_eq!(val, "hello world");
}

#[test]
fn unset_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set", "@tempvar", "value"])
        .run()
        .assert_success();

    // Verify it's set
    let val = tmux.query("#{@tempvar}");
    assert_eq!(val, "value");

    // Unset it
    tmux.cmd()
        .args(["set", "-u", "@tempvar"])
        .run()
        .assert_success();

    // Should be empty now
    let val = tmux.query("#{@tempvar}");
    assert_eq!(val, "");
}

#[test]
fn window_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-window-option", "mode-keys", "vi"])
        .run()
        .assert_success();

    let val = tmux
        .cmd()
        .args(["show-window-options", "-v", "mode-keys"])
        .run();
    assert_eq!(val.stdout_trimmed(), "vi");
}

#[test]
fn default_terminal_option() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // default-terminal should have a value
    let val = tmux.cmd().args(["show-options", "-gv", "default-terminal"]).run();
    let term = val.stdout_trimmed();
    assert!(!term.is_empty(), "default-terminal should not be empty");
}

#[test]
fn option_inheritance() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set global base-index
    tmux.cmd()
        .args(["set-option", "-g", "base-index", "5"])
        .run()
        .assert_success();

    // Verify global value
    let val = tmux.cmd().args(["show-options", "-gv", "base-index"]).run();
    assert_eq!(val.stdout_trimmed(), "5");

    // Override at session level
    tmux.cmd()
        .args(["set-option", "base-index", "3"])
        .run()
        .assert_success();

    // Session-level override should be visible
    let val = tmux.cmd().args(["show-options", "-v", "base-index"]).run();
    assert_eq!(val.stdout_trimmed(), "3");

    // Global should still be 5
    let val = tmux.cmd().args(["show-options", "-gv", "base-index"]).run();
    assert_eq!(val.stdout_trimmed(), "5");
}
