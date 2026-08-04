use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent, next_facing, next_stance};
use gdtf_battle_sim::{
    acts::{
        AimRequest, ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
    },
    prelude::{Direction, StanceKind},
};
use gdtf_test_utils::{press_key, probed};

use super::harness::*;

#[derive(Clone)]
enum Drive {
    Key(KeyCode),
    Intent(ActIntent),
}

fn drive_one_act(drive: Drive) -> (App, Entity) {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);
    match drive {
        Drive::Key(key) => press_key(&mut app, key),
        Drive::Intent(intent) => app
            .world_mut()
            .resource_mut::<PendingActIntent>()
            .push(intent),
    }
    app.update();
    (app, ganger)
}

#[test]
fn stance_key_emits_next_of_cycle_and_matches_direct_intent() {
    let key = test_keybinds().stance_cycle();
    let (app, ganger) = drive_one_act(Drive::Key(key));
    let via_key = probed::<SetStanceRequested>(&app);
    assert_eq!(
        via_key.len(),
        1,
        "one SetStanceRequested via the stance key"
    );
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].stance,
        next_stance(StanceKind::Standing),
        "the next-of-cycle stance",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::StanceCycle));
    let via_intent = probed::<SetStanceRequested>(&app2);
    assert_eq!(via_intent.len(), 1, "one SetStanceRequested via the intent");
    assert_eq!(
        via_key[0], via_intent[0],
        "key and button must produce byte-for-byte equal SetStanceRequested",
    );
}

#[test]
fn aim_key_toggles_and_matches_direct_intent() {
    let key = test_keybinds().aim_toggle();
    let (app, ganger) = drive_one_act(Drive::Key(key));
    let via_key = probed::<SetAimingRequested>(&app);
    assert_eq!(via_key.len(), 1, "one SetAimingRequested via the aim key");
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].aim,
        AimRequest::new(true),
        "aim toggles from the ganger's current false to true",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::AimToggle));
    let via_intent = probed::<SetAimingRequested>(&app2);
    assert_eq!(via_intent.len(), 1, "one SetAimingRequested via the intent");
    assert_eq!(
        via_key[0], via_intent[0],
        "key and button must produce byte-for-byte equal SetAimingRequested",
    );
}

#[test]
fn facing_key_emits_next_of_cycle_and_matches_direct_intent() {
    let key = test_keybinds().facing_cycle();
    let (app, ganger) = drive_one_act(Drive::Key(key));
    let via_key = probed::<SetFacingRequested>(&app);
    assert_eq!(
        via_key.len(),
        1,
        "one SetFacingRequested via the facing key"
    );
    assert_eq!(via_key[0].actor, ganger, "actor = *SelectedShooter");
    assert_eq!(
        via_key[0].facing,
        next_facing(Direction::North),
        "the next-of-cycle facing",
    );

    let (app2, _) = drive_one_act(Drive::Intent(ActIntent::FacingCycle));
    let via_intent = probed::<SetFacingRequested>(&app2);
    assert_eq!(via_intent.len(), 1, "one SetFacingRequested via the intent");
    assert_eq!(
        via_key[0], via_intent[0],
        "key and button must produce byte-for-byte equal SetFacingRequested",
    );
}

#[test]
fn reload_intent_emits_one_reload_requested_for_the_selection() {
    let (app, ganger) = drive_one_act(Drive::Intent(ActIntent::Reload));
    let reloads = probed::<ReloadRequested>(&app);
    assert_eq!(
        reloads.len(),
        1,
        "one ReloadRequested via the reload intent",
    );
    assert_eq!(
        reloads[0].actor, ganger,
        "the ReloadRequested actor is the *SelectedShooter",
    );
}
