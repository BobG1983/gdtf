use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::ThrowGrenadeButton;
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::ThrowGrenadeRequested,
    prelude::{Cell, CellLevel, Faction, Level},
    weapon::{TrajectoryStyle, WieldedBy},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};


fn spawn_throw_actor(
    app: &mut App,
    x: i32,
    y: i32,
    gang: u8,
    trajectory: TrajectoryStyle,
) -> Entity {
    let actor = app.world_mut().spawn((at(x, y), Faction::new(gang))).id();
    app.world_mut().spawn((WieldedBy::new(actor), trajectory));
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

fn hover_cell(app: &mut App, x: i32, y: i32) {
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(CellLevel::new(
            Cell::new(x, y),
            Level::new(0),
        ))));
}

fn throw_visible(app: &mut App) -> bool {
    visibility::<ThrowGrenadeButton>(app) == Some(Visibility::Visible)
}

fn add_throw_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ThrowGrenadeRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ThrowGrenadeRequested>.after(ContextualActSystems::Drain),
    );
}

fn throws(app: &App) -> Vec<ThrowGrenadeRequested> {
    probed::<ThrowGrenadeRequested>(app)
}

#[test]
fn arc_weapon_and_hovered_cell_offers_throw() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    hover_cell(&mut app, 15, 15);
    app.update();

    assert!(
        throw_visible(&mut app),
        "an Arc weapon + a hovered target cell must reveal the Throw button (GTW-546)",
    );
    assert!(
        root_visible(&mut app),
        "an offered Throw must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !shove_visible(&mut app),
        "no shovable neighbour in reach -> the Shove button stays hidden",
    );
}

#[test]
fn straight_weapon_or_no_hover_does_not_offer_throw() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Straight);
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        !throw_visible(&mut app),
        "a Straight-trajectory weapon offers NO throw even with a cell hovered (button hidden)",
    );

    let actor = spawn_throw_actor(&mut app, 6, 6, 0, TrajectoryStyle::Arc);
    app.world_mut().insert_resource(InspectTarget::new(None));
    app.update();
    assert!(
        !throw_visible(&mut app),
        "an Arc weapon with NOTHING hovered offers NO throw (no target cell, button hidden)",
    );

    let _ = actor;
    hover_cell(&mut app, 20, 20);
    app.update();
    assert!(
        throw_visible(&mut app),
        "an Arc weapon + a hovered cell reveals the Throw button (discriminating)",
    );
}

#[test]
fn pressing_throw_emits_throw_grenade_requested_for_hovered_cell() {
    let mut app = battle_running_app();
    add_throw_probe(&mut app);
    let thrower = spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    let target = CellLevel::new(Cell::new(15, 15), Level::new(0));
    hover_cell(&mut app, 15, 15);

    app.update();
    assert!(
        throw_visible(&mut app),
        "sanity: the Throw button is offered before the press",
    );
    let Some(throw_btn) = single_with::<ThrowGrenadeButton>(&mut app) else {
        return;
    };

    hover_cell(&mut app, 15, 15);
    press_ui_button(&mut app, throw_btn);
    app.update();

    let emitted = throws(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Throw with a target offered must emit exactly one ThrowGrenadeRequested",
    );
    assert_eq!(
        emitted[0].thrower, thrower,
        "the thrower is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried HOVERED cell (the blind lob's aim)",
    );
}
