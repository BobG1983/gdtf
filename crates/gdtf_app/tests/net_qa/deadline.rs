//! The generic frame-deadline sweep (GTW-736's `sweep_pending`) still times out an
//! UNCLAIMED queued request.
//!
//! Every request that has a same-frame consumer is answered before the sweep can fire
//! (T4 inject, T5 snapshot, T7 screenshot, and T9 start-battle all claim theirs), so
//! this exercises the remaining unclaimed queue: `GetOutput` (the T6 event drain, whose
//! consumer is not yet built). Driven on a LIVE-battle `BattleAppBuilder` app so the
//! battle-dependent `GetOutput` enqueues at all — off-battle it would be rejected
//! `NoBattle` at route time — then, with nothing to claim it, its `FrameDeadline`
//! elapses and the sweep answers `Timeout` rather than leaving the client hanging.

use gdtf_qa_protocol::envelope::{QaError, QaRequest, QaResponse};

use crate::inject_support::{inject_battle_app, send};

/// An enqueued `GetOutput` with no T6 consumer is swept to `Timeout` once its deadline
/// elapses — the generic pump answers rather than leaving the client hanging.
#[test]
fn unclaimed_queued_request_past_its_deadline_answers_timeout() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    let reply = send(&tx, QaRequest::GetOutput { max: None });
    // The route enqueues on the first update; each later update's sweep ticks the
    // deadline. Pump well past the frame budget so the sweep fires deterministically.
    for _ in 0..16 {
        app.update();
    }
    assert!(
        matches!(reply.try_recv(), Ok(QaResponse::Error(QaError::Timeout))),
        "an unclaimed queued request must answer Timeout after its deadline",
    );
}
