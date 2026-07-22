//! Per-act contextual intents: downed execute / stabilize (GTW-294 / GTW-571).

use gdtf_battle_input::{
    SelectedShooter,
    contextual::{ExecuteAct, PendingContextualIntents, StabilizeAct},
};
use gdtf_battle_sim::{
    acts::{ExecuteDownedRequested, StabilizeDownedRequested},
    prelude::{Direction, StanceKind},
};
use gdtf_test_utils::probed;

use super::harness::*;

// ---------------------------------------------------------------------------------
// GTW-294 / GTW-571 — pushing a target onto the per-act contextual queues
// (PendingContextualIntents<ExecuteAct> / <StabilizeAct>) emits exactly one
// ExecuteDownedRequested / StabilizeDownedRequested for the SelectedShooter as the actor
// over the carried downed target; with the selection cleared, nothing is written.
// ---------------------------------------------------------------------------------

/// GTW-294 / GTW-571 — pushing `target` onto `PendingContextualIntents<ExecuteAct>` and
/// `<StabilizeAct>` with a selected shooter emits EXACTLY one
/// `ExecuteDownedRequested { actor, target }` and one
/// `StabilizeDownedRequested { actor, target }` through the per-act generic
/// `drain_contextual_intents` drains, the actor being the `*SelectedShooter` and the
/// target the carried downed entity (the downed-target affordance surrogate, over the
/// GTW-571 per-act contextual queue).
#[test]
fn downed_intents_emit_requests_for_selection_over_carried_target() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    // The downed target entity — only its identity matters at this point (the sim's
    // faction/adjacency gate is the authoritative check, not this layer).
    let target = app.world_mut().spawn(ENEMY_FACTION).id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ExecuteAct>>()
        .push(target);
    app.world_mut()
        .resource_mut::<PendingContextualIntents<StabilizeAct>>()
        .push(target);
    app.update();

    let executes = probed::<ExecuteDownedRequested>(&app);
    assert_eq!(
        executes.len(),
        1,
        "one ExecuteDownedRequested via the execute intent",
    );
    assert_eq!(
        executes[0],
        ExecuteDownedRequested::new(actor, target),
        "ExecuteDownedRequested has actor = *SelectedShooter and target = carried",
    );

    let stabilizes = probed::<StabilizeDownedRequested>(&app);
    assert_eq!(
        stabilizes.len(),
        1,
        "one StabilizeDownedRequested via the stabilize intent",
    );
    assert_eq!(
        stabilizes[0],
        StabilizeDownedRequested::new(actor, target),
        "StabilizeDownedRequested has actor = *SelectedShooter and target = carried",
    );
}

/// GTW-294 — with the selection cleared (`SelectedShooter(None)`), pushing both downed
/// intents writes NOTHING (the drain resolves the actor from the selection and is a no-op
/// without one — the same fail-closed shape as the Reload arm).
#[test]
fn downed_intents_emit_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // A downed target exists but there is NO selected actor.
    let target = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<ExecuteAct>>()
        .push(target);
    app.world_mut()
        .resource_mut::<PendingContextualIntents<StabilizeAct>>()
        .push(target);
    app.update();

    assert!(
        probed::<ExecuteDownedRequested>(&app).is_empty(),
        "no ExecuteDownedRequested without a selection",
    );
    assert!(
        probed::<StabilizeDownedRequested>(&app).is_empty(),
        "no StabilizeDownedRequested without a selection",
    );
}
