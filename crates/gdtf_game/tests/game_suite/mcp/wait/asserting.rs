//! The helper the act cases read a wait with only passes for the condition that came true.

use cobalt_mcp_protocol::command::RunOptions;
use gdtf_game::qa_wire::WaitConditionNet;

use crate::mcp::{
    act_support::assert_waited_for,
    command_exchange::{WAIT, exchange, run},
    socket_support::{TestResult, game_app_listening},
};

#[test]
fn the_wait_assertion_turns_down_a_reply_that_answered_another_condition() -> TestResult {
    let answered = exchange(
        game_app_listening,
        run(WAIT, "(condition:CaughtUp)", RunOptions::default()),
    )?;

    assert!(
        assert_waited_for(WaitConditionNet::CaughtUp, answered.clone()).is_ok(),
        "this wait was held for CaughtUp and answered it, so asserting that condition passes: \
         {answered:?}",
    );
    assert!(
        assert_waited_for(WaitConditionNet::WalkComplete, answered.clone()).is_err(),
        "the same reply names CaughtUp, so a case reading it as a completed walk must be told \
         it read the wrong wait: {answered:?}",
    );
    Ok(())
}
