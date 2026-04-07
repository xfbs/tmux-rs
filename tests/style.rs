//! Style trimming tests, ported from regress/style-trim.sh
//!
//! The original script tests `#{=N:V}` and `#{=-N:V}` length-trimming on
//! variables that contain `#` escape characters and `#[...]` style markers.
//! It runs against a real attached client to verify the rendered output.
//!
//! We test only the format-string side here (display-message -p), not the
//! drawn pane output. The original's "drawn" assertions require an attached
//! client and are covered indirectly by the format trim values.

use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

fn check(tmux: &TmuxTestHarness, fmt: &str, expected_value: &str) {
    let v = tmux.query(fmt);
    assert_eq!(v, expected_value, "format: {fmt}");
}

// Ported from regress/style-trim.sh (format-side only)
#[test]
fn style_trim_format_strings() {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session().run().assert_success();
    tmux.wait_ready(Duration::from_secs(5));

    // V = '#0'
    tmux.cmd().args(["setenv", "-g", "V", "#0"]).run().assert_success();
    check(&tmux, "#{V} #{w:V}", "#0 2");
    check(&tmux, "#{=3:V}", "#0");
    check(&tmux, "#{=-3:V}", "#0");

    // V = '###[bg=yellow]0'
    tmux.cmd().args(["setenv", "-g", "V", "###[bg=yellow]0"]).run().assert_success();
    check(&tmux, "#{V} #{w:V}", "###[bg=yellow]0 2");
    check(&tmux, "#{=3:V}", "###[bg=yellow]0");
    check(&tmux, "#{=-3:V}", "###[bg=yellow]0");

    // V = '#0123456'
    tmux.cmd().args(["setenv", "-g", "V", "#0123456"]).run().assert_success();
    check(&tmux, "#{V} #{w:V}", "#0123456 8");
    check(&tmux, "#{=3:V}", "#01");
    check(&tmux, "#{=-3:V}", "456");

    // V = '##0123456'
    tmux.cmd().args(["setenv", "-g", "V", "##0123456"]).run().assert_success();
    check(&tmux, "#{V} #{w:V}", "##0123456 8");
    check(&tmux, "#{=3:V}", "##01");
    check(&tmux, "#{=-3:V}", "456");

    // V = '###0123456'
    tmux.cmd().args(["setenv", "-g", "V", "###0123456"]).run().assert_success();
    check(&tmux, "#{V} #{w:V}", "###0123456 9");
    check(&tmux, "#{=3:V}", "####0");
    check(&tmux, "#{=-3:V}", "456");

    // V = '#[bg=yellow]0123456'
    tmux.cmd().args(["setenv", "-g", "V", "#[bg=yellow]0123456"]).run().assert_success();
    check(&tmux, "#{V} #{w:V}", "#[bg=yellow]0123456 7");
    check(&tmux, "#{=3:V}", "#[bg=yellow]012");
    check(&tmux, "#{=-3:V}", "#[bg=yellow]456");
}
