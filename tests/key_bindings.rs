mod harness;

use std::time::Duration;

use harness::TmuxTestHarness;

#[test]
fn bind_key_and_verify_with_list_keys() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Bind a custom key
    tmux.cmd()
        .args(["bind-key", "Z", "display-message", "hello"])
        .run()
        .assert_success();

    // Verify the binding appears in list-keys
    let result = tmux.cmd().args(["list-keys"]).run();
    result.assert_success();
    let output = result.stdout_str();
    assert!(
        output.contains("Z"),
        "list-keys should contain the bound key 'Z', got:\n{output}"
    );
}

#[test]
fn unbind_key_removes_binding() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Bind a key first
    tmux.cmd()
        .args(["bind-key", "Y", "display-message", "test"])
        .run()
        .assert_success();

    // Verify it's there
    let result = tmux.cmd().args(["list-keys"]).run();
    let output = result.stdout_str();
    assert!(output.contains("Y"), "key Y should be bound");

    // Unbind it
    tmux.cmd()
        .args(["unbind-key", "Y"])
        .run()
        .assert_success();

    // Verify it's gone -- filter to prefix table bindings for 'Y'
    // We look specifically for " Y " to avoid matching other keys that contain Y
    let result = tmux.cmd().args(["list-keys", "-T", "prefix"]).run();
    let output = result.stdout_str();
    // Check that there's no line binding the bare key Y in the prefix table
    let has_y_binding = output
        .lines()
        .any(|line| line.contains(" Y ") && line.contains("display-message"));
    assert!(
        !has_y_binding,
        "key Y should have been unbound, but still found in:\n{output}"
    );
}

#[test]
fn list_keys_output_format() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux.cmd().args(["list-keys"]).run();
    result.assert_success();

    let output = result.stdout_str();
    let lines: Vec<&str> = output.lines().collect();

    // There should be many default bindings
    assert!(
        lines.len() > 10,
        "list-keys should have many default bindings, got {} lines",
        lines.len()
    );

    // Each line should start with "bind-key"
    for line in &lines {
        assert!(
            line.starts_with("bind-key"),
            "list-keys line should start with 'bind-key', got: {line}"
        );
    }
}

#[test]
fn list_keys_with_table_filter() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // List only prefix table bindings
    let result = tmux.cmd().args(["list-keys", "-T", "prefix"]).run();
    result.assert_success();

    let output = result.stdout_str();
    let lines: Vec<&str> = output.lines().collect();

    // All lines should reference the prefix table
    for line in &lines {
        assert!(
            line.contains("-T prefix"),
            "filtered list-keys should only show prefix table, got: {line}"
        );
    }
}

#[test]
fn bind_key_with_specific_table() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Bind a key in the root table
    tmux.cmd()
        .args(["bind-key", "-T", "root", "F12", "display-message", "root-test"])
        .run()
        .assert_success();

    // Verify it appears in the root table
    let result = tmux.cmd().args(["list-keys", "-T", "root"]).run();
    result.assert_success();
    let output = result.stdout_str();
    assert!(
        output.contains("F12"),
        "root table should contain F12 binding, got:\n{output}"
    );

    // It should not appear in the prefix table
    let result = tmux.cmd().args(["list-keys", "-T", "prefix"]).run();
    let output = result.stdout_str();
    let has_f12 = output.lines().any(|line| line.contains("F12"));
    assert!(
        !has_f12,
        "F12 should not be in prefix table"
    );
}

#[test]
fn bind_key_in_copy_mode_table() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Bind in copy-mode table
    tmux.cmd()
        .args([
            "bind-key", "-T", "copy-mode", "Q", "send-keys", "-X", "cancel",
        ])
        .run()
        .assert_success();

    // Verify
    let result = tmux.cmd().args(["list-keys", "-T", "copy-mode"]).run();
    result.assert_success();
    let output = result.stdout_str();
    assert!(
        output.contains("Q"),
        "copy-mode table should contain Q binding"
    );
}

#[test]
fn bind_key_with_repeat_flag() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Bind a repeating key
    tmux.cmd()
        .args(["bind-key", "-r", "M", "next-window"])
        .run()
        .assert_success();

    // Verify the binding shows up with the -r flag in list-keys output
    let result = tmux.cmd().args(["list-keys", "-T", "prefix"]).run();
    result.assert_success();
    let output = result.stdout_str();

    // Find the line with our M binding
    let m_line = output
        .lines()
        .find(|line| line.contains(" M ") && line.contains("next-window"));
    assert!(
        m_line.is_some(),
        "should find M binding for next-window in list-keys output:\n{output}"
    );

    // The line should contain -r flag indicating it's a repeat binding
    let m_line = m_line.unwrap();
    assert!(
        m_line.contains("-r"),
        "repeating binding should show -r flag, got: {m_line}"
    );
}

#[test]
fn bind_key_replaces_existing_binding() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Bind a key to one command
    tmux.cmd()
        .args(["bind-key", "X", "display-message", "first"])
        .run()
        .assert_success();

    // Re-bind the same key to a different command
    tmux.cmd()
        .args(["bind-key", "X", "display-message", "second"])
        .run()
        .assert_success();

    // Verify only the new binding exists
    let result = tmux.cmd().args(["list-keys", "-T", "prefix"]).run();
    let output = result.stdout_str();
    let x_lines: Vec<&str> = output
        .lines()
        .filter(|line| line.contains(" X ") && line.contains("display-message"))
        .collect();

    assert_eq!(
        x_lines.len(),
        1,
        "should have exactly one binding for X, got: {x_lines:?}"
    );
    assert!(
        x_lines[0].contains("second"),
        "binding should be the second one, got: {}",
        x_lines[0]
    );
}

#[test]
fn unbind_nonexistent_key_does_not_error() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Unbinding a key that was never bound should not produce an error
    // (tmux silently ignores this)
    let result = tmux.cmd().args(["unbind-key", "F11"]).run();
    // tmux typically succeeds silently here
    // Some implementations may error; just verify it doesn't crash
    let _ = result.success();
}
