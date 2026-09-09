use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions},
    message::McpResponse,
};

use super::{
    command_exchange::{APP_PHASE, exchange, run},
    socket_support::{TestResult, battle_app_listening},
};

#[test]
fn the_battle_fixture_answers_app_phase_with_a_live_battlescape() -> TestResult {
    let reply = exchange(
        battle_app_listening,
        run(APP_PHASE, "()", RunOptions::default()),
    )?;
    let McpResponse::Outcome(CommandOutcome::Ran { reply, .. }) = reply else {
        unreachable!("a plain app.phase call must RUN, got {reply:?}");
    };
    let body = reply.as_str();
    assert!(
        body.contains("battlescape:Some(BattleRunning)"),
        "the battle fixture must be reachable through `exchange` and rest in BattleRunning: \
         {body}",
    );
    Ok(())
}
