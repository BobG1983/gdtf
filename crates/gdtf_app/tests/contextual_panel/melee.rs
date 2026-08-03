use bevy::{ecs::entity::Entity, prelude::*};
use gdtf_app::test_support::MeleeButton;
use gdtf_battle_input::{SelectedShooter, contextual::ContextualActSystems};
use gdtf_battle_sim::{
    acts::{MeleeRequested, MeleeTarget},
    ganger::Facing,
    prelude::{Direction, Faction, Position, Stance, StanceKind},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

fn spawn_melee_actor(app: &mut App, x: i32, y: i32, gang: u8) -> Entity {
    let actor = app
        .world_mut()
        .spawn((
            at(x, y),
            Faction::new(gang),
            Stance::new(StanceKind::Standing),
            Facing::new(Direction::East),
        ))
        .id();
    app.world_mut().insert_resource(SelectedShooter::new(actor));
    actor
}

fn add_melee_probe(app: &mut App) {
    app.init_resource::<MessageProbe<MeleeRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<MeleeRequested>.after(ContextualActSystems::Drain),
    );
}

fn melees(app: &App) -> Vec<MeleeRequested> {
    probed::<MeleeRequested>(app)
}


#[test]
fn adjacent_alive_enemy_in_los_offers_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();

    assert!(
        melee_visible(&mut app),
        "an 8-adjacent alive enemy in LOS must reveal the Melee button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Melee must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "an ALIVE enemy is no Execute target (Execute needs a DOWNED enemy) -> hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

#[test]
fn non_adjacent_or_ally_does_not_offer_melee() {
    let mut app = battle_running_app();
    spawn_melee_actor(&mut app, 5, 5, 0);
    let enemy = spawn_alive_enemy(&mut app, 20, 20, 1);
    spawn_alive_enemy(&mut app, 5, 6, 0);
    app.update();

    assert!(
        !melee_visible(&mut app),
        "a non-adjacent enemy + an adjacent ALLY offer NO melee (the button stays hidden)",
    );

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(enemy) {
        *pos = at(6, 6);
    }
    app.update();
    assert!(
        melee_visible(&mut app),
        "moving the alive enemy into 8-adjacency reveals the Melee button (discriminating)",
    );
}

#[test]
fn pressing_melee_emits_melee_requested_for_target() {
    let mut app = battle_running_app();
    add_melee_probe(&mut app);
    let attacker = spawn_melee_actor(&mut app, 5, 5, 0);
    let target = spawn_alive_enemy(&mut app, 6, 6, 1);

    app.update();
    assert!(
        melee_visible(&mut app),
        "sanity: the Melee button is offered before the press",
    );
    let Some(melee_btn) = single_with::<MeleeButton>(&mut app) else {
        return;
    };

    press_ui_button(&mut app, melee_btn);
    app.update();

    let emitted = melees(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Melee with a target offered must emit exactly one MeleeRequested",
    );
    assert_eq!(
        emitted[0].attacker, attacker,
        "the attacker is the SelectedShooter",
    );
    assert_eq!(
        emitted[0].target,
        MeleeTarget::Ganger(target),
        "the target is the carried opposing neighbour (the ganger melee form)",
    );
}
