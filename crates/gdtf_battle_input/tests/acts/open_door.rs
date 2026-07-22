//! Per-act contextual intents: open door (GTW-315 / GTW-571).

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

// ---------------------------------------------------------------------------------
// GTW-315 / GTW-571 — pushing a door onto PendingContextualIntents<OpenDoorAct> emits
// exactly one OpenDoorRequested for the SelectedShooter as the actor over the carried
// door; with no selection, nothing is written (the drain resolves the actor from the
// selection).
// ---------------------------------------------------------------------------------

/// GTW-315 / GTW-571 — pushing `door` onto `PendingContextualIntents<OpenDoorAct>` with a
/// selected player actor emits EXACTLY one `OpenDoorRequested { actor, door }` through the
/// act's generic `drain_contextual_intents` drain, the actor being the `*SelectedShooter`
/// and the door the carried openable entity (the Open-Door affordance surrogate, over the
/// GTW-571 per-act contextual queue). The sim's `dispatch_open_door` gate (CLOSED +
/// 8-adjacent + affords `OpenDoorTu`) is the authoritative check, not this layer.
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
    // The door target entity — only its identity matters at this point (the sim's
    // OpenState/adjacency/TU gate is the authoritative check, not this layer).
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

/// GTW-315 — with the selection cleared (`SelectedShooter(None)`), pushing the open-door
/// intent writes NOTHING (the drain resolves the actor from the selection and is a no-op
/// without one — the same fail-closed shape as the Execute / Reload arms).
#[test]
fn open_door_intent_emits_nothing_without_selection() {
    let mut app = acts_app();
    add_probes(&mut app);
    // A door target exists but there is NO selected actor.
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
