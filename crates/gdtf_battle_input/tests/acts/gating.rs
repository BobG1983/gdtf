//! The selection-gate boundary of the drain: the global end-turn needs no
//! selection; with no selection every act is a no-op (GTW-309, AC6).

use gdtf_battle_input::{ActIntent, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{
    Direction, OccupancyGrid, StanceKind,
    acts::{
        EndTurnRequested, ReloadRequested, SetAimingRequested, SetFacingRequested,
        SetStanceRequested,
    },
};
use gdtf_test_utils::{press_key, press_left, probed};

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-309 — pushing ActIntent::EndTurn emits exactly one (fieldless) EndTurnRequested
// through the same drain, with NO selection (a GLOBAL turn signal).
// ---------------------------------------------------------------------------------

/// GTW-309 — pushing the GLOBAL `ActIntent::EndTurn` emits EXACTLY one fieldless
/// `EndTurnRequested` through the `dispatch_act_intents` drain (the action-bar End-Turn
/// button surrogate). Unlike the per-ganger acts, end-turn needs NO `SelectedShooter`, so
/// this drives it with the selection cleared and still asserts exactly one message.
#[test]
fn end_turn_intent_emits_one_end_turn_requested_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // No ganger spawned, selection cleared — end-turn is GLOBAL, not per-actor.
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::EndTurn);
    app.update();

    let ends = probed::<EndTurnRequested>(&app);
    assert_eq!(
        ends.len(),
        1,
        "one EndTurnRequested via the end-turn intent, with no selection",
    );
    assert_eq!(
        ends[0], EndTurnRequested,
        "the drained message is the fieldless EndTurnRequested unit value",
    );
}

// ---------------------------------------------------------------------------------
// AC6 — with NO SelectedShooter, every act key/click is a no-op (zero messages).
// ---------------------------------------------------------------------------------

/// AC6 — with the selection cleared, driving ALL act keys (stance / aim / facing) + a
/// left-click emits ZERO messages of every `*Requested` type and does not panic.
#[test]
fn no_selection_makes_every_act_a_no_op() {
    let mut app = acts_app();
    add_probes(&mut app);
    let binds = test_keybinds();

    // An armed ganger EXISTS in the world but is NOT selected.
    let _ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    app.world_mut().insert_resource(SelectedShooter::cleared());
    // Hover an in-bounds cell (via the real picker) so only the selection — not the
    // hover — gates the click.
    let hovered = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert!(
        app.world()
            .get_resource::<OccupancyGrid>()
            .and_then(|g| g.occupant(&hovered))
            .is_none(),
        "precondition: the hovered cell is empty (no occupant to select)",
    );

    press_key(&mut app, binds.stance_cycle());
    press_key(&mut app, binds.aim_toggle());
    press_key(&mut app, binds.facing_cycle());
    // The Reload act has no key binding — push its intent directly (the button surrogate)
    // to prove the drain is a no-op with no selection (GTW-275).
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::Reload);
    press_left(&mut app);
    app.update();

    assert!(
        fires(&app).is_empty(),
        "no FireRequested without a selection"
    );
    assert!(
        probed::<SetStanceRequested>(&app).is_empty(),
        "no SetStanceRequested without a selection",
    );
    assert!(
        probed::<SetAimingRequested>(&app).is_empty(),
        "no SetAimingRequested without a selection",
    );
    assert!(
        probed::<SetFacingRequested>(&app).is_empty(),
        "no SetFacingRequested without a selection",
    );
    assert!(
        probed::<ReloadRequested>(&app).is_empty(),
        "no ReloadRequested without a selection",
    );
}
