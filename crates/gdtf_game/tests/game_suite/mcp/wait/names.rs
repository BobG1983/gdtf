use std::sync::mpsc::{Receiver, TryRecvError};

use cobalt_mcp_protocol::{
    command::{CommandOutcome, RunOptions},
    message::McpResponse,
};

use super::support::{PARKED_FRAMES, answered_within};
use crate::mcp::{
    battle_fixture::{menu_app_with_mcp, run_request, send},
    command_exchange::{WAIT, exchange, run},
    socket_support::{TestResult, game_app_listening},
};

/// Whether a condition already holds on the screen it was asked from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Settles {
    AtOnce,
    NotYet,
}

#[test]
fn wait_on_caught_up_is_a_registered_name_with_a_shape_the_host_accepts() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(WAIT, "(condition:CaughtUp)", RunOptions::default()),
    )?;
    assert!(
        !matches!(reply, McpResponse::Outcome(CommandOutcome::Unknown { .. })),
        "`wait` must be a REGISTERED name, not Unknown: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            McpResponse::Outcome(CommandOutcome::BadArguments { .. })
        ),
        "`(condition:CaughtUp)` is the published argument shape and must deserialize: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            McpResponse::Outcome(CommandOutcome::Unavailable { .. })
        ),
        "`wait` is available on every screen, so nothing may refuse it: {reply:?}",
    );
    assert!(
        matches!(reply, McpResponse::Outcome(CommandOutcome::Ran { .. })),
        "in the Menu there is no cursor and no act log, so CaughtUp already holds and the call \
         runs: {reply:?}",
    );
    Ok(())
}

#[test]
fn a_condition_this_host_does_not_offer_is_bad_arguments_with_the_wait_shape() -> TestResult {
    let reply = exchange(
        game_app_listening,
        run(WAIT, "(condition:Settled)", RunOptions::default()),
    )?;
    let McpResponse::Outcome(CommandOutcome::BadArguments { detail, schema }) = reply else {
        unreachable!("a condition name the host never published must be refused, got {reply:?}");
    };
    assert!(
        detail.as_str().contains("Settled") || detail.as_str().contains("variant"),
        "the fault must say which condition was wrong: {}",
        detail.as_str(),
    );
    assert!(
        schema.as_str().contains("WaitArgs"),
        "the refusal carries `wait`'s own argument shape so one round trip is enough to fix the \
         call: {}",
        schema.as_str(),
    );
    Ok(())
}

#[test]
fn every_published_condition_is_accepted_and_only_the_unmet_ones_park() {
    let (mut app, tx) = menu_app_with_mcp();
    let asked: Vec<(&str, Settles, Receiver<McpResponse>)> = [
        ("(condition:CaughtUp)", Settles::AtOnce),
        ("(condition:Phase(()))", Settles::AtOnce),
        ("(condition:LogAtLeast(0))", Settles::AtOnce),
        ("(condition:WalkComplete)", Settles::AtOnce),
        ("(condition:TurnChanged)", Settles::NotYet),
        ("(condition:BattleDecided)", Settles::NotYet),
        ("(condition:GenerationComplete)", Settles::NotYet),
    ]
    .into_iter()
    .map(|(arguments, settles)| (arguments, settles, send(&tx, run_request(WAIT, arguments))))
    .collect();

    for (arguments, settles, reply) in asked {
        match settles {
            Settles::AtOnce => {
                let answered = answered_within(&mut app, &reply);
                assert!(
                    matches!(answered, McpResponse::Outcome(CommandOutcome::Ran { .. })),
                    "`{arguments}` must be a condition this host understands, and in the Menu it \
                     must settle {settles:?}; got {answered:?}",
                );
            }
            Settles::NotYet => {
                for frame in 0..PARKED_FRAMES {
                    app.update();
                    assert_eq!(
                        reply.try_recv().err(),
                        Some(TryRecvError::Empty),
                        "`{arguments}` must be a condition this host understands, and in the \
                         Menu it must settle {settles:?}; it answered on frame {frame}",
                    );
                }
            }
        }
    }
}
