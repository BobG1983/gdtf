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

#[test]
fn downed_intents_emit_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
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
