//! GTW-739: the REAL T6 outbox pump (`drive_output`) on a live-battle `BattleAppBuilder`
//! app.
//!
//! A `GetOutput` is answered by PROJECTING the sim-owned act log the same frame it is
//! routed — the router pushes in `InputSystems::Gather`, the sim records in
//! `SimSystems::Record`, and the pump drains after `Record`, all in one update. These tests
//! drive that real path: they append curated acts straight into the battle's `ActLog`
//! (the log's real `append` API), then assert the drained `EventBatch` carries the
//! projected wire events, that a second drain is empty, and that a ring overflow reports the
//! dropped gap while retaining the newest entries. No wall-clock waits: one `app.update()`
//! per drain.

use std::sync::mpsc;

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActLogCapacity, ActProvenance, RecordedAct},
    acts::MoveRejection,
};
use gdtf_net_qa_transport::IncomingRequest;
use gdtf_qa_protocol::{
    envelope::{QaRequest, QaResponse},
    events::{DroppedCount, EventBatch, MoveRejectionNet, NetEvent},
    ids::{EventCap, GangerToken},
};

use crate::inject_support::{inject_battle_app, send};

/// Append one rejected-move act (which curates to a [`NetEvent::MoveRejected`]) for `actor`
/// straight into the battle's real act log.
fn append_move_refused(app: &mut App, actor: Entity) {
    app.world_mut()
        .resource_mut::<ActLog>()
        .append(RecordedAct::new(
            actor,
            ActProvenance::Commanded,
            ActDeed::MoveRefused {
                reason: MoveRejection::Unreachable,
            },
        ));
}

/// The wire event `append_move_refused(_, actor)` must project to.
const fn move_rejected(actor: Entity) -> NetEvent {
    NetEvent::MoveRejected {
        actor:  GangerToken::new(actor.to_bits()),
        reason: MoveRejectionNet::Unreachable,
    }
}

/// Send a `GetOutput`, pump exactly one frame, and return the drained batch (or `None` on a
/// non-`Output` reply).
fn drain(
    app: &mut App,
    tx: &mpsc::Sender<IncomingRequest>,
    max: Option<EventCap>,
) -> Option<EventBatch> {
    let reply = send(tx, QaRequest::GetOutput { max });
    app.update();
    match reply.try_recv() {
        Ok(QaResponse::Output(batch)) => Some(batch),
        _ => None,
    }
}

/// A curated act appended to the log drains through the real pump as its projected wire
/// event, with no dropped-event gap for an in-window read.
#[test]
fn a_projected_curated_act_drains_as_a_real_event_batch() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // Drain any battle-setup acts so the QA cursor sits at the log head.
    drop(drain(&mut app, &tx, None));

    let actor = app.world_mut().spawn_empty().id();
    append_move_refused(&mut app, actor);

    let batch = drain(&mut app, &tx, None);
    assert!(
        batch.is_some(),
        "GetOutput in a battle must answer a real Output batch",
    );
    let Some(batch) = batch else { return };
    assert!(
        batch.events.contains(&move_rejected(actor)),
        "the drained batch must carry the projected event, got {:?}",
        batch.events,
    );
    assert_eq!(
        batch.dropped,
        DroppedCount::new(0),
        "an in-window drain drops nothing",
    );
}

/// After a drain empties the window, a second drain the client makes is empty and reports
/// no drops — the non-destructive cursor advanced past what it already delivered.
#[test]
fn a_second_drain_after_the_first_is_empty_with_no_drops() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    drop(drain(&mut app, &tx, None));

    let actor = app.world_mut().spawn_empty().id();
    append_move_refused(&mut app, actor);

    let first = drain(&mut app, &tx, None);
    assert!(
        first.is_some_and(|batch| !batch.events.is_empty()),
        "the first drain sees the appended event",
    );

    let second = drain(&mut app, &tx, None);
    assert!(second.is_some(), "the second drain still answers a batch");
    let Some(second) = second else { return };
    assert!(
        second.events.is_empty(),
        "the second drain is empty, got {:?}",
        second.events,
    );
    assert_eq!(
        second.dropped,
        DroppedCount::new(0),
        "an empty drain drops nothing"
    );
}

/// A ring overflow drops the OLDEST entries and counts them: the batch reports the dropped
/// gap and carries only the newest entries the ring retained.
#[test]
fn capacity_overflow_reports_the_dropped_gap_and_retains_the_newest() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    // Spawn the actors first, then swap in a tiny ring. The QA cursor is still at its start
    // (this test never drains before the overflow), so the gap is measured from zero rather
    // than triggering the fresh-battle resync.
    let actors: Vec<Entity> = (0..10)
        .map(|_| app.world_mut().spawn_empty().id())
        .collect();
    app.world_mut()
        .insert_resource(ActLog::new(ActLogCapacity::new(4)));
    for actor in &actors {
        append_move_refused(&mut app, *actor);
    }

    let batch = drain(&mut app, &tx, None);
    assert!(batch.is_some(), "GetOutput must answer a batch");
    let Some(batch) = batch else { return };

    assert_eq!(
        batch.dropped,
        DroppedCount::new(6),
        "the 6 entries evicted before the cursor reached them are counted",
    );
    let newest: Vec<NetEvent> = actors[6..]
        .iter()
        .map(|actor| move_rejected(*actor))
        .collect();
    assert_eq!(
        batch.events, newest,
        "only the newest 4 within the ring survive, in order",
    );
}

/// An event cap bounds the batch, and the next drain resumes exactly where the capped one
/// stopped — the cursor advances only past what was delivered.
#[test]
fn the_event_cap_limits_the_batch_and_the_next_drain_resumes() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    drop(drain(&mut app, &tx, None));

    let actors: Vec<Entity> = (0..5).map(|_| app.world_mut().spawn_empty().id()).collect();
    for actor in &actors {
        append_move_refused(&mut app, *actor);
    }

    let first = drain(&mut app, &tx, Some(EventCap::new(2)));
    assert!(
        first.is_some_and(|batch| batch.events.len() == 2),
        "the cap bounds the first batch to 2",
    );
    let second = drain(&mut app, &tx, Some(EventCap::new(2)));
    assert!(
        second.is_some_and(|batch| batch.events.len() == 2),
        "the second capped drain resumes with the next 2",
    );
    let third = drain(&mut app, &tx, None);
    assert!(
        third.is_some_and(|batch| batch.events.len() == 1),
        "the final drain returns the remaining 1",
    );
}
