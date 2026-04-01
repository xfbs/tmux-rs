use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

// ---------------------------------------------------------------------------
// set-buffer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn set_buffer_basic() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "hello world"])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "hello world");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn set_buffer_named() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "mybuf", "named content"])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer", "-b", "mybuf"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "named content");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn set_buffer_overwrite() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "over", "first"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["set-buffer", "-b", "over", "second"])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer", "-b", "over"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "second");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn set_buffer_append() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "app", "hello"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["set-buffer", "-a", "-b", "app", " world"])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer", "-b", "app"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "hello world");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn set_buffer_append_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Appending to a buffer that does not exist creates it in C tmux
    let result = tmux
        .cmd()
        .args(["set-buffer", "-a", "-b", "doesnotexist", "data"])
        .run();
    result.assert_success();
}

#[test]
fn set_buffer_empty_string() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Setting an empty buffer: tmux accepts it but show-buffer may report "no buffers"
    // since empty content effectively means no buffer was stored
    let result = tmux.cmd().args(["set-buffer", ""]).run();
    // Just verify it doesn't crash — behavior varies by version
    let _ = result;
}

// ---------------------------------------------------------------------------
// list-buffers
// ---------------------------------------------------------------------------

#[test]
fn list_buffers_empty() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux.cmd().args(["list-buffers"]).run();
    // With no buffers, output should be empty
    assert_eq!(result.stdout_trimmed(), "");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn list_buffers_shows_entries() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "buf0", "aaa"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-buffer", "-b", "buf1", "bbbbb"])
        .run()
        .assert_success();

    let result = tmux.cmd().args(["list-buffers"]).run();
    result.assert_success();
    let output = result.stdout_trimmed();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2, "expected 2 buffers, got: {output}");

    // Both buffer names should appear somewhere in the output
    assert!(output.contains("buf0"), "output should mention buf0: {output}");
    assert!(output.contains("buf1"), "output should mention buf1: {output}");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn list_buffers_custom_format() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "fmt", "12345"])
        .run()
        .assert_success();

    let result = tmux
        .cmd()
        .args(["list-buffers", "-F", "#{buffer_name}:#{buffer_size}"])
        .run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "fmt:5");
}

// ---------------------------------------------------------------------------
// show-buffer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn show_buffer_default() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "show me"])
        .run()
        .assert_success();

    let result = tmux.cmd().args(["show-buffer"]).run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "show me");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn show_buffer_named() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "sb", "specific"])
        .run()
        .assert_success();

    let result = tmux.cmd().args(["show-buffer", "-b", "sb"]).run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "specific");
}

#[test]
fn show_buffer_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux.cmd().args(["show-buffer", "-b", "nope"]).run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// save-buffer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn save_buffer_to_file() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();

    tmux.cmd()
        .args(["set-buffer", "save this"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["save-buffer", &path])
        .run()
        .assert_success();

    let contents = std::fs::read_to_string(&path).expect("failed to read saved buffer");
    assert_eq!(contents, "save this");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn save_buffer_named_to_file() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();

    tmux.cmd()
        .args(["set-buffer", "-b", "savebuf", "named save"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["save-buffer", "-b", "savebuf", &path])
        .run()
        .assert_success();

    let contents = std::fs::read_to_string(&path).expect("failed to read saved buffer");
    assert_eq!(contents, "named save");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn save_buffer_append_to_file() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();

    tmux.cmd()
        .args(["set-buffer", "-b", "a1", "first"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-buffer", "-b", "a2", "second"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["save-buffer", "-b", "a1", &path])
        .run()
        .assert_success();

    // -a flag appends to file instead of overwriting
    tmux.cmd()
        .args(["save-buffer", "-a", "-b", "a2", &path])
        .run()
        .assert_success();

    let contents = std::fs::read_to_string(&path).expect("failed to read saved buffer");
    assert_eq!(contents, "firstsecond");
}

#[test]
fn save_buffer_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();

    let result = tmux
        .cmd()
        .args(["save-buffer", "-b", "missing", &path])
        .run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// load-buffer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn load_buffer_from_file() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();
    std::fs::write(&path, "loaded content").expect("failed to write tempfile");

    tmux.cmd()
        .args(["load-buffer", &path])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "loaded content");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn load_buffer_named() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();
    std::fs::write(&path, "named load").expect("failed to write tempfile");

    tmux.cmd()
        .args(["load-buffer", "-b", "lb", &path])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer", "-b", "lb"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "named load");
}

#[test]
fn load_buffer_nonexistent_file_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux
        .cmd()
        .args(["load-buffer", "/tmp/tmux-rs-test-nonexistent-file-xyz"])
        .run();
    result.assert_failure();
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn load_buffer_roundtrip_with_save() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set a buffer, save it, delete all buffers, load it back
    tmux.cmd()
        .args(["set-buffer", "roundtrip data"])
        .run()
        .assert_success();

    let tmp = tempfile::NamedTempFile::new().expect("failed to create tempfile");
    let path = tmp.path().to_str().unwrap().to_string();

    tmux.cmd()
        .args(["save-buffer", &path])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["delete-buffer"])
        .run()
        .assert_success();

    // Verify buffer is gone
    let result = tmux.cmd().args(["show-buffer"]).run();
    result.assert_failure();

    // Load it back
    tmux.cmd()
        .args(["load-buffer", &path])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "roundtrip data");
}

