//! In-crate unit tests for the reaction-trigger module's turn-boundary reset
//! ([`reset_reactions_used`]) — the cap-lifecycle behavior in isolation (GTW-468 C6).
//!
//! The full END-TO-END trigger behavior (AC1–AC7) is exercised against the PRODUCTION
//! wiring in `tests/reaction_trigger/` (the `BattleSimPlugin` +
//! `SetupBattleRequested` idiom, driven through the brain/move/dispatch path). These
//! in-crate tests cover only the reset system's contract: a `TurnStarted` zeroes every
//! watcher's [`ReactionsUsed`], and no boundary leaves it untouched.

use bevy::{
    app::{App, Update},
    prelude::IntoScheduleConfigs,
};

use super::reset_reactions_used;
use crate::{ganger::Faction, tuning::ReactionsUsed, turn::TurnStarted};

/// Build a one-system app that runs [`reset_reactions_used`] each update, with the
/// `TurnStarted` buffer registered so the `MessageReader` param validates.
fn reset_app() -> App {
    let mut app = App::new();
    app.add_message::<TurnStarted>();
    app.add_systems(Update, reset_reactions_used.run_if(|| true));
    app
}

/// Spawn a ganger carrying a [`ReactionsUsed`] at `used` and return its entity.
fn spawn_used(app: &mut App, used: u32) -> bevy::prelude::Entity {
    app.world_mut().spawn(ReactionsUsed::new(used)).id()
}

/// The current [`ReactionsUsed`] count of `entity`.
fn used_count(app: &App, entity: bevy::prelude::Entity) -> Option<u32> {
    app.world().get::<ReactionsUsed>(entity).map(|u| **u)
}

#[test]
fn a_turn_boundary_zeroes_every_watcher() {
    let mut app = reset_app();
    let a = spawn_used(&mut app, 3);
    let b = spawn_used(&mut app, 1);

    // A turn started this tick — both counters must reset to 0.
    app.world_mut()
        .write_message(TurnStarted::new(Faction::new(0)));
    app.update();

    assert_eq!(used_count(&app, a), Some(0), "watcher A's counter resets");
    assert_eq!(used_count(&app, b), Some(0), "watcher B's counter resets");
}

#[test]
fn no_boundary_leaves_the_counter_untouched() {
    let mut app = reset_app();
    let a = spawn_used(&mut app, 2);

    // No TurnStarted written — the reset must NOT fire (the counter persists across the
    // turn; it is only zeroed at a boundary, C6).
    app.update();

    assert_eq!(
        used_count(&app, a),
        Some(2),
        "without a turn boundary the per-turn counter persists",
    );
}

#[test]
fn multiple_boundaries_one_tick_still_reset_once() {
    let mut app = reset_app();
    let a = spawn_used(&mut app, 5);

    // Two TurnStarted in one tick (e.g. a double advance): the reset is idempotent — the
    // counter ends at 0 regardless of how many boundaries crossed.
    app.world_mut()
        .write_message(TurnStarted::new(Faction::new(0)));
    app.world_mut()
        .write_message(TurnStarted::new(Faction::new(1)));
    app.update();

    assert_eq!(used_count(&app, a), Some(0), "the reset is idempotent");
}
