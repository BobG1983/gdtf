//! The deadline pump over a REAL [`PendingQueue`] in a real `App` (GTW-803).

use std::sync::mpsc::{Receiver, TryRecvError};

use bevy::prelude::*;
use gdtf_qa_protocol::message::{QaError, QaResponse};

use crate::{
    Responder,
    pending::{
        PendingQueue,
        deadline::{DEADLINE_BUDGET, DeadlineTick},
        sweep_pending,
    },
};

/// A stand-in payload: the pending map is generic over the payload, and every real one is
/// the host's, so the transport's own test brings its own.
#[derive(Debug)]
struct TestPayload;

/// How many `sweep_pending` runs an entry survives before the tick that expires it —
/// derived from the real [`DEADLINE_BUDGET`] rather than a hard-coded literal.
fn frames_to_expiry() -> u32 {
    let mut deadline = DEADLINE_BUDGET;
    let mut frames = 1;
    while matches!(deadline.tick(), DeadlineTick::Live) {
        frames += 1;
    }
    frames
}

/// An app running only the REAL sweep over a `TestPayload` queue.
fn sweeping_app() -> App {
    let mut app = App::new();
    app.init_resource::<PendingQueue<TestPayload>>()
        .add_systems(Update, sweep_pending::<TestPayload>);
    app
}

/// Enqueue one payload, handing back the reply channel the client would block on.
fn enqueue(app: &mut App) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    if let Some(mut queue) = app
        .world_mut()
        .get_resource_mut::<PendingQueue<TestPayload>>()
    {
        queue.push_new(TestPayload, responder);
    }
    reply_rx
}

/// An entry nothing claims is answered `Timeout` — on the frame its budget expires, and
/// not one frame earlier.
#[test]
fn an_unclaimed_entry_times_out_when_its_budget_expires() {
    let mut app = sweeping_app();
    let reply_rx = enqueue(&mut app);

    let expiry = frames_to_expiry();
    for frame in 1..expiry {
        app.update();
        assert!(
            matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
            "frame {frame} of {expiry} must not answer yet",
        );
    }

    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Ok(QaResponse::Error(QaError::Timeout))),
        "the sweep must answer Timeout on the frame the budget expires",
    );
}

/// An entry a consumer claims before its budget expires never reaches the sweep: it is
/// answered with whatever the claimer sends, and no `Timeout` ever follows.
#[test]
fn a_claimed_entry_is_answered_by_its_claimer_and_never_swept() {
    let mut app = sweeping_app();
    let reply_rx = enqueue(&mut app);

    // Claim it the way a real consumer does, then answer on its responder.
    let claimed = app
        .world_mut()
        .get_resource_mut::<PendingQueue<TestPayload>>()
        .map(|mut queue| queue.drain_ready())
        .unwrap_or_default();
    assert_eq!(claimed.len(), 1, "the push must be drainable");
    for (_payload, responder) in claimed {
        responder.reply(QaResponse::Error(QaError::Busy));
    }

    assert!(
        matches!(reply_rx.try_recv(), Ok(QaResponse::Error(QaError::Busy))),
        "the claimer's reply must reach the client",
    );

    // Well past the budget, the sweep has nothing left to time out.
    for _ in 0..=frames_to_expiry() {
        app.update();
    }
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Disconnected)),
        "a claimed entry must never produce a second, swept reply",
    );
}
