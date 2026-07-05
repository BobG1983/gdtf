//! Posture keys vs direct-intent parity + the reload intent (AC5, GTW-275).

use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent, next_facing, next_stance};
use gdtf_battle_sim::{
    Direction, StanceKind,
    acts::{
        AimRequest, ReloadRequested, SetAimingRequested, SetFacingRequested, SetStanceRequested,
    },
};
use gdtf_test_utils::{press_key, probed};

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC5 — posture keys emit the matching *Requested for *SelectedShooter with the
// next-of-cycle value, byte-for-byte equal to the same intent pushed directly.
// ---------------------------------------------------------------------------------

/// The two ways to drive one posture act — a synthesized KEY press, or a direct
/// `ActIntent` push (the 222c-button surrogate over the SAME seam). Not `Copy`:
/// [`Drive::Intent`] holds an `ActIntent`, which is no longer `Copy`.
#[derive(Clone)]
enum Drive {
    /// Synthesize a just-pressed of `key`.
    Key(KeyCode),
    /// Push `intent` directly onto the shared queue (the 222c-button surrogate).
    Intent(ActIntent),
}

/// Builds a fresh acts app, selects an armed ganger (at the standing/north start), and
/// applies `drive` (a key press OR a direct intent push), then `update()`s once.
/// Returns `(app, ganger)` for the caller to read the relevant probe.
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

/// AC5 — the stance-cycle KEY emits one `SetStanceRequested` for `*SelectedShooter` with
/// the next-of-cycle stance, byte-for-byte EQUAL to the message the direct `StanceCycle`
/// intent produces over the SAME seam. (The keyboard keeps the blind cycle; the GTW-267
/// action-bar replaced its BLIND-cycle button with direct-set `SetStance` toggles.)
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

/// AC5 — the aim key emits one `SetAimingRequested` toggling the actor's aim, byte-for-
/// byte EQUAL to the direct `AimToggle` intent's message.
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

/// AC5 — the facing key emits one `SetFacingRequested` for the next-of-cycle facing,
/// byte-for-byte EQUAL to the direct `FacingCycle` intent's message.
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

/// GTW-275 AC4 — pushing `ActIntent::Reload` emits exactly one `ReloadRequested` for
/// the `*SelectedShooter` through the `gdtf_battle_input` seam (the weapon panel's Reload
/// button surrogate, over the SAME `dispatch_act_intents` drain the other intents use).
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
