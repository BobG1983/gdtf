use bevy::{
    ecs::system::SystemState,
    prelude::*,
    ui::{BackgroundColor, Node, Val},
};

use super::{FillFraction, ProgressBarFill, set_progress_bar, spawn_progress_bar};
use crate::widgets::core::test_support::{LOST, REMAINING, harness};

type ProgressBarSet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static mut Node, With<ProgressBarFill>>,
);

fn fill_of(app: &mut App, track: Entity) -> Option<Entity> {
    let mut state: SystemState<Query<&Children>> = SystemState::new(app.world_mut());
    let Ok(children) = state.get(app.world()) else {
        return None;
    };
    children
        .get(track)
        .ok()?
        .iter()
        .find(|&child| app.world().get::<ProgressBarFill>(child).is_some())
}

fn drive_set_progress_bar(app: &mut App, track: Entity, fraction: FillFraction) -> bool {
    let mut state: SystemState<ProgressBarSet> = SystemState::new(app.world_mut());
    let Ok((children, mut fills)) = state.get_mut(app.world_mut()) else {
        return false;
    };
    let ok = set_progress_bar(track, fraction, &children, &mut fills);
    state.apply(app.world_mut());
    ok
}

#[test]
fn progress_bar_renders_at_fraction() {
    let mut app = harness();
    let track = {
        let mut commands = app.world_mut().commands();
        spawn_progress_bar(
            &mut commands,
            FillFraction::from_ratio(20.0, 60.0),
            REMAINING,
            LOST,
            (),
        )
    };
    app.world_mut().flush();

    assert_eq!(
        app.world().get::<BackgroundColor>(track).map(|c| c.0),
        Some(LOST),
        "the track must carry the lost color",
    );
    let maybe_fill = fill_of(&mut app, track);
    assert!(maybe_fill.is_some(), "bar must have a fill child");
    let Some(fill) = maybe_fill else { return };
    assert_eq!(
        app.world().get::<BackgroundColor>(fill).map(|c| c.0),
        Some(REMAINING),
        "the fill must carry the remaining color",
    );
    assert_eq!(
        app.world().get::<Node>(fill).map(|n| n.width),
        Some(Val::Percent(20.0 / 60.0 * 100.0)),
        "the fill width must be the value fraction as a percent",
    );
}

#[test]
fn progress_bar_update_mutates_same_fill_entity() {
    let mut app = harness();
    let track = {
        let mut commands = app.world_mut().commands();
        spawn_progress_bar(&mut commands, FillFraction::new(1.0), REMAINING, LOST, ())
    };
    app.world_mut().flush();

    let maybe_before = fill_of(&mut app, track);
    assert!(maybe_before.is_some(), "bar must have a fill child");
    let Some(fill_before) = maybe_before else {
        return;
    };
    assert_eq!(
        app.world().get::<Node>(fill_before).map(|n| n.width),
        Some(Val::Percent(100.0)),
        "precondition: starts full",
    );

    let ok = drive_set_progress_bar(&mut app, track, FillFraction::from_ratio(1.0, 4.0));
    assert!(ok, "set_progress_bar must find the fill child");

    let maybe_after = fill_of(&mut app, track);
    assert!(maybe_after.is_some(), "bar must still have a fill child");
    let Some(fill_after) = maybe_after else {
        return;
    };
    assert_eq!(
        fill_after, fill_before,
        "the fill entity id must be STABLE across the update (mutate, not respawn)",
    );
    assert_eq!(
        app.world().get::<Node>(fill_after).map(|n| n.width),
        Some(Val::Percent(25.0)),
        "the fill width must reflect the updated fraction",
    );
}
