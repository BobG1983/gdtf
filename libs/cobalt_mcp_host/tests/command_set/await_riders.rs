//! The await-ready rider, plus the control that a call carrying no rider still runs.

use cobalt_mcp_host::{
    command::McpCommand,
    test_support::{
        FAKE_COMMANDS, FakePhase, FakePoint, fake_app, fake_facts_loaded, fake_facts_unloaded,
        run_fake_command,
    },
};
use cobalt_mcp_protocol::command::{
    AwaitBudget, CommandAvailability, CommandName, CommandOutcome, RunOptions, UnavailableCode,
};

use crate::support::{POINT_ARGS, args, no_answer_yet, outcome, plain};

const fn await_for(seconds: u64) -> RunOptions {
    RunOptions::new(Some(AwaitBudget::new(seconds)), None)
}

// The refusal `fake.point` publishes for itself with nothing loaded.
fn point_refuses_unloaded() -> (UnavailableCode, String) {
    let availability = <FakePoint as McpCommand>::availability(&fake_facts_unloaded());
    let CommandAvailability::Unavailable { code, note } = availability else {
        unreachable!("fake.point refuses itself with nothing loaded, got {availability:?}");
    };
    (code, note.as_str().to_owned())
}

#[test]
fn an_await_ready_rider_holds_the_call_until_the_command_admits() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_unloaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePoint::NAME,
        &args(POINT_ARGS),
        &await_for(60),
    );

    app.update();
    no_answer_yet(&channel);

    app.insert_resource(fake_facts_loaded());
    app.update();

    let answered = outcome(&channel);
    assert!(
        matches!(answered, CommandOutcome::Ran { .. }),
        "a held call runs as soon as the facts admit it, got {answered:?}"
    );
}

#[test]
fn a_spent_await_budget_answers_the_command_s_own_refusal() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_unloaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePoint::NAME,
        &args(POINT_ARGS),
        &await_for(0),
    );

    let answered = outcome(&channel);
    let CommandOutcome::Unavailable { code, note } = answered else {
        unreachable!("a budget of zero decides once, got {answered:?}");
    };
    assert_eq!(
        (code, note.as_str().to_owned()),
        point_refuses_unloaded(),
        "a spent budget answers the refusal the last test produced, never a rider refusal"
    );
}

#[test]
fn an_await_ready_rider_on_a_name_the_host_does_not_know_answers_at_once() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_unloaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &CommandName::from_static("fake.nope"),
        &args("()"),
        &await_for(60),
    );

    let answered = outcome(&channel);
    assert!(
        matches!(answered, CommandOutcome::Unknown { .. }),
        "waiting cannot make a name appear, so an unknown name answers before any frame runs, \
         got {answered:?}"
    );
}

#[test]
fn the_default_riders_still_run_the_command() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &plain(),
    );

    app.update();

    assert!(matches!(outcome(&channel), CommandOutcome::Ran { .. }));
}

#[test]
fn a_held_call_is_answered_exactly_once() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_unloaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePoint::NAME,
        &args(POINT_ARGS),
        &await_for(60),
    );

    app.update();
    app.insert_resource(fake_facts_loaded());
    app.update();
    assert!(matches!(outcome(&channel), CommandOutcome::Ran { .. }));

    for _ in 0..8 {
        app.update();
    }
    assert!(
        channel.try_recv().is_err(),
        "a call that waited for admission still produces exactly one answer"
    );
}
