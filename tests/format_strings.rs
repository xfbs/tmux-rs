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

// --- String comparison operators ---

#[test]
fn format_string_equal() {
    let tmux = setup();
    test_format(&tmux, "#{==:abc,abc}", "1");
    test_format(&tmux, "#{==:abc,xyz}", "0");
    test_format(&tmux, "#{==:,}", "1");
    test_format(&tmux, "#{==:a,}", "0");
}

#[test]
fn format_string_not_equal() {
    let tmux = setup();
    test_format(&tmux, "#{!=:abc,xyz}", "1");
    test_format(&tmux, "#{!=:abc,abc}", "0");
}

#[test]
fn format_string_less_greater() {
    let tmux = setup();
    test_format(&tmux, "#{<:abc,xyz}", "1");
    test_format(&tmux, "#{<:xyz,abc}", "0");
    test_format(&tmux, "#{>:xyz,abc}", "1");
    test_format(&tmux, "#{>:abc,xyz}", "0");
}

// --- Pattern matching ---

#[test]
fn format_glob_matching() {
    let tmux = setup();
    test_format(&tmux, "#{m:*test*,this is a test}", "1");
    test_format(&tmux, "#{m:*test*,no match here}", "0");
    test_format(&tmux, "#{m:foo*,foobar}", "1");
    test_format(&tmux, "#{m:foo*,barfoo}", "0");
}

#[test]
fn format_regex_matching_case_insensitive() {
    let tmux = setup();
    test_format(&tmux, "#{m/ri:TEST,this is a test}", "1");
    test_format(&tmux, "#{m/ri:NOPE,this is a test}", "0");
}

// --- Arithmetic ---

#[test]
fn format_arithmetic_add() {
    let tmux = setup();
    test_format(&tmux, "#{e|+:3,4}", "7");
    test_format(&tmux, "#{e|+:0,0}", "0");
    test_format(&tmux, "#{e|+:100,200}", "300");
}

#[test]
fn format_arithmetic_subtract() {
    let tmux = setup();
    test_format(&tmux, "#{e|-:10,3}", "7");
    test_format(&tmux, "#{e|-:5,5}", "0");
}

#[test]
fn format_arithmetic_multiply() {
    let tmux = setup();
    test_format(&tmux, "#{e|*:5,6}", "30");
    test_format(&tmux, "#{e|*:0,100}", "0");
}

#[test]
fn format_arithmetic_divide() {
    let tmux = setup();
    test_format(&tmux, "#{e|/:10,3}", "3");
    test_format(&tmux, "#{e|/:15,5}", "3");
}

// --- Variable expansion ---

#[test]
fn format_pane_variables() {
    let tmux = setup();

    // pane_id should start with %
    let pane_id = tmux.query("#{pane_id}");
    assert!(
        pane_id.starts_with('%'),
        "pane_id should start with %, got: {pane_id}"
    );

    // pane_pid should be a number
    let pane_pid = tmux.query("#{pane_pid}");
    assert!(
        pane_pid.parse::<u32>().is_ok(),
        "pane_pid should be numeric, got: {pane_pid}"
    );

    // pane dimensions
    test_format(&tmux, "#{pane_width}", "80");
    test_format(&tmux, "#{pane_height}", "24");
}

#[test]
fn format_window_variables() {
    let tmux = setup();

    // window_id should start with @
    let win_id = tmux.query("#{window_id}");
    assert!(
        win_id.starts_with('@'),
        "window_id should start with @, got: {win_id}"
    );

    test_format(&tmux, "#{window_index}", "0");
    test_format(&tmux, "#{window_width}", "80");
    test_format(&tmux, "#{window_height}", "24");

    // window_panes count
    test_format(&tmux, "#{window_panes}", "1");

    // After split, should be 2
    tmux.cmd().args(["split-window", "-d"]).run().assert_success();
    test_format(&tmux, "#{window_panes}", "2");
}

#[test]
fn format_session_variables() {
    let tmux = setup();

    // session_id should start with $
    let sess_id = tmux.query("#{session_id}");
    assert!(
        sess_id.starts_with('$'),
        "session_id should start with $, got: {sess_id}"
    );

    test_format(&tmux, "#{session_name}", "Summer");
    test_format(&tmux, "#{session_windows}", "1");

    // After new window, should be 2
    tmux.cmd().args(["new-window", "-d"]).run().assert_success();
    test_format(&tmux, "#{session_windows}", "2");
}

#[test]
fn format_cursor_position() {
    let tmux = setup();

    // cursor_x and cursor_y should be numeric
    let cx = tmux.query("#{cursor_x}");
    let cy = tmux.query("#{cursor_y}");
    assert!(cx.parse::<u32>().is_ok(), "cursor_x should be numeric: {cx}");
    assert!(cy.parse::<u32>().is_ok(), "cursor_y should be numeric: {cy}");
}

// --- Nested format expansion ---

#[test]
fn format_nested_comparison_with_variables() {
    let tmux = setup();
    // session_name is "Summer" from setup()
    test_format(&tmux, "#{==:#{session_name},Summer}", "1");
    test_format(&tmux, "#{==:#{session_name},Winter}", "0");
    test_format(&tmux, "#{!=:#{session_name},Winter}", "1");
}

#[test]
fn format_nested_conditional_with_variables() {
    let tmux = setup();
    // If session_name == Summer, return "warm", else "cold"
    test_format(
        &tmux,
        "#{?#{==:#{session_name},Summer},warm,cold}",
        "warm",
    );
    tmux.cmd().args(["rename-session", "Winter"]).run().assert_success();
    test_format(
        &tmux,
        "#{?#{==:#{session_name},Summer},warm,cold}",
        "cold",
    );
}

// --- String length ---

#[test]
fn format_string_length() {
    let tmux = setup();
    // n: returns length — but it operates on the format variable name's value length
    let result = tmux.query("#{n:pane_width}");
    // pane_width is "80", which has length 2
    assert_eq!(result, "2");
}
