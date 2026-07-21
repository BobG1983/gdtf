//! GTW-739 retargeted this file. It previously pinned the generic frame-deadline sweep
//! (`sweep_pending`) by the one request that had no consumer yet — `GetOutput`, the T6
//! event drain — and asserted the sweep answered it `Timeout`.
//!
//! Now the T6 outbox pump (`drive_output`) claims and answers `GetOutput` the SAME frame it
//! is routed (it drains after `SimSystems::Record`, and the router pushes in
//! `InputSystems::Gather` earlier in the same update), so a `GetOutput` in a live battle
//! returns a real `EventBatch` and never reaches the deadline sweep. This test asserts that
//! new contract on the same live-battle harness.

use gdtf_qa_protocol::envelope::{QaRequest, QaResponse};

use crate::inject_support::{inject_battle_app, send};

/// A `GetOutput` in a live battle is answered by the T6 outbox pump with a real event batch
/// the same frame — it no longer falls through to the deadline sweep's `Timeout`.
#[test]
fn get_output_in_a_battle_returns_a_real_event_batch_not_a_timeout() {
    let Some((mut app, tx)) = inject_battle_app() else {
        return;
    };
    let reply = send(&tx, QaRequest::GetOutput { max: None });
    // The router routes the request in `InputSystems::Gather` and the T6 pump drains it
    // after `SimSystems::Record`, both in this single update — no wall-clock wait.
    app.update();
    assert!(
        matches!(reply.try_recv(), Ok(QaResponse::Output(_))),
        "GetOutput in a battle must answer a real Output batch, not a Timeout",
    );
}
