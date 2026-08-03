use bevy::{
    prelude::*,
    ui::{Node, Val},
};

use super::{
    super::{SegmentLabel, spawn_segmented_control},
    support::{SEG_COLORS, segments_of, spawn_fire_mode},
};
use crate::widgets::core::{Orientation, test_support::harness};

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

#[test]
fn segmented_control_reads_as_one_connected_control() {
    use bevy::ui::{BorderColor, Overflow};

    let mut app = harness();
    let control = spawn_fire_mode(&mut app);

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

    let second_border = app.world().get::<BorderColor>(second).map(|b| b.left);
    assert_eq!(
        second_border,
        Some(SEG_COLORS.base_text),
        "the divider uses the base_text color",
    );
}

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
