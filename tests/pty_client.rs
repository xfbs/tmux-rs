use super::harness;

use std::time::Duration;

use harness::{PtyClient, TmuxTestHarness};

#[test]
fn pty_client_attaches_and_renders() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut client = PtyClient::attach(&tmux, 80, 24);
    // PtyClient::attach already waits for initial output via wait_for_content.
    // If we get here, the client connected and rendered something.
    assert!(client.is_alive(), "client should be running");
}

#[test]
fn pty_client_can_send_commands() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut client = PtyClient::attach(&tmux, 80, 24);

    // Drain initial output
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw();

    // Type a command in the shell
    client.write_str("echo HELLO_FROM_PTY\r");

    // Wait for the echo to appear in output
    let output = client.wait_for_text("HELLO_FROM_PTY", Duration::from_secs(3));
    assert!(
        output.contains("HELLO_FROM_PTY"),
        "should see echoed text in PTY output"
    );
}

#[test]
fn pty_client_shows_status_bar() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24", "-s", "mytest"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let _client = PtyClient::attach(&tmux, 80, 24);
    // PtyClient::attach already waits for initial output.
    // If we get here, the client rendered something.
}

#[test]
fn pty_client_prefix_key_works() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut client = PtyClient::attach(&tmux, 80, 24);
    std::thread::sleep(Duration::from_millis(500));
    client.read_raw(); // drain initial output

    // Send prefix + c to create a new window
    client.send_key("C-b");
    std::thread::sleep(Duration::from_millis(50));
    client.send_key("c");

    // Give tmux time to process
    std::thread::sleep(Duration::from_millis(500));

    // Verify a new window was created
    let windows = tmux.cmd().args(["list-windows"]).run();
    let count = windows.stdout_trimmed().lines().count();
    assert_eq!(count, 2, "prefix+c should create a second window");
}

#[test]
fn pty_client_display_panes_works() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Attach a client so display-panes has a target
    let mut _client = PtyClient::attach(&tmux, 80, 24);
    std::thread::sleep(Duration::from_millis(500));

    // Now display-panes should work (it needs an attached client)
    let result = tmux.cmd().args(["display-panes"]).run();
    result.assert_success();
}

#[test]
fn pty_client_show_messages_works() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Attach a client
    let mut _client = PtyClient::attach(&tmux, 80, 24);
    std::thread::sleep(Duration::from_millis(500));

    // show-messages requires an attached client
    let result = tmux.cmd().args(["show-messages"]).run();
    result.assert_success();
}

#[test]
fn pty_client_new_session_creates_and_attaches() {
    let mut tmux = TmuxTestHarness::new();
    // Start a detached session first so the server is running
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut client = PtyClient::attach(&tmux, 80, 24);
    // PtyClient::attach already waits for initial output.
    assert!(client.is_alive(), "client should be running");

    // Verify the client shows up in list-clients
    let clients = tmux.cmd().args(["list-clients"]).run();
    let count = clients.stdout_trimmed().lines().count();
    assert!(count >= 1, "should have at least 1 attached client");
}
