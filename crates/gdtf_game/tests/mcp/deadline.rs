use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions},
    message::{QaError, QaResponse},
};

use super::{
    command_exchange::{APP_PHASE, exchange, run},
    socket_support::{TestResult, game_app_listening},
};

#[test]
fn an_admitted_run_is_answered_by_its_handler_not_the_deadline_sweep() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(APP_PHASE, "()", RunOptions::default()),
    )?;
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
