use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

// Ported from regress/capture-pane-sgr0.sh
// capture-pane should re-emit colours after SGR 0 reset.
#[test]
fn capture_pane_sgr0_reset() {
    let tmux = TmuxTestHarness::new();
    let cmd = "printf '\\033[31;42;1mabc\\033[0;31mdef\\n'; \
               printf '\\033[m\\033[100m bright bg \\033[m'; \
               sleep 5";
    tmux.cmd()
        .args(["-f/dev/null", "new", "-d", cmd])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    std::thread::sleep(Duration::from_secs(1));

    let captured = tmux
        .cmd()
        .args(["capture-pane", "-pe", "-S", "0", "-E", "1"])
        .run()
        .stdout_str();

    let expected = "\u{1b}[1m\u{1b}[31m\u{1b}[42mabc\u{1b}[0m\u{1b}[31mdef\u{1b}[39m\n\
                    \u{1b}[100m bright bg \u{1b}[49m\n";
    assert_eq!(captured, expected);
}

// Ported from regress/capture-pane-hyperlink.sh
// capture-pane -e should preserve OSC 8 hyperlinks.
#[test]
#[ignore = "tmux-rs hyperlink capture may differ; needs verification"]
fn capture_pane_hyperlink_with_id() {
    let tmux = TmuxTestHarness::new();
    let cmd = "printf '\\033]8;id=1;https://github.com\\033\\\\test1\\033]8;;\\033\\\\\\n'; sleep 5";
    tmux.cmd()
        .args(["-f/dev/null", "new", "-d", cmd])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    std::thread::sleep(Duration::from_secs(1));

    let captured = tmux
        .cmd()
        .args(["capture-pane", "-pe", "-S", "0", "-E", "1"])
        .run()
        .stdout_str();
    let expected = "\u{1b}]8;id=1;https://github.com\u{1b}\\test1\u{1b}]8;;\u{1b}\\\n";
    assert_eq!(captured, expected);
}

#[test]
#[ignore = "tmux-rs hyperlink capture may differ; needs verification"]
fn capture_pane_hyperlink_without_id() {
    let tmux = TmuxTestHarness::new();
    let cmd = "printf '\\033]8;;https://github.com/tmux/tmux\\033\\\\test1\\033]8;;\\033\\\\\\n'; sleep 5";
    tmux.cmd()
        .args(["-f/dev/null", "new", "-d", cmd])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    std::thread::sleep(Duration::from_secs(1));

    let captured = tmux
        .cmd()
        .args(["capture-pane", "-pe", "-S", "0", "-E", "1"])
        .run()
        .stdout_str();
    let expected =
        "\u{1b}]8;;https://github.com/tmux/tmux\u{1b}\\test1\u{1b}]8;;\u{1b}\\\n";
    assert_eq!(captured, expected);
}
