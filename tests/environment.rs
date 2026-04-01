mod harness;

use std::time::Duration;

use harness::TmuxTestHarness;

#[test]
fn set_environment_and_show() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a session-level environment variable
    tmux.cmd()
        .args(["set-environment", "MY_TEST_VAR", "hello_world"])
        .run()
        .assert_success();

    // Verify it appears in show-environment output
    let result = tmux.cmd().args(["show-environment", "MY_TEST_VAR"]).run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "MY_TEST_VAR=hello_world");
}

#[test]
fn set_environment_overwrite() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-environment", "OVERWRITE_VAR", "first"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["set-environment", "OVERWRITE_VAR", "second"])
        .run()
        .assert_success();

    let result = tmux.cmd().args(["show-environment", "OVERWRITE_VAR"]).run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "OVERWRITE_VAR=second");
}

#[test]
fn set_environment_global() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a global environment variable
    tmux.cmd()
        .args(["set-environment", "-g", "GLOBAL_VAR", "global_value"])
        .run()
        .assert_success();

    // Verify it appears in global show-environment
    let result = tmux
        .cmd()
        .args(["show-environment", "-g", "GLOBAL_VAR"])
        .run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "GLOBAL_VAR=global_value");

    // It should NOT appear in session-level environment (unless inherited)
    let result = tmux
        .cmd()
        .args(["show-environment", "GLOBAL_VAR"])
        .run();
    // Looking up a var that doesn't exist at session level should fail
    assert!(!result.success());
}

#[test]
fn set_environment_unset() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set then unset a variable
    tmux.cmd()
        .args(["set-environment", "EPHEMERAL", "temp"])
        .run()
        .assert_success();

    // Confirm it exists
    let result = tmux.cmd().args(["show-environment", "EPHEMERAL"]).run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "EPHEMERAL=temp");

    // Unset it
    tmux.cmd()
        .args(["set-environment", "-u", "EPHEMERAL"])
        .run()
        .assert_success();

    // Now looking it up should fail
    let result = tmux.cmd().args(["show-environment", "EPHEMERAL"]).run();
    assert!(!result.success());
}

#[test]
fn set_environment_global_unset() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-environment", "-g", "G_REMOVE", "bye"])
        .run()
        .assert_success();

    let result = tmux
        .cmd()
        .args(["show-environment", "-g", "G_REMOVE"])
        .run();
    result.assert_success();

    // Unset the global var
    tmux.cmd()
        .args(["set-environment", "-gu", "G_REMOVE"])
        .run()
        .assert_success();

    let result = tmux
        .cmd()
        .args(["show-environment", "-g", "G_REMOVE"])
        .run();
    assert!(!result.success());
}

#[test]
fn show_environment_lists_multiple_vars() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-environment", "LIST_A", "1"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-environment", "LIST_B", "2"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-environment", "LIST_C", "3"])
        .run()
        .assert_success();

    let result = tmux.cmd().args(["show-environment"]).run();
    result.assert_success();
    let output = result.stdout_str();

    assert!(output.contains("LIST_A=1"), "missing LIST_A in:\n{output}");
    assert!(output.contains("LIST_B=2"), "missing LIST_B in:\n{output}");
    assert!(output.contains("LIST_C=3"), "missing LIST_C in:\n{output}");
}

#[test]
fn show_environment_global_lists_vars() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-environment", "-g", "GLIST_X", "alpha"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-environment", "-g", "GLIST_Y", "beta"])
        .run()
        .assert_success();

    let result = tmux.cmd().args(["show-environment", "-g"]).run();
    result.assert_success();
    let output = result.stdout_str();

    assert!(
        output.contains("GLIST_X=alpha"),
        "missing GLIST_X in:\n{output}"
    );
    assert!(
        output.contains("GLIST_Y=beta"),
        "missing GLIST_Y in:\n{output}"
    );
}

#[test]
fn show_environment_nonexistent_var_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux
        .cmd()
        .args(["show-environment", "DOES_NOT_EXIST_12345"])
        .run();
    assert!(!result.success());
}
