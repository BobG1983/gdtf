use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::ThrowGrenadeButton;
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::{ThrowGrenadeRequested, throw_grenade_tu_cost},
    ganger::Tu,
    prelude::{Cell, CellLevel, Faction, Level},
    tuning::CombatTuning,
    weapon::{TrajectoryStyle, WieldedBy},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

/// What one throw would charge, taken from the sim's own cost helper against the live tuning.
pub(super) fn throw_cost(app: &App) -> Tu {
    app.world()
        .get_resource::<CombatTuning>()
        .map_or_else(|| Tu::new(0), throw_grenade_tu_cost)
}

pub(super) fn spawn_throw_actor(
    app: &mut App,
    x: i32,
    y: i32,
    gang: u8,
    trajectory: TrajectoryStyle,
) -> Entity {
    spawn_throw_actor_with_rounds(app, x, y, gang, trajectory, 1)
}

pub(super) fn spawn_throw_actor_with_rounds(
    app: &mut App,
    x: i32,
    y: i32,
    gang: u8,
    trajectory: TrajectoryStyle,
    rounds: u16,
) -> Entity {
    let pool = throw_cost(app);
    let actor = app
        .world_mut()
        .spawn((at(x, y), Faction::new(gang), pool))
        .id();
    app.world_mut()
        .spawn((WieldedBy::new(actor), trajectory, magazine_of(rounds)));
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

pub(super) fn hover_cell(app: &mut App, x: i32, y: i32) {
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(CellLevel::new(
            Cell::new(x, y),
            Level::new(0),
        ))));
}

pub(super) fn clear_hover(app: &mut App) {
    app.world_mut().insert_resource(InspectTarget::new(None));
}

pub(super) fn throw_visible(app: &mut App) -> bool {
    visibility::<ThrowGrenadeButton>(app) == Some(Visibility::Visible)
}

pub(super) fn add_throw_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ThrowGrenadeRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ThrowGrenadeRequested>.after(ContextualActSystems::Drain),
    );
}

pub(super) fn throws(app: &App) -> Vec<ThrowGrenadeRequested> {
    probed::<ThrowGrenadeRequested>(app)
}

pub(super) fn the_throw_button(app: &mut App) -> Entity {
    the_only::<ThrowGrenadeButton>(
        app,
        "the panel must offer exactly one Throw button to press",
    )
}

#[test]
fn arc_weapon_and_hovered_cell_offers_throw() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    hover_cell(&mut app, 15, 15);
    app.update();

    assert!(
        throw_visible(&mut app),
        "an Arc weapon + a hovered target cell must reveal the Throw button",
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
fn a_straight_weapon_does_not_offer_throw() {
    let mut app = battle_running_app();
    spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Straight);
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        !throw_visible(&mut app),
        "a Straight-trajectory weapon offers NO throw even with a cell hovered (button hidden)",
    );

    let _ = spawn_throw_actor(&mut app, 6, 6, 0, TrajectoryStyle::Arc);
    hover_cell(&mut app, 20, 20);
    app.update();
    assert!(
        throw_visible(&mut app),
        "an Arc weapon + a hovered cell reveals the Throw button (discriminating)",
    );
}

#[test]
fn an_empty_magazine_does_not_offer_throw() {
    let mut app = battle_running_app();
    spawn_throw_actor_with_rounds(&mut app, 5, 5, 0, TrajectoryStyle::Arc, 0);
    hover_cell(&mut app, 15, 15);
    app.update();

    assert!(
        !throw_visible(&mut app),
        "an EMPTY magazine offers NO throw — the sim refuses it, so the panel must not carry it",
    );

    let weapon = the_only::<TrajectoryStyle>(
        &mut app,
        "exactly one weapon must carry a TrajectoryStyle, or the round this case loads goes into \
         a weapon the offer never reads and the reload half proves nothing",
    );
    if let Ok(mut entity) = app.world_mut().get_entity_mut(weapon) {
        entity.insert(magazine_of(1));
    }
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        throw_visible(&mut app),
        "loading one round into the same weapon reveals the Throw button (discriminating)",
    );
}

#[test]
fn switching_to_a_ganger_with_nothing_throwable_hides_throw() {
    let mut app = battle_running_app();
    let thrower = spawn_throw_actor(&mut app, 5, 5, 0, TrajectoryStyle::Arc);
    let slugger = spawn_throw_actor(&mut app, 6, 6, 0, TrajectoryStyle::Straight);
    app.world_mut()
        .insert_resource(SelectedShooter::new(thrower));
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        throw_visible(&mut app),
        "sanity: the Arc wielder with a hovered cell offers Throw",
    );

    app.world_mut()
        .insert_resource(SelectedShooter::new(slugger));
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        !throw_visible(&mut app),
        "the same hover must not keep Throw up after the selected ganger has nothing throwable",
    );
}

#[test]
fn emptying_the_magazine_hides_throw_while_hover_stays() {
    let mut app = battle_running_app();
    spawn_throw_actor_with_rounds(&mut app, 5, 5, 0, TrajectoryStyle::Arc, 1);
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        throw_visible(&mut app),
        "sanity: a loaded Arc weapon with a hovered cell offers Throw",
    );

    let weapon = the_only::<TrajectoryStyle>(
        &mut app,
        "exactly one weapon must carry a TrajectoryStyle so emptying it is the offer the panel \
         reads",
    );
    if let Ok(mut entity) = app.world_mut().get_entity_mut(weapon) {
        entity.insert(magazine_of(0));
    }
    hover_cell(&mut app, 15, 15);
    app.update();
    assert!(
        !throw_visible(&mut app),
        "emptying the magazine must hide Throw even though the same cell stays hovered",
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
    let throw_btn = the_throw_button(&mut app);

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
