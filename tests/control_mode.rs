use super::harness;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use harness::TmuxTestHarness;

/// Spawn a control-mode client attached to the given harness.
/// Returns the child process with piped stdin/stdout/stderr.
fn spawn_control_client(tmux: &TmuxTestHarness) -> std::process::Child {
    let bin = harness::client_bin();
    Command::new(&bin)
        .args(["-S", tmux.socket_path(), "-C", "attach"])
        .env("TERM", "screen")
        .env_remove("TMUX")
        .env_remove("TMUX_CONF")
        .env_remove("TMUX_TMPDIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn control mode client")
}

/// Read lines from a BufReader until `predicate` returns true for a line,
/// or until `timeout` elapses. Returns all lines collected.
fn read_until<R: std::io::Read>(
    reader: &mut BufReader<R>,
    timeout: Duration,
    predicate: impl Fn(&str) -> bool,
) -> Vec<String> {
    let start = Instant::now();
    let mut lines = Vec::new();
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        // We rely on the child eventually producing output or EOF.
        // Use a thread with a timeout approach.
        match reader.read_line(&mut line_buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line_buf.trim_end_matches('\n').to_string();
                let matched = predicate(&trimmed);
                lines.push(trimmed);
                if matched {
                    break;
                }
            }
            Err(_) => break,
        }
        if start.elapsed() > timeout {
            break;
        }
    }
    lines
}

/// Send a command to the control mode client and collect output lines until
/// a line matching the predicate appears or timeout.
fn send_and_collect(
    stdin: &mut impl Write,
    reader: &mut BufReader<impl std::io::Read>,
    cmd: &str,
    timeout: Duration,
    predicate: impl Fn(&str) -> bool,
) -> Vec<String> {
    writeln!(stdin, "{}", cmd).expect("failed to write command");
    stdin.flush().expect("failed to flush stdin");
    read_until(reader, timeout, predicate)
}

/// Helper to set up a harness with a detached session and a control mode client.
/// Returns (harness, child, stdin, stdout_reader).
fn setup_control() -> (
    TmuxTestHarness,
    std::process::Child,
    std::process::ChildStdin,
    BufReader<std::process::ChildStdout>,
) {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", "80", "-y", "24"])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    let mut child = spawn_control_client(&tmux);
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    (tmux, child, stdin, reader)
}

