//! Leaving a mount: the Exit button is the occupant's alone, and pressing it asks the sim.

use bevy::prelude::*;
use gdtf_app::test_support::ExitEmplacementButton;
use gdtf_battle_input::contextual::ContextualActSystems;
use gdtf_battle_sim::{
    acts::ExitEmplacementRequested,
    emplacement::{EmplacementState, MountedBy},
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

#[test]
fn exit_offered_only_to_the_occupant() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    spawn_emplacement(
        &mut app,
        5,
        5,
        EmplacementState::Occupied,
        Some(actor),
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );
    app.update();

    assert!(
        exit_emplacement_visible(&mut app),
        "the selection manning an emplacement must reveal the Exit button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Exit must reveal the panel root",
    );
    assert!(
        !enter_emplacement_visible(&mut app),
        "the only emplacement is OCCUPIED (by the selection) -> the Enter button stays hidden",
    );

    let other = app.world_mut().spawn_empty().id();
    let emplacements = all_with::<EmplacementState>(&mut app);
    if let [emplacement] = emplacements.as_slice() {
        app.world_mut()
            .entity_mut(*emplacement)
            .insert(MountedBy::new(other));
    }
    app.update();
    assert!(
        !exit_emplacement_visible(&mut app),
        "an emplacement occupied by SOMEONE ELSE offers the selection no Exit (occupant-only)",
    );
}

#[test]
fn a_vacant_seat_still_naming_the_actor_as_occupant_offers_no_exit() {
    let mut app = battle_running_app();
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    let emplacement = spawn_emplacement(
        &mut app,
        5,
        5,
        EmplacementState::Vacant,
        Some(actor),
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );
    app.update();

    assert!(
        !exit_emplacement_visible(&mut app),
        "a VACANT seat is nobody's to leave, whatever occupant it still names -> no Exit",
    );

    if let Ok(mut entity) = app.world_mut().get_entity_mut(emplacement) {
        entity.insert(EmplacementState::Occupied);
    }
    app.update();
    assert!(
        exit_emplacement_visible(&mut app),
        "flipping the same seat to OCCUPIED reveals the Exit button (discriminating)",
    );
}

fn add_exit_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ExitEmplacementRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ExitEmplacementRequested>.after(ContextualActSystems::Drain),
    );
}

fn exit_requests(app: &App) -> Vec<ExitEmplacementRequested> {
    probed::<ExitEmplacementRequested>(app)
}

#[test]
fn pressing_exit_emits_exit_emplacement_requested_for_manned_mount() {
    let mut app = battle_running_app();
    add_exit_probe(&mut app);
    let actor = spawn_emplacement_actor(&mut app, 5, 5, 0);
    let emplacement = spawn_emplacement(
        &mut app,
        5,
        5,
        EmplacementState::Occupied,
        Some(actor),
        Some(cardinal_sides()),
        Some(cardinal_facing()),
    );

    app.update();
    assert!(
        exit_emplacement_visible(&mut app),
        "sanity: the Exit button is offered before the press",
    );
    let exit_btn = the_only::<ExitEmplacementButton>(
        &mut app,
        "the panel must offer exactly one Exit button to press",
    );

    press_ui_button(&mut app, exit_btn);
    app.update();

    let emitted = exit_requests(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Exit while manning must emit exactly one ExitEmplacementRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].emplacement, emplacement,
        "the emplacement is the carried manned mount",
    );
}
