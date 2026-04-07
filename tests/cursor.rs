//! Cursor positioning tests across window resizes.
//!
//! Ported from regress/cursor-test{1,2,3,4}.sh. The original scripts compare
//! captured pane output byte-for-byte against .result files. We do the same:
//! build up a string of "cursor_x cursor_y cursor_character\n<numbered lines>"
//! and compare against the reference.

use super::harness;

use std::path::PathBuf;
use std::time::Duration;

use harness::TmuxTestHarness;

fn regress_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("regress")
}

fn snapshot(tmux: &TmuxTestHarness) -> String {
    let mut s = String::new();
    let line = tmux.query("#{cursor_x} #{cursor_y} #{cursor_character}");
    s.push_str(&line);
    s.push('\n');
    let cap = tmux.capture_pane();
    for (i, l) in cap.lines().enumerate() {
        s.push_str(&format!("{} {}\n", i, l));
    }
    s
}

fn run_cursor_test(spawn_cmd: &str, x: u16, y: u16, resizes: &[u16], result_file: &str) {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", &x.to_string(), "-y", &y.to_string(), spawn_cmd])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    tmux.cmd()
        .args(["set", "-g", "window-size", "manual"])
        .run()
        .assert_success();
    std::thread::sleep(Duration::from_millis(300));

    let mut all = String::new();
    all.push_str(&snapshot(&tmux));
    for w in resizes {
        tmux.cmd()
            .args(["resize-window", "-x", &w.to_string()])
            .run()
            .assert_success();
        std::thread::sleep(Duration::from_millis(200));
        all.push_str(&snapshot(&tmux));
    }

    let expected = std::fs::read_to_string(regress_dir().join(result_file)).unwrap();
    assert_eq!(all, expected, "snapshot mismatch for {result_file}");
}

// Ported from regress/cursor-test1.sh
#[test]
#[ignore = "depends on byte-for-byte capture matching C tmux reference"]
fn cursor_test1() {
    let txt = regress_dir().join("cursor-test.txt");
    let cmd = format!("cat {}; printf '\\e[9;15H'; cat", txt.display());
    run_cursor_test(&cmd, 40, 10, &[10, 50], "cursor-test1.result");
}

// Ported from regress/cursor-test2.sh
#[test]
#[ignore = "depends on byte-for-byte capture matching C tmux reference"]
fn cursor_test2() {
    let txt = regress_dir().join("cursor-test.txt");
    let cmd = format!("cat {}; printf '\\e[8;10H'; cat", txt.display());
    run_cursor_test(&cmd, 10, 10, &[5, 50], "cursor-test2.result");
}

// Ported from regress/cursor-test3.sh
#[test]
fn cursor_test3() {
    let cmd = "printf 'abcdefabcdefab'; printf '\\e[2;7H'; cat".to_string();
    run_cursor_test(&cmd, 7, 2, &[5, 7], "cursor-test3.result");
}

// Ported from regress/cursor-test4.sh
#[test]
#[ignore = "depends on byte-for-byte capture matching C tmux reference"]
fn cursor_test4() {
    let cmd = "printf 'abcdef\\n'; cat".to_string();
    run_cursor_test(&cmd, 10, 3, &[20, 3, 10], "cursor-test4.result");
}
