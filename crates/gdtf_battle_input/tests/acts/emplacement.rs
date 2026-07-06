//! Per-act contextual intents: enter / exit emplacement (GTW-543 / GTW-571).

use gdtf_battle_input::{
    SelectedShooter,
    contextual::{EnterEmplacementAct, ExitEmplacementAct, PendingContextualIntents},
};
use gdtf_battle_sim::{
    acts::{EnterEmplacementRequested, ExitEmplacementRequested},
    prelude::{Direction, StanceKind},
};
use gdtf_test_utils::probed;

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-543 / GTW-571 — pushing an emplacement onto the per-act contextual queues
// (PendingContextualIntents<EnterEmplacementAct> / <ExitEmplacementAct>) emits exactly
// one EnterEmplacementRequested / ExitEmplacementRequested for the SelectedShooter as
// the actor over the carried emplacement; with no selection nothing is written (the
// drain resolves the actor from the selection).
// ---------------------------------------------------------------------------------

/// GTW-543 / GTW-571 — pushing `emplacement` onto
/// `PendingContextualIntents<EnterEmplacementAct>` with a selected player actor emits
/// EXACTLY one `EnterEmplacementRequested { actor, emplacement }` through the act's generic
/// `drain_contextual_intents` drain, the actor being the `*SelectedShooter` and the
/// emplacement the carried terrain entity (the Enter affordance surrogate, over the GTW-571
/// per-act contextual seam). The sim's `dispatch_enter_emplacement` gate (VACANT +
/// 8-adjacent + affords `EnterEmplacementTu`) is the authoritative check, not this seam.
#[test]
fn enter_emplacement_intent_emits_request_for_selection_over_carried_emplacement() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    // The emplacement target entity — only its identity matters at this seam (the sim's
    // EmplacementState/adjacency/TU gate is the authoritative check, not this layer).
    let emplacement = app.world_mut().spawn_empty().id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<EnterEmplacementAct>>()
        .push(emplacement);
    app.update();

    let enters = probed::<EnterEmplacementRequested>(&app);
    assert_eq!(
        enters.len(),
        1,
        "one EnterEmplacementRequested via the enter-emplacement intent",
    );
    assert_eq!(
        enters[0],
        EnterEmplacementRequested::new(actor, emplacement),
        "EnterEmplacementRequested has actor = *SelectedShooter and emplacement = carried",
    );
}

/// GTW-543 — with the selection cleared (`SelectedShooter(None)`), pushing the enter-emplacement
/// intent writes NOTHING (the drain resolves the actor from the selection and is a no-op without
/// one — the same fail-closed shape as the `OpenDoor` / `Execute` arms).
#[test]
fn enter_emplacement_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // An emplacement target exists but there is NO selected actor.
    let emplacement = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<EnterEmplacementAct>>()
        .push(emplacement);
    app.update();

    assert!(
        probed::<EnterEmplacementRequested>(&app).is_empty(),
        "no EnterEmplacementRequested without a selection",
    );
}

/// GTW-543 / GTW-571 — pushing `emplacement` onto
/// `PendingContextualIntents<ExitEmplacementAct>` with a selected player actor emits EXACTLY
/// one `ExitEmplacementRequested { actor, emplacement }` through the act's generic
/// `drain_contextual_intents` drain, the actor being the `*SelectedShooter` and the
/// emplacement the carried terrain entity (the Exit affordance surrogate, over the GTW-571
/// per-act contextual seam). The sim's `dispatch_exit_emplacement` gate (the recorded
/// occupant IS the actor + affords `ExitEmplacementTu`) is the authoritative check, not this
/// seam.
#[test]
fn exit_emplacement_intent_emits_request_for_selection_over_carried_emplacement() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    let emplacement = app.world_mut().spawn_empty().id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ExitEmplacementAct>>()
        .push(emplacement);
    app.update();

    let exits = probed::<ExitEmplacementRequested>(&app);
    assert_eq!(
        exits.len(),
        1,
        "one ExitEmplacementRequested via the exit-emplacement intent",
    );
    assert_eq!(
        exits[0],
        ExitEmplacementRequested::new(actor, emplacement),
        "ExitEmplacementRequested has actor = *SelectedShooter and emplacement = carried",
    );
}

/// GTW-543 — with the selection cleared, pushing the exit-emplacement intent writes NOTHING (the
/// drain resolves the actor from the selection and is a no-op without one).
#[test]
fn exit_emplacement_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    let emplacement = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ExitEmplacementAct>>()
        .push(emplacement);
    app.update();

    assert!(
        probed::<ExitEmplacementRequested>(&app).is_empty(),
        "no ExitEmplacementRequested without a selection",
    );
}
