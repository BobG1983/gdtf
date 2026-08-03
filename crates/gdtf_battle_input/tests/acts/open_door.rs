use gdtf_battle_input::{
    SelectedShooter,
    contextual::{OpenDoorAct, PendingContextualIntents},
};
use gdtf_battle_sim::{
    acts::OpenDoorRequested,
    prelude::{Direction, StanceKind},
};
use gdtf_test_utils::probed;

use super::harness::*;


#[test]
fn open_door_intent_emits_request_for_selection_over_carried_door() {
    let mut app = acts_app();
    add_probes(&mut app);
    let actor = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, actor);
    let door = app.world_mut().spawn_empty().id();

    app.world_mut()
        .resource_mut::<PendingContextualIntents<OpenDoorAct>>()
        .push(door);
    app.update();

    let opens = probed::<OpenDoorRequested>(&app);
    assert_eq!(
        opens.len(),
        1,
        "one OpenDoorRequested via the open-door intent",
    );
    assert_eq!(
        opens[0],
        OpenDoorRequested::new(actor, door),
        "OpenDoorRequested has actor = *SelectedShooter and door = carried",
    );
}

#[test]
fn open_door_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    let door = app.world_mut().spawn_empty().id();
    app.world_mut().insert_resource(SelectedShooter::cleared());

    app.world_mut()
        .resource_mut::<PendingContextualIntents<OpenDoorAct>>()
        .push(door);
    app.update();

    assert!(
        probed::<OpenDoorRequested>(&app).is_empty(),
        "no OpenDoorRequested without a selection",
    );
}
