use super::harness;

use std::time::Duration;

use harness::TmuxTestHarness;

/// Parsed pane info from `list-panes`.
#[derive(Debug)]
struct PaneInfo {
    id: String,
    width: u32,
    height: u32,
    left: u32,
    top: u32,
}

/// Query pane geometry for all panes in the current window.
fn list_panes(tmux: &TmuxTestHarness) -> Vec<PaneInfo> {
    let result = tmux
        .cmd()
        .args([
            "list-panes",
            "-F",
            "#{pane_id} #{pane_width} #{pane_height} #{pane_left} #{pane_top}",
        ])
        .run();
    result.assert_success();
    result
        .stdout_trimmed()
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert!(
                parts.len() >= 5,
                "unexpected list-panes output: {line}"
            );
            PaneInfo {
                id: parts[0].to_string(),
                width: parts[1].parse().unwrap(),
                height: parts[2].parse().unwrap(),
                left: parts[3].parse().unwrap(),
                top: parts[4].parse().unwrap(),
            }
        })
        .collect()
}

/// Create a harness with a session of the given size and the given number of panes.
fn setup(cols: u32, rows: u32, num_panes: u32) -> TmuxTestHarness {
    let mut tmux = TmuxTestHarness::new();
    tmux.new_session()
        .args(["-x", &cols.to_string(), "-y", &rows.to_string()])
        .run()
        .assert_success();
    tmux.wait_ready(Duration::from_secs(5));
    for _ in 1..num_panes {
        tmux.cmd().args(["split-window"]).run().assert_success();
    }
    tmux
}

