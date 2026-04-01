use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

fn setup() -> TmuxTestHarness {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // Set up session name and user options used by conditional tests
    tmux.cmd()
        .args(["rename-session", "Summer"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "@true", "1"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "@false", "0"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "@warm", "Summer"])
        .run()
        .assert_success();
    tmux.cmd()
        .args(["set", "@cold", "Winter"])
        .run()
        .assert_success();
    tmux
}

fn test_format(tmux: &TmuxTestHarness, format: &str, expected: &str) {
    let actual = tmux.query(format);
    assert_eq!(
        actual, expected,
        "Format test failed for '{format}'.\nExpected: '{expected}'\nBut got:  '{actual}'"
    );
}

fn test_conditional_with_pane_in_mode(
    tmux: &TmuxTestHarness,
    format: &str,
    exp_true: &str,
    exp_false: &str,
) {
    tmux.cmd().args(["copy-mode"]).run().assert_success();
    test_format(tmux, format, exp_true);
    tmux.send_keys(&["-X", "cancel"]).assert_success();
    test_format(tmux, format, exp_false);
}

fn test_conditional_with_session_name(
    tmux: &TmuxTestHarness,
    format: &str,
    exp_summer: &str,
    exp_winter: &str,
) {
    tmux.cmd()
        .args(["rename-session", "Summer"])
        .run()
        .assert_success();
    test_format(tmux, format, exp_summer);
    tmux.cmd()
        .args(["rename-session", "Winter"])
        .run()
        .assert_success();
    test_format(tmux, format, exp_winter);
    tmux.cmd()
        .args(["rename-session", "Summer"])
        .run()
        .assert_success();
}

// Ported from regress/format-strings.sh

#[test]
fn format_plain_string() {
    let tmux = setup();
    test_format(&tmux, "abc xyz", "abc xyz");
}

#[test]
fn format_basic_escapes() {
    let tmux = setup();
    test_format(&tmux, "##", "#");
    test_format(&tmux, "#,", ",");
    test_format(&tmux, "{", "{");
    test_format(&tmux, "##{", "#{");
    test_format(&tmux, "#}", "}");
    test_format(&tmux, "###}", "#}");
}

#[test]
fn format_simple_expansion() {
    let tmux = setup();
    test_format(&tmux, "#{pane_in_mode}", "0");
    test_format(&tmux, "#{session_name}", "Summer");
    test_format(&tmux, "#{window_width}", "80");
    test_format(&tmux, "#{window_height}", "24");
}

#[test]
fn format_simple_conditionals() {
    let tmux = setup();
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,xyz}", "abc", "xyz");
}

#[test]
fn format_expansion_in_conditionals() {
    let tmux = setup();
    test_conditional_with_pane_in_mode(
        &tmux,
        "#{?#{pane_in_mode},#{@warm},#{@cold}}",
        "Summer",
        "Winter",
    );
}

#[test]
fn format_basic_escapes_in_conditionals() {
    let tmux = setup();
    // Value of an if-condition
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,##,xyz}", "#", "xyz");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,#,,xyz}", ",", "xyz");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,{,xyz}", "{", "xyz");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,##{,xyz}", "#{", "xyz");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,#},xyz}", "}", "xyz");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,###},xyz}", "#}", "xyz");

    // Default value
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,##}", "abc", "#");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,#,}", "abc", ",");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,{}", "abc", "{");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,##{}", "abc", "#{");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,#}}", "abc", "}");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,abc,###}}", "abc", "#}");

    // Mixed
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,{,#}}", "{", "}");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,#},{}", "}", "{");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,##{,###}}", "#{", "#}");
    test_conditional_with_pane_in_mode(&tmux, "#{?pane_in_mode,###},##{}", "#}", "#{");
}

#[test]
fn format_conditional_with_comparison() {
    let tmux = setup();
    test_conditional_with_session_name(
        &tmux,
        "#{?#{==:#{session_name},Summer},abc,xyz}",
        "abc",
        "xyz",
    );
}

#[test]
fn format_conditional_in_conditional() {
    let tmux = setup();
    test_conditional_with_pane_in_mode(
        &tmux,
        "#{?pane_in_mode,#{?#{==:#{session_name},Summer},ABC,XYZ},xyz}",
        "ABC",
        "xyz",
    );
    test_conditional_with_pane_in_mode(
        &tmux,
        "#{?pane_in_mode,abc,#{?#{==:#{session_name},Summer},ABC,XYZ}}",
        "abc",
        "ABC",
    );
}

#[test]
fn format_logical_and() {
    let tmux = setup();
    test_format(&tmux, "#{&&:0,0}", "0");
    test_format(&tmux, "#{&&:0,1}", "0");
    test_format(&tmux, "#{&&:1,0}", "0");
    test_format(&tmux, "#{&&:1,1}", "1");
}

#[test]
fn format_logical_or() {
    let tmux = setup();
    test_format(&tmux, "#{||:0,0}", "0");
    test_format(&tmux, "#{||:0,1}", "1");
    test_format(&tmux, "#{||:1,0}", "1");
    test_format(&tmux, "#{||:1,1}", "1");
}
