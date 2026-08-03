use bevy::{
    ecs::system::SystemState,
    prelude::*,
    ui::{Display, Node},
};

use super::{
    super::{Segment, SegmentIndex, set_segment_visible},
    support::{segments_of, spawn_fire_mode},
};
use crate::widgets::core::test_support::harness;

type SegmentVisibilitySet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, (&'static SegmentIndex, &'static mut Node), With<Segment>>,
);

fn drive_set_segment_visible(app: &mut App, control: Entity, index: usize, visible: bool) -> bool {
    let mut state: SystemState<SegmentVisibilitySet> = SystemState::new(app.world_mut());
    let Ok((children, mut segments)) = state.get_mut(app.world_mut()) else {
        return false;
    };
    let ok = set_segment_visible(control, index, visible, &children, &mut segments);
    state.apply(app.world_mut());
    ok
}

fn segment_display(app: &App, segment: Entity) -> Option<Display> {
    app.world().get::<Node>(segment).map(|n| n.display)
}

#[test]
fn set_segment_visible_hides_one_segment_keeping_stable_ids() {
    let mut app = harness();
    let control = spawn_fire_mode(&mut app);
    app.update();

    let before = segments_of(&mut app, control);
    assert_eq!(before.len(), 3, "must have 3 segments");
    let seg0 = before[0].0;
    let seg1 = before[1].0;
    let seg2 = before[2].0;

    for seg in [seg0, seg1, seg2] {
        assert_eq!(
            segment_display(&app, seg),
            Some(Display::Flex),
            "every segment starts visible (Display::Flex)",
        );
    }

    assert!(
        drive_set_segment_visible(&mut app, control, 2, false),
        "hiding an in-range segment must report a match",
    );
    assert_eq!(
        segment_display(&app, seg2),
        Some(Display::None),
        "the hidden segment collapses to Display::None",
    );
    assert_eq!(
        segment_display(&app, seg0),
        Some(Display::Flex),
        "segment 0 stays visible when a sibling is hidden",
    );
    assert_eq!(
        segment_display(&app, seg1),
        Some(Display::Flex),
        "segment 1 stays visible when a sibling is hidden",
    );

    assert!(drive_set_segment_visible(&mut app, control, 2, true));
    assert_eq!(
        segment_display(&app, seg2),
        Some(Display::Flex),
        "re-showing the segment returns it to Display::Flex",
    );

    let after = segments_of(&mut app, control);
    assert_eq!(
        after.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        before.iter().map(|&(e, _)| e).collect::<Vec<_>>(),
        "segment entity ids must be stable across hide/show (mutate, never respawn)",
    );

    assert!(
        !drive_set_segment_visible(&mut app, control, 9, false),
        "an out-of-range index reports no match (no-op)",
    );
}
