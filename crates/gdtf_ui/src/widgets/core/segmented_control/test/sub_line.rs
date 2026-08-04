use bevy::{
    ecs::system::SystemState,
    prelude::*,
    text::{FontSize, FontWeight, TextColor as UiTextColor, TextFont},
};

use super::{
    super::{
        Segment, SegmentColors, SegmentIndex, SegmentSubLabel, SegmentSubText, SegmentText,
        set_segment_sub_line,
    },
    support::{SEG_COLORS, segment_look, segments_of, spawn_fire_mode},
};
use crate::widgets::core::test_support::harness;

fn sub_line_of(app: &mut App, segment: Entity) -> Option<(Entity, String, f32, Color)> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    let node = children.ok().and_then(|children| {
        children.get(segment).ok().and_then(|kids| {
            kids.iter()
                .find(|&c| app.world().get::<SegmentSubText>(c).is_some())
        })
    })?;
    let content = app.world().get::<Text>(node).map(|t| t.0.clone())?;
    let size = app
        .world()
        .get::<TextFont>(node)
        .map_or(f32::NAN, |f| match f.font_size {
            FontSize::Px(px) => px,
            _ => f32::NAN,
        });
    let color = app
        .world()
        .get::<UiTextColor>(node)
        .map_or(Color::NONE, |c| c.0);
    Some((node, content, size, color))
}

fn label_font_size(app: &mut App, segment: Entity) -> Option<f32> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let children = state.get(app.world());
    let label = children.ok().and_then(|children| {
        children.get(segment).ok().and_then(|kids| {
            kids.iter()
                .find(|&c| app.world().get::<SegmentText>(c).is_some())
        })
    })?;
    app.world()
        .get::<TextFont>(label)
        .and_then(|f| match f.font_size {
            FontSize::Px(px) => Some(px),
            _ => None,
        })
}

type SubLineSet = (
    Commands<'static, 'static>,
    Query<'static, 'static, (&'static Children, &'static SegmentColors)>,
    Query<'static, 'static, (&'static SegmentIndex, &'static Children), With<Segment>>,
    Query<'static, 'static, &'static mut Text, With<SegmentSubText>>,
);

fn drive_set_segment_sub_line(
    app: &mut App,
    control: Entity,
    index: usize,
    sub_line: Option<SegmentSubLabel>,
) -> bool {
    let mut state: SystemState<SubLineSet> = SystemState::new(app.world_mut());
    let Ok((mut commands, controls, segments, mut sub_texts)) = state.get_mut(app.world_mut())
    else {
        return false;
    };
    let ok = set_segment_sub_line(
        &mut commands,
        control,
        index,
        sub_line.as_ref(),
        &controls,
        &segments,
        &mut sub_texts,
    );
    state.apply(app.world_mut());
    app.update();
    ok
}

#[test]
fn segment_without_sub_line_has_only_the_label() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs = segments_of(&mut app, control);
    assert_eq!(segs.len(), 3, "must have 3 segments");
    for (seg, _) in segs {
        let (_, weight, _) = segment_look(&mut app, seg);
        assert!(
            matches!(weight, FontWeight::NORMAL | FontWeight::BOLD),
            "every segment has a label text child",
        );
        assert!(
            sub_line_of(&mut app, seg).is_none(),
            "a segment spawned without a sub-line has no SegmentSubText node",
        );
    }
}

#[test]
fn segment_with_sub_line_has_both_texts_smaller_and_dimmer() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segments = segments_of(&mut app, control);
    let burst = segments[1].0;

    assert!(
        drive_set_segment_sub_line(&mut app, control, 1, Some(SegmentSubLabel::new("3 TU")),),
        "setting an in-range segment's sub-line reports a match",
    );

    let label_size = label_font_size(&mut app, burst);
    assert_eq!(
        label_size,
        Some(16.0),
        "the LABEL keeps its own (larger) font size",
    );

    let sub = sub_line_of(&mut app, burst);
    assert!(sub.is_some(), "the segment now has a SegmentSubText node");
    let Some((_, content, size, color)) = sub else {
        return;
    };
    assert_eq!(content, "3 TU", "the sub-line carries the caption");
    assert!(
        size < 16.0,
        "the sub-line font ({size}) is SMALLER than the 16pt label",
    );
    assert!(
        color.alpha() < SEG_COLORS.base_text.alpha(),
        "the sub-line color ({:?}) is DIMMER (lower alpha) than the base text ({:?})",
        color,
        SEG_COLORS.base_text,
    );
}

#[test]
fn segment_sub_line_set_update_clear_mutates_in_place() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let segs_before = segments_of(&mut app, control);
    let seg0 = segs_before[0].0;

    assert!(drive_set_segment_sub_line(
        &mut app,
        control,
        0,
        Some(SegmentSubLabel::new("3 TU")),
    ));
    let first = sub_line_of(&mut app, seg0);
    assert!(first.is_some(), "the sub-line node was spawned");
    let Some((node_before, text_before, ..)) = first else {
        return;
    };
    assert_eq!(text_before, "3 TU");

    assert!(drive_set_segment_sub_line(
        &mut app,
        control,
        0,
        Some(SegmentSubLabel::new("5 TU")),
    ));
    let updated = sub_line_of(&mut app, seg0);
    let Some((node_after, text_after, ..)) = updated else {
        unreachable!("the sub-line node still exists after an update");
    };
    assert_eq!(
        node_after, node_before,
        "the sub-line node id is STABLE across an update (mutate, not respawn)",
    );
    assert_eq!(text_after, "5 TU", "the sub-line text is updated in place");

    assert!(drive_set_segment_sub_line(&mut app, control, 0, None));
    assert!(
        sub_line_of(&mut app, seg0).is_none(),
        "clearing the sub-line despawns its node",
    );

    let segs_after = segments_of(&mut app, control);
    assert_eq!(
        segs_after.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        segs_before.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        "segment entity ids are stable across the sub-line lifecycle",
    );

    assert!(
        !drive_set_segment_sub_line(&mut app, control, 9, Some(SegmentSubLabel::new("x"))),
        "an out-of-range index reports no match (no-op)",
    );
}
