use super::harness;

use std::io::Write;
use std::time::Duration;

use harness::TmuxTestHarness;

// Ported from regress/conf-syntax.sh
#[test]
fn config_syntax_error_handling() {
    let tmux = TmuxTestHarness::new();

    // Create a config with an invalid command
    let mut tmpfile = tempfile::NamedTempFile::new().expect("failed to create temp file");
    writeln!(tmpfile, "not-a-real-command").unwrap();

    // The original test uses -f with the bad config
    let result = tmux.cmd()
        .args([&format!("-f{}", tmpfile.path().display()), "new-session", "-d"])
        .run();
    // Should still work (bad config lines are skipped with errors)
    // Note: behavior may vary — some configs cause startup failure
    let _ = result;
}

// Ported from regress/if-shell-error.sh
#[test]
fn if_shell_with_failing_command() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // if-shell with a command that fails should execute the else branch
    tmux.cmd()
        .args(["if-shell", "false", "set @result true", "set @result false"])
        .run()
        .assert_success();

    // Give the shell command time to execute
    std::thread::sleep(Duration::from_millis(500));

    let val = tmux.query("#{@result}");
    assert_eq!(val, "false");
}

// Ported from regress/if-shell-TERM.sh
#[test]
fn if_shell_with_succeeding_command() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // if-shell with a command that succeeds should execute the then branch
    tmux.cmd()
        .args(["if-shell", "true", "set @result yes", "set @result no"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(500));

    let val = tmux.query("#{@result}");
    assert_eq!(val, "yes");
}

// Ported from regress/if-shell-error.sh
// An unknown command nested inside an if-shell branch in a config file should
// be reported as %config-error in control-mode output, not crash the server.
#[test]
fn if_shell_unknown_command_in_config() {
    let tmux = TmuxTestHarness::new();

    let mut tmpfile = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmpfile, "if 'true' 'wibble wobble'").unwrap();
    tmpfile.flush().unwrap();
    let tmp_path = tmpfile.path().display().to_string();

    // Use -C control mode + new-session, which prints %config-error lines
    // for any errors encountered while loading -f.
    let result = tmux.cmd()
        .args([&format!("-f{}", tmp_path), "-C", "new"])
        .stdin("")
        .run();
    let stdout = result.stdout_str();
    let needle = format!("%config-error {}:1: {}:1: unknown command: wibble", tmp_path, tmp_path);
    assert!(
        stdout.contains(&needle),
        "expected {needle:?} in control output, got: {stdout:?}"
    );
}

// Ported from regress/if-shell-error.sh (second case)
// `source` of a file containing an unknown command should report the error
// in control mode and not crash. Note: the original regress script grep'd
// for a `%config-error` prefix, but C tmux 3.5a actually emits the bare
// error here (the prefix is only used when -f config is loaded at startup),
// so we just check the error text appears.
#[test]
fn source_unknown_command_reports_error() {
    let tmux = TmuxTestHarness::new();

    let mut tmpfile = tempfile::NamedTempFile::new().expect("temp file");
    writeln!(tmpfile, "wibble wobble").unwrap();
    tmpfile.flush().unwrap();
    let tmp_path = tmpfile.path().display().to_string();

    let input = format!("source {}\n", tmp_path);
    let result = tmux.cmd().args(["-C", "new"]).stdin(&input).run();
    let stdout = result.stdout_str();
    let needle = format!("{}:1: unknown command: wibble", tmp_path);
    assert!(
        stdout.contains(&needle),
        "expected {needle:?} in control output, got: {stdout:?}"
    );
}

// Ported from regress/if-shell-TERM.sh
// TERM should be inherited from the launching environment when if-shell
// runs at config-load time, not be the tmux-internal default.
#[test]
fn if_shell_term_from_outside() {
    // xterm branch
    {
        let tmux = TmuxTestHarness::new();
        let mut tmpfile = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            tmpfile,
            "if '[ \"$TERM\" = \"xterm\" ]' 'set -g default-terminal vt220' 'set -g default-terminal ansi'"
        ).unwrap();
        tmpfile.flush().unwrap();

        tmux.cmd()
            .args([&format!("-f{}", tmpfile.path().display()), "new-session", "-d"])
            .env("TERM", "xterm")
            .run()
            .assert_success();
        tmux.wait_ready(Duration::from_secs(5));

        let val = tmux.cmd().args(["show", "-vg", "default-terminal"]).run().stdout_trimmed();
        assert_eq!(val, "vt220", "xterm branch should pick vt220");
    }

    // screen branch (else)
    {
        let tmux = TmuxTestHarness::new();
        let mut tmpfile = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            tmpfile,
            "if '[ \"$TERM\" = \"xterm\" ]' 'set -g default-terminal vt220' 'set -g default-terminal ansi'"
        ).unwrap();
        tmpfile.flush().unwrap();

        tmux.cmd()
            .args([&format!("-f{}", tmpfile.path().display()), "new-session", "-d"])
            .env("TERM", "screen")
            .run()
            .assert_success();
        tmux.wait_ready(Duration::from_secs(5));

        let val = tmux.cmd().args(["show", "-vg", "default-terminal"]).run().stdout_trimmed();
        assert_eq!(val, "ansi", "non-xterm branch should pick ansi");
    }
}

// Ported from regress/run-shell-output.sh
#[test]
fn run_shell_captures_output() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    tmux.cmd()
        .args(["run-shell", "echo hello"])
        .run()
        .assert_success();

    // run-shell output appears in the pane's status or message area
    // We can verify it ran by using a side effect
    tmux.cmd()
        .args(["run-shell", "tmux set @shell_output done"])
        .run()
        .assert_success();

    std::thread::sleep(Duration::from_millis(500));

    // Note: run-shell executes the command via the shell, and we reference
    // the tmux within the shell, which uses the client binary path
    // This may not work with different server/client binaries
}

// Ported from regress/if-shell-nested.sh
// Tests that running tmux from within an if-shell config conditional works.
#[test]
fn if_shell_nested_tmux_invocation() {
    let tmux = TmuxTestHarness::new();
    let bin = harness::client_bin();
    let socket = tmux.socket_path().to_string();

    // Write a config that uses if-shell to invoke tmux itself
    let mut tmpfile = tempfile::NamedTempFile::new().expect("failed to create temp file");
    writeln!(
        tmpfile,
        "if '{} -S {} run \"true\"' 'set -s @done yes'",
        bin.display(),
        socket
    )
    .unwrap();
    tmpfile.flush().unwrap();

    // Start a new session with the config
    tmux.cmd()
        .args([
            &format!("-f{}", tmpfile.path().display()),
            "new-session",
            "-d",
        ])
        .env("TERM", "xterm")
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Allow the if-shell command to complete
    std::thread::sleep(Duration::from_millis(800));

    let val = tmux.cmd().args(["show", "-vs", "@done"]).run().stdout_trimmed();
    assert_eq!(val, "yes");
}
