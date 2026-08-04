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

#[test]
fn enter_emplacement_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
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
