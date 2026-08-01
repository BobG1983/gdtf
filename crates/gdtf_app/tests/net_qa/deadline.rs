//! GTW-943 retargeted this file onto the command layer's own pending queue.
//!
//! It has always pinned the same property from whichever side had it: that a routed request
//! is answered by its own consumer rather than falling through to the transport's
//! frame-deadline sweep. GTW-739 pinned it through `GetOutput` and the T6 outbox pump; that
//! request is gone, and the queue the sweep now guards is
//! `PendingQueue<CommandCall<C>>` — the per-command queue `register_command_set` inits, with
//! `sweep_pending::<CommandCall<C>>` registered beside it.
//!
//! The distinction matters because both answers travel the same socket: a `Timeout` here
//! would look, to a client, exactly like the app being slow. `DEADLINE_BUDGET` is four
//! frames, so a run driven well past that and still answered by its handler is the sweep
//! losing the race it must lose.

use gdtf_qa_protocol::{
    command::{CommandOutcome, RunOptions},
    message::{QaError, QaResponse},
};

use super::{
    command_exchange::{APP_PHASE, exchange, run},
    socket_support::TestResult,
};

/// An admitted `Run` is answered by its command's own handler, never by the deadline sweep.
///
/// The exchange drives the app for far more frames than the four-frame deadline budget
/// while the client waits, so a call the decode step or the handler failed to claim would
/// come back `Timeout`. It comes back `Ran`.
#[test]
fn an_admitted_run_is_answered_by_its_handler_not_the_deadline_sweep() -> TestResult {
    let reply = exchange(run(APP_PHASE, "{}", RunOptions::default()))?;
    assert!(
        !matches!(reply, QaResponse::Error(QaError::Timeout)),
        "an admitted call must be claimed and answered by its command, not swept: {reply:?}",
    );
    let QaResponse::Outcome(outcome) = reply else {
        unreachable!("a Run is answered with an outcome, got {reply:?}");
    };
    assert!(
        matches!(outcome, CommandOutcome::Ran { .. }),
        "app.phase runs and answers its declared reply: {outcome:?}",
    );
    Ok(())
}