// ---------------------------------------------------------------------------
// 1. even-horizontal
// ---------------------------------------------------------------------------
#[test]
fn layout_even_horizontal() {
    let tmux = setup(200, 50, 4);
    tmux.cmd()
        .args(["select-layout", "even-horizontal"])
        .run()
        .assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 4);

    // All panes should have equal width (within 1 cell due to rounding + separators).
    let widths: Vec<u32> = panes.iter().map(|p| p.width).collect();
    let min = *widths.iter().min().unwrap();
    let max = *widths.iter().max().unwrap();
    assert!(
        max - min <= 1,
        "even-horizontal widths should be equal (within 1), got {widths:?}"
    );

    // All panes should have the same height (full window height minus status line).
    let heights: Vec<u32> = panes.iter().map(|p| p.height).collect();
    let h0 = heights[0];
    for h in &heights {
        assert_eq!(*h, h0, "even-horizontal heights should all match, got {heights:?}");
    }

    // Panes should be side by side (increasing left coordinate).
    for i in 1..panes.len() {
        assert!(
            panes[i].left > panes[i - 1].left,
            "panes should be ordered left-to-right"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. even-vertical
// ---------------------------------------------------------------------------
#[test]
fn layout_even_vertical() {
    let tmux = setup(200, 50, 4);
    tmux.cmd()
        .args(["select-layout", "even-vertical"])
        .run()
        .assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 4);

    // All panes should have roughly equal height. With 4 panes and borders,
    // some rounding differences are expected. Allow up to 4 difference.
    let heights: Vec<u32> = panes.iter().map(|p| p.height).collect();
    let min = *heights.iter().min().unwrap();
    let max = *heights.iter().max().unwrap();
    assert!(
        max - min <= 4,
        "even-vertical heights should be roughly equal, got {heights:?}"
    );

    // All panes should have the same width.
    let widths: Vec<u32> = panes.iter().map(|p| p.width).collect();
    let w0 = widths[0];
    for w in &widths {
        assert_eq!(*w, w0, "even-vertical widths should all match, got {widths:?}");
    }

    // Panes should be stacked (increasing top coordinate).
    for i in 1..panes.len() {
        assert!(
            panes[i].top > panes[i - 1].top,
            "panes should be ordered top-to-bottom"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. main-horizontal
// ---------------------------------------------------------------------------
#[test]
fn layout_main_horizontal() {
    let tmux = setup(200, 50, 3);
    tmux.cmd()
        .args(["select-layout", "main-horizontal"])
        .run()
        .assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 3);

    // In main-horizontal: first pane spans full width at top,
    // remaining panes are arranged side-by-side below it.
    // The first pane should span the full width.
    assert_eq!(
        panes[0].width, 200,
        "main-horizontal: first pane should be full width"
    );

    // The non-main panes should be arranged horizontally (same top position).
    assert_eq!(
        panes[1].top, panes[2].top,
        "main-horizontal: non-main panes should share the same row"
    );
}

// ---------------------------------------------------------------------------
// 4. main-vertical
// ---------------------------------------------------------------------------
#[test]
fn layout_main_vertical() {
    let tmux = setup(200, 50, 3);
    tmux.cmd()
        .args(["select-layout", "main-vertical"])
        .run()
        .assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 3);

    // In main-vertical: first pane spans full height on the left,
    // remaining panes are arranged vertically on the right.
    // First pane should span the full height.
    assert_eq!(
        panes[0].height, 50,
        "main-vertical: first pane should be full height"
    );

    // Non-main panes should be stacked (same left position, different top).
    assert_eq!(
        panes[1].left, panes[2].left,
        "main-vertical: non-main panes should share the same column"
    );

    // First pane should span the full height.
    let full_height = panes[0].height;
    // Non-main panes should have equal heights (within 1).
    let sub_heights: Vec<u32> = panes[1..].iter().map(|p| p.height).collect();
    let min = *sub_heights.iter().min().unwrap();
    let max = *sub_heights.iter().max().unwrap();
    assert!(
        max - min <= 1,
        "main-vertical: non-main pane heights should be equal (within 1), got {sub_heights:?}"
    );
    // Non-main panes should be shorter than the main pane.
    assert!(
        full_height > sub_heights[0],
        "main-vertical: main pane height ({full_height}) should be larger than sub pane ({})",
        sub_heights[0]
    );
}

// ---------------------------------------------------------------------------
// 5. tiled with 5 and 6 panes
// ---------------------------------------------------------------------------
#[test]
fn layout_tiled_5_panes() {
    let tmux = setup(200, 200, 5);
    tmux.cmd()
        .args(["select-layout", "tiled"])
        .run()
        .assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 5);

    // In a tiled layout, no pane should be excessively large or small.
    let widths: Vec<u32> = panes.iter().map(|p| p.width).collect();
    let heights: Vec<u32> = panes.iter().map(|p| p.height).collect();
    let max_w = *widths.iter().max().unwrap();
    let min_w = *widths.iter().min().unwrap();
    let max_h = *heights.iter().max().unwrap();
    let min_h = *heights.iter().min().unwrap();
    // Width ratio should be reasonable (max is at most 2x min + 1 for rounding).
    assert!(
        max_w <= min_w * 2 + 2,
        "tiled 5: width spread too large: {widths:?}"
    );
    assert!(
        max_h <= min_h * 2 + 2,
        "tiled 5: height spread too large: {heights:?}"
    );
}

#[test]
fn layout_tiled_6_panes() {
    let tmux = setup(200, 200, 6);
    tmux.cmd()
        .args(["select-layout", "tiled"])
        .run()
        .assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 6);

    // With 6 panes in a 200x200 window, tiled should produce a roughly 3x2 or 2x3 Grid.
    // Check that no two panes overlap.
    for i in 0..panes.len() {
        for j in (i + 1)..panes.len() {
            let a = &panes[i];
            let b = &panes[j];
            let a_right = a.left + a.width;
            let a_bottom = a.top + a.height;
            let b_right = b.left + b.width;
            let b_bottom = b.top + b.height;
            let overlap_x = a.left < b_right && b.left < a_right;
            let overlap_y = a.top < b_bottom && b.top < a_bottom;
            assert!(
                !(overlap_x && overlap_y),
                "tiled 6: panes {} and {} overlap: {:?} vs {:?}",
                a.id,
                b.id,
                (a.left, a.top, a.width, a.height),
                (b.left, b.top, b.width, b.height),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Custom layout string
// ---------------------------------------------------------------------------
#[test]
fn layout_custom_string() {
    let tmux = setup(200, 50, 1);
    // Split to get 2 panes, then apply a custom layout.
    tmux.cmd().args(["split-window", "-h"]).run().assert_success();

    // Get the current layout string, then re-apply it.
    let layout_str = tmux.query("#{window_layout}");
    assert!(!layout_str.is_empty(), "layout string should not be empty");

    // Apply the same layout string back.
    tmux.cmd()
        .args(["select-layout", &layout_str])
        .run()
        .assert_success();

    // Verify panes still exist with the same geometry.
    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 2);
}

// ---------------------------------------------------------------------------
// 7. Layout after resize
// ---------------------------------------------------------------------------
#[test]
fn layout_after_resize() {
    let tmux = setup(200, 50, 2);
    tmux.cmd()
        .args(["select-layout", "even-horizontal"])
        .run()
        .assert_success();

    let panes_before = list_panes(&tmux);
    assert_eq!(panes_before.len(), 2);
    let w0_before = panes_before[0].width;

    // Resize the first pane wider.
    tmux.cmd()
        .args(["resize-pane", "-t", &panes_before[0].id, "-R", "20"])
        .run()
        .assert_success();

    let panes_after = list_panes(&tmux);
    assert_eq!(panes_after.len(), 2);
    let w0_after = panes_after[0].width;
    let w1_after = panes_after[1].width;

    // First pane should be wider now.
    assert!(
        w0_after > w0_before,
        "after resize-pane -R 20, first pane should be wider: before={w0_before}, after={w0_after}"
    );

    // Total width should still add up (200 minus 1 separator).
    assert_eq!(
        w0_after + w1_after + 1,
        200,
        "total width should be 200 (with 1-char separator)"
    );
}

// ---------------------------------------------------------------------------
// 8. Nested splits
// ---------------------------------------------------------------------------
#[test]
fn layout_nested_splits() {
    let tmux = setup(200, 50, 1);

    // Split horizontally to get 2 side-by-side panes.
    tmux.cmd().args(["split-window", "-h"]).run().assert_success();

    // Now split the right pane vertically.
    tmux.cmd().args(["split-window", "-v"]).run().assert_success();

    let panes = list_panes(&tmux);
    assert_eq!(panes.len(), 3, "should have 3 panes after nested splits");

    // Pane 0 should be on the left (left=0), panes 1 and 2 on the right.
    assert_eq!(panes[0].left, 0, "first pane should be at left=0");
    assert!(
        panes[1].left > 0 && panes[2].left > 0,
        "right-side panes should have left > 0"
    );

    // The two right panes should be stacked vertically (same left, different top).
    assert_eq!(
        panes[1].left, panes[2].left,
        "right-side panes should have same left coordinate"
    );
    assert_ne!(
        panes[1].top, panes[2].top,
        "right-side panes should have different top coordinates"
    );
}

// ---------------------------------------------------------------------------
// 9. Layout persistence (save and restore)
// ---------------------------------------------------------------------------
#[test]
fn layout_persistence() {
    let tmux = setup(200, 50, 4);
    tmux.cmd()
        .args(["select-layout", "main-vertical"])
        .run()
        .assert_success();

    // Save the layout string.
    let saved_layout = tmux.query("#{window_layout}");
    assert!(!saved_layout.is_empty());

    // Mess up the layout by switching to tiled.
    tmux.cmd()
        .args(["select-layout", "tiled"])
        .run()
        .assert_success();

    let tiled_layout = tmux.query("#{window_layout}");
    // Tiled layout should differ from main-vertical.
    assert_ne!(
        saved_layout, tiled_layout,
        "tiled layout should differ from main-vertical"
    );

    // Restore the saved layout.
    tmux.cmd()
        .args(["select-layout", &saved_layout])
        .run()
        .assert_success();

    let restored_layout = tmux.query("#{window_layout}");
    assert_eq!(
        saved_layout, restored_layout,
        "restored layout should match saved layout"
    );
}
