use super::harness;

use std::path::PathBuf;
use std::time::Duration;

use harness::TmuxTestHarness;

fn regress_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("regress")
}

// Ported from regress/utf8-test.sh
// Plays UTF-8-test.txt into a pane via cat, then captures and compares
// against utf8-test.result.
#[test]
#[ignore = "depends on exact escape-sequence capture matching C tmux output byte-for-byte"]
fn utf8_capture_matches_reference() {
    let dir = regress_dir();
    let input = dir.join("UTF-8-test.txt");
    let expected = std::fs::read_to_string(dir.join("utf8-test.result")).unwrap();

    let tmux = TmuxTestHarness::new();
    tmux.cmd()
        .args([
            "-f/dev/null",
            "set",
            "-g",
            "remain-on-exit",
            "on",
            ";",
            "set",
            "-g",
            "remain-on-exit-format",
            "",
            ";",
            "new",
            "-d",
            "--",
            "cat",
            input.to_str().unwrap(),
        ])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    std::thread::sleep(Duration::from_secs(1));

    let captured = tmux
        .cmd()
        .args(["capture-pane", "-pCeJS-"])
        .run()
        .stdout_str();

    assert_eq!(captured, expected);
}

// Ported from regress/combine-test.sh
// Tests rendering of combining characters; compares capture-pane -pe output.
#[test]
#[ignore = "byte-for-byte capture comparison against C tmux reference"]
fn combining_characters_capture() {
    let dir = regress_dir();
    let expected = std::fs::read_to_string(dir.join("combine-test.result")).unwrap();

    let tmux = TmuxTestHarness::new();
    let script = "\
printf '\\e[H\\e[J'
printf '\\e[3;1H\\316\\233\\e[3;1H\\314\\2120\\n'
printf '\\e[4;1H\\316\\233\\e[4;2H\\314\\2121\\n'
printf '\\e[5;1H👍\\e[5;1H🏻2\\n'
printf '\\e[6;1H👍\\e[6;3H🏻3\\n'
printf '\\e[7;1H👍\\e[7;10H👍\\e[7;3H🏻\\e[7;12H🏻4\\n'
sleep 5
";
    tmux.cmd()
        .args(["-f/dev/null", "new", "-d", script])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    std::thread::sleep(Duration::from_secs(1));

    let captured = tmux
        .cmd()
        .args(["capture-pane", "-pe"])
        .run()
        .stdout_str();
    assert_eq!(captured, expected);
}
