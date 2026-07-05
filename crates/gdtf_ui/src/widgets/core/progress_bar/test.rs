//! Tests for the `ProgressBar` widget behavior (the `progress_bar.rs` surface).
//!
//! Runs on the shared in-crate harness (see
//! [`test_support`](crate::widgets::core::test_support)). Each test is
//! pin-discriminating: it asserts MUTATE-in-place (stable entity ids across an
//! update) and the fraction → percent width mapping.

use bevy::{
    ecs::system::SystemState,
    prelude::*,
    ui::{BackgroundColor, Node, Val},
};

use super::{FillFraction, ProgressBarFill, set_progress_bar, spawn_progress_bar};
use crate::widgets::core::test_support::{LOST, REMAINING, harness};

/// The two-query [`SystemState`] driving [`set_progress_bar`] in tests (clippy
/// `type_complexity`).
type ProgressBarSet = (
    Query<'static, 'static, &'static Children>,
    Query<'static, 'static, &'static mut Node, With<ProgressBarFill>>,
);

/// The fill child of the bar rooted at `track`, if any.
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

/// Drives [`set_progress_bar`] once against the live world's queries.
fn drive_set_progress_bar(app: &mut App, track: Entity, fraction: FillFraction) -> bool {
    let mut state: SystemState<ProgressBarSet> = SystemState::new(app.world_mut());
    let Ok((children, mut fills)) = state.get_mut(app.world_mut()) else {
        return false;
    };
    let ok = set_progress_bar(track, fraction, &children, &mut fills);
    state.apply(app.world_mut());
    ok
}

/// AC — a `ProgressBar` renders its fill at the requested fraction width, and the
/// `lost` color is on the track while `remaining` is on the fill.
///
/// Pin-discriminating: a wrong fraction → percent mapping, or a swapped track/fill
/// color, fails an assert (the two colors are distinct).
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

/// AC — updating the value MUTATES the SAME fill entity to the new width (a respawn
/// would change the fill entity id).
///
/// Pin-discriminating: capture the fill id, update, re-find the fill, assert the id
/// is unchanged AND the width changed.
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
