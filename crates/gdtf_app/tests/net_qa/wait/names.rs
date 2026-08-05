use std::sync::mpsc::{Receiver, TryRecvError};

use gdtf_qa_protocol::{
    command::{CommandOutcome, RunOptions},
    message::QaResponse,
};

use super::support::SETTLE_FRAMES;
use crate::{
    battle_fixture::{menu_app_with_net_qa, run_request, send},
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
        !matches!(reply, QaResponse::Outcome(CommandOutcome::Unknown { .. })),
        "`wait` must be a REGISTERED name, not Unknown: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::BadArguments { .. })
        ),
        "`(condition:CaughtUp)` is the published argument shape and must deserialize: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::Unavailable { .. })
        ),
        "`wait` is available on every screen, so nothing may refuse it: {reply:?}",
    );
    assert!(
        matches!(reply, QaResponse::Outcome(CommandOutcome::Ran { .. })),
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
    let QaResponse::Outcome(CommandOutcome::BadArguments { detail, schema }) = reply else {
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
    let (mut app, tx) = menu_app_with_net_qa();
    let asked: Vec<(&str, Settles, Receiver<QaResponse>)> = [
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

    for _ in 0..SETTLE_FRAMES {
        app.update();
    }

    for (arguments, settles, reply) in asked {
        let answered = reply.try_recv();
        let as_asked = match settles {
            Settles::AtOnce => matches!(
                answered,
                Ok(QaResponse::Outcome(CommandOutcome::Ran { .. }))
            ),
            Settles::NotYet => matches!(answered, Err(TryRecvError::Empty)),
        };
        assert!(
            as_asked,
            "`{arguments}` must be a condition this host understands, and in the Menu it must \
             settle {settles:?}; got {answered:?}",
        );
    }
}
