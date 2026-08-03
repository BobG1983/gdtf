use bevy::{
    app::{App, Update},
    prelude::IntoScheduleConfigs,
};

use super::reset_reactions_used;
use crate::{ganger::Faction, tuning::ReactionsUsed, turn::TurnStarted};

fn reset_app() -> App {
    let mut app = App::new();
    app.add_message::<TurnStarted>();
    app.add_systems(Update, reset_reactions_used.run_if(|| true));
    app
}

fn spawn_used(app: &mut App, used: u32) -> bevy::prelude::Entity {
    app.world_mut().spawn(ReactionsUsed::new(used)).id()
}

fn used_count(app: &App, entity: bevy::prelude::Entity) -> Option<u32> {
    app.world().get::<ReactionsUsed>(entity).map(|u| **u)
}

#[test]
fn a_turn_boundary_zeroes_every_watcher() {
    let mut app = reset_app();
    let a = spawn_used(&mut app, 3);
    let b = spawn_used(&mut app, 1);

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

    app.world_mut()
        .write_message(TurnStarted::new(Faction::new(0)));
    app.world_mut()
        .write_message(TurnStarted::new(Faction::new(1)));
    app.update();

    assert_eq!(used_count(&app, a), Some(0), "the reset is idempotent");
}