/// Cleanup helper: send detach and kill child.
fn cleanup(mut stdin: std::process::ChildStdin, mut child: std::process::Child) {
    let _ = writeln!(stdin, "detach");
    let _ = stdin.flush();
    drop(stdin);
    // Give it a moment then kill
    std::thread::sleep(Duration::from_millis(200));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn control_mode_list_sessions() {
    let (tmux, child, mut stdin, mut reader) = setup_control();

    // Wait briefly for initial output from control mode
    let initial = read_until(&mut reader, Duration::from_secs(3), |line| {
        // The initial greeting ends with a %end line or similar.
        // Just consume until we see a %begin/%end pair or timeout.
        line.starts_with("%end")
    });
    assert!(
        !initial.is_empty(),
        "expected initial control mode output"
    );

    // Send list-sessions command
    let lines = send_and_collect(
        &mut stdin,
        &mut reader,
        "list-sessions",
        Duration::from_secs(5),
        |line| line.starts_with("%end"),
    );

    // We should see output containing a session entry (the default session name
    // or "0:") between %begin and %end.
    let output = lines.join("\n");
    assert!(
        output.contains("%begin") && output.contains("%end"),
        "expected %%begin/%%end framing in control mode output, got:\n{}",
        output
    );
    // The session should appear in the output
    assert!(
        lines.iter().any(|l| !l.starts_with('%') && !l.is_empty()),
        "expected session listing in output, got:\n{}",
        output
    );

    cleanup(stdin, child);
    drop(tmux);
}

#[test]
fn control_mode_new_window_notification() {
    let (tmux, child, mut stdin, mut reader) = setup_control();

    // Drain initial output (attach response + session-changed notification)
    let _initial = read_until(&mut reader, Duration::from_secs(3), |line| {
        line.starts_with("%end")
    });

    // Send new-window and wait for the %window-add notification specifically,
    // since it arrives after the command's %begin/%end pair.
    let lines = send_and_collect(
        &mut stdin,
        &mut reader,
        "new-window",
        Duration::from_secs(5),
        |line| line.starts_with("%window-add"),
    );

    let output = lines.join("\n");
    assert!(
        lines.iter().any(|l| l.starts_with("%window-add")),
        "expected %%window-add notification, got:\n{}",
        output
    );

    cleanup(stdin, child);
    drop(tmux);
}

#[test]
fn control_mode_split_window_layout_change() {
    let (tmux, child, mut stdin, mut reader) = setup_control();

    // Drain initial output (attach response + session-changed notification)
    let _initial = read_until(&mut reader, Duration::from_secs(3), |line| {
        line.starts_with("%end")
    });

    // Send split-window and wait for the %layout-change notification,
    // which arrives after the command's %begin/%end pair.
    let lines = send_and_collect(
        &mut stdin,
        &mut reader,
        "split-window",
        Duration::from_secs(5),
        |line| line.starts_with("%layout-change"),
    );

    let output = lines.join("\n");
    assert!(
        lines.iter().any(|l| l.starts_with("%layout-change")),
        "expected %%layout-change notification, got:\n{}",
        output
    );

    cleanup(stdin, child);
    drop(tmux);
}

#[test]
fn control_mode_rename_session() {
    let (tmux, child, mut stdin, mut reader) = setup_control();

    // Drain initial output (attach response + session-changed notification)
    let _initial = read_until(&mut reader, Duration::from_secs(3), |line| {
        line.starts_with("%end")
    });

    // Send rename-session and wait for a line containing "newname",
    // which appears in the %session-renamed or %session-changed notification
    // after the command's %begin/%end pair.
    let lines = send_and_collect(
        &mut stdin,
        &mut reader,
        "rename-session newname",
        Duration::from_secs(5),
        |line| line.contains("newname"),
    );

    let output = lines.join("\n");
    assert!(
        lines.iter().any(|l| l.contains("newname")),
        "expected 'newname' in output, got:\n{}",
        output
    );

    cleanup(stdin, child);
    drop(tmux);
}

#[test]
fn control_mode_multiple_commands() {
    let (tmux, child, mut stdin, mut reader) = setup_control();

    // Drain initial output
    let _initial = read_until(&mut reader, Duration::from_secs(3), |line| {
        line.starts_with("%end")
    });

    // Send first command
    let lines1 = send_and_collect(
        &mut stdin,
        &mut reader,
        "list-sessions",
        Duration::from_secs(5),
        |line| line.starts_with("%end"),
    );
    assert!(
        lines1.iter().any(|l| l.starts_with("%begin")),
        "first command should produce %%begin, got:\n{}",
        lines1.join("\n")
    );

    // Send second command
    let lines2 = send_and_collect(
        &mut stdin,
        &mut reader,
        "list-windows",
        Duration::from_secs(5),
        |line| line.starts_with("%end"),
    );
    assert!(
        lines2.iter().any(|l| l.starts_with("%begin")),
        "second command should produce %%begin, got:\n{}",
        lines2.join("\n")
    );

    // Send third command
    let lines3 = send_and_collect(
        &mut stdin,
        &mut reader,
        "list-panes",
        Duration::from_secs(5),
        |line| line.starts_with("%end"),
    );
    assert!(
        lines3.iter().any(|l| l.starts_with("%begin")),
        "third command should produce %%begin, got:\n{}",
        lines3.join("\n")
    );

    cleanup(stdin, child);
    drop(tmux);
}

#[test]
fn control_mode_display_message() {
    let (tmux, child, mut stdin, mut reader) = setup_control();

    // Drain initial output
    let _initial = read_until(&mut reader, Duration::from_secs(3), |line| {
        line.starts_with("%end")
    });

    // Send display-message to query the session name
    let lines = send_and_collect(
        &mut stdin,
        &mut reader,
        "display-message -p \"#{session_name}\"",
        Duration::from_secs(5),
        |line| line.starts_with("%end"),
    );

    let output = lines.join("\n");
    // Should have %begin/%end framing
    assert!(
        output.contains("%begin") && output.contains("%end"),
        "expected %%begin/%%end framing, got:\n{}",
        output
    );
    // The session name should appear between %begin and %end.
    // Default session name is "0".
    let data_lines: Vec<&String> = lines
        .iter()
        .filter(|l| !l.starts_with('%') && !l.is_empty())
        .collect();
    assert!(
        !data_lines.is_empty(),
        "expected session name in output, got:\n{}",
        output
    );
    // The default session name is typically "0"
    assert!(
        data_lines.iter().any(|l| l.trim() == "0"),
        "expected default session name '0', got data lines: {:?}",
        data_lines
    );

    cleanup(stdin, child);
    drop(tmux);
}