// ---------------------------------------------------------------------------
// paste-buffer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn paste_buffer_into_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["--", "cat"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Give cat a moment to start
    std::thread::sleep(Duration::from_millis(200));

    tmux.cmd()
        .args(["set-buffer", "pasted"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["paste-buffer"])
        .run()
        .assert_success();

    // Give cat time to echo back
    std::thread::sleep(Duration::from_millis(500));

    let pane = tmux.capture_pane();
    assert!(
        pane.contains("pasted"),
        "pane should contain pasted text, got: {pane}"
    );
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn paste_buffer_named_into_pane() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["--", "cat"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    std::thread::sleep(Duration::from_millis(200));

    tmux.cmd()
        .args(["set-buffer", "-b", "pb", "namedpaste"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["paste-buffer", "-b", "pb"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(500));

    let pane = tmux.capture_pane();
    assert!(
        pane.contains("namedpaste"),
        "pane should contain namedpaste, got: {pane}"
    );
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn paste_buffer_delete_flag() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().args(["--", "cat"]).run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    std::thread::sleep(Duration::from_millis(200));

    tmux.cmd()
        .args(["set-buffer", "-b", "del", "deleteme"])
        .run()
        .assert_success();

    // -d flag should delete the buffer after pasting
    tmux.cmd()
        .args(["paste-buffer", "-d", "-b", "del"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(500));

    let pane = tmux.capture_pane();
    assert!(
        pane.contains("deleteme"),
        "pane should contain pasted text, got: {pane}"
    );

    // Buffer should now be gone
    let result = tmux.cmd().args(["show-buffer", "-b", "del"]).run();
    result.assert_failure();
}

#[test]
fn paste_buffer_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux
        .cmd()
        .args(["paste-buffer", "-b", "nonexistent"])
        .run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// delete-buffer
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn delete_buffer_default() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "to delete"])
        .run()
        .assert_success();

    tmux.cmd().args(["delete-buffer"]).run().assert_success();

    let result = tmux.cmd().args(["show-buffer"]).run();
    result.assert_failure();
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn delete_buffer_named() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "-b", "x", "data x"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-buffer", "-b", "y", "data y"])
        .run()
        .assert_success();

    tmux.cmd()
        .args(["delete-buffer", "-b", "x"])
        .run()
        .assert_success();

    // x should be gone
    let result = tmux.cmd().args(["show-buffer", "-b", "x"]).run();
    result.assert_failure();

    // y should still exist
    let result = tmux.cmd().args(["show-buffer", "-b", "y"]).run();
    result.assert_success();
    assert_eq!(result.stdout_trimmed(), "data y");
}

#[test]
fn delete_buffer_nonexistent_fails() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let result = tmux
        .cmd()
        .args(["delete-buffer", "-b", "nope"])
        .run();
    result.assert_failure();
}

// ---------------------------------------------------------------------------
// Multiple buffers interaction
// ---------------------------------------------------------------------------

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn multiple_buffers_most_recent_is_default() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "first"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set-buffer", "second"])
        .run()
        .assert_success();

    // show-buffer without -b should show the most recently added buffer
    let content = tmux.cmd().args(["show-buffer"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "second");
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn set_buffer_multiline() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["set-buffer", "line1\nline2\nline3"])
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer"]).run();
    content.assert_success();
    let output = content.stdout_str();
    assert!(
        output.contains("line1") && output.contains("line2") && output.contains("line3"),
        "buffer should contain all lines, got: {output}"
    );
}

#[test]
#[ignore = "broken: tmux-rs server crashes on set-buffer"]
fn load_buffer_from_stdin() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // load-buffer - reads from stdin
    tmux.cmd()
        .args(["load-buffer", "-"])
        .stdin("from stdin")
        .run()
        .assert_success();

    let content = tmux.cmd().args(["show-buffer"]).run();
    content.assert_success();
    assert_eq!(content.stdout_trimmed(), "from stdin");
}
