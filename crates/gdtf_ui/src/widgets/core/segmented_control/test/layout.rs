//! Orientation + GTW-277 V2 connected-control geometry tests: Row/Column
//! layout, the one-connected-container reading (no gaps, root-owned corners,
//! leading-edge dividers), and the vertical divider edge.

use bevy::{
    prelude::*,
    ui::{Node, Val},
};

use super::{
    super::{SegmentLabel, spawn_segmented_control},
    support::{SEG_COLORS, segments_of, spawn_fire_mode},
};
use crate::widgets::core::{Orientation, test_support::harness};

/// AC — both Row and Column orientations lay out (the flex-direction differs).
#[test]
fn segmented_control_supports_row_and_column() {
    let mut app = harness();
    let labels = [SegmentLabel::new("A"), SegmentLabel::new("B")];
    let (row, col) = {
        let mut commands = app.world_mut().commands();
        let row = spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Horizontal,
            (),
        );
        let col = spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Vertical,
            (),
        );
        (row, col)
    };
    app.world_mut().flush();

    assert_eq!(
        app.world().get::<Node>(row).map(|n| n.flex_direction),
        Some(FlexDirection::Row),
        "horizontal orientation lays out as a Row",
    );
    assert_eq!(
        app.world().get::<Node>(col).map(|n| n.flex_direction),
        Some(FlexDirection::Column),
        "vertical orientation lays out as a Column",
    );
}

/// GTW-277 (screenshot review V2): a `SegmentedControl` renders as ONE connected
/// container, NOT a stack of detached pills — the segments butt together with NO
/// inter-segment gap, the ROOT carries the rounded corners + clips its children (so the
/// inner segment corners are square and the whole control reads as a single rounded box),
/// and every segment AFTER the first carries a hairline divider on its leading edge (left
/// for a Row), while the first segment has no leading divider (its leading edge is the
/// control's outer edge).
///
/// Pin-discriminating: a non-zero column/row gap (the old detached-pill look), a missing
/// root radius/clip, or a leading divider on segment 0 / a missing divider on segment 1
/// each fails an assert. Asserts layout KIND (gap is zero, divider edge is non-zero), not
/// px magnitudes.
#[test]
fn segmented_control_reads_as_one_connected_control() {
    use bevy::ui::{BorderColor, Overflow};

    let mut app = harness();
    let control = spawn_fire_mode(&mut app);

    // The ROOT: no inter-segment gap, a rounded outer container, clipped children.
    let root_node = app.world().get::<Node>(control).cloned();
    assert!(root_node.is_some(), "control root must have a Node");
    let Some(root) = root_node else { return };
    assert_eq!(
        root.column_gap,
        Val::ZERO,
        "a connected control has NO inter-segment column gap (V2)",
    );
    assert_eq!(
        root.row_gap,
        Val::ZERO,
        "a connected control has NO inter-segment row gap (V2)",
    );
    assert_eq!(
        root.overflow,
        Overflow::clip(),
        "the root clips its children so the inner segment corners stay hidden (V2)",
    );
    assert_ne!(
        root.border_radius.top_left,
        Val::ZERO,
        "the root (the single container) carries the rounded corner (V2)",
    );

    // The segments: butt together (no per-segment rounding), divider on each leading edge
    // EXCEPT the first.
    let segments = segments_of(&mut app, control);
    assert_eq!(segments.len(), 3, "must have 3 segments");
    let first = segments[0].0;
    let second = segments[1].0;

    let first_node = app.world().get::<Node>(first).cloned();
    let second_node = app.world().get::<Node>(second).cloned();
    let (Some(first_n), Some(second_n)) = (first_node, second_node) else {
        return;
    };
    assert_eq!(
        first_n.border_radius.top_left,
        Val::ZERO,
        "segments carry NO per-segment rounding — the root owns the corners (V2)",
    );
    assert_eq!(
        first_n.border.left,
        Val::ZERO,
        "the FIRST segment has no leading divider (its leading edge is the outer edge)",
    );
    assert_ne!(
        second_n.border.left,
        Val::ZERO,
        "every segment after the first carries a leading-edge divider (the connected \
         control's adjacent dividers, V2)",
    );

    // The divider is the base_text color (a subtle hairline), painted once at spawn.
    let second_border = app.world().get::<BorderColor>(second).map(|b| b.left);
    assert_eq!(
        second_border,
        Some(SEG_COLORS.base_text),
        "the divider uses the base_text color",
    );
}

/// GTW-277 (V2): a VERTICAL segmented control's dividers run along the TOP edge (the
/// orientation-correct leading edge), not the left, so a stacked Stand/Kneel/Prone control
/// reads as one connected vertical control with horizontal dividers between rows.
///
/// Pin-discriminating: a vertical control whose divider is on the left (the horizontal
/// edge) instead of the top fails the assert.
#[test]
fn vertical_segmented_control_divides_on_the_top_edge() {
    let mut app = harness();
    let labels = [
        SegmentLabel::new("Stand"),
        SegmentLabel::new("Kneel"),
        SegmentLabel::new("Prone"),
    ];
    let control = {
        let mut commands = app.world_mut().commands();
        spawn_segmented_control(
            &mut commands,
            &labels,
            0,
            SEG_COLORS,
            Orientation::Vertical,
            (),
        )
    };
    app.world_mut().flush();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    let n1 = app.world().get::<Node>(segs[1].0).cloned();
    let Some(node1) = n1 else { return };
    assert_ne!(
        node1.border.top,
        Val::ZERO,
        "a vertical control's divider runs along the TOP (leading) edge of each row (V2)",
    );
    assert_eq!(
        node1.border.left,
        Val::ZERO,
        "a vertical control's divider is NOT on the left/horizontal edge",
    );
}
