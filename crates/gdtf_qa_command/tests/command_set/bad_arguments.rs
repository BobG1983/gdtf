//! A payload that will not decode is answered `BadArguments` AT CLAIM TIME, with that
//! command's own derived schema attached, and never reaches the handler.

use gdtf_qa_command::{
    command::QaCommand,
    test_support::{
        FAKE_COMMANDS, FakeCell, FakePhase, fake_app, fake_facts_loaded, run_fake_command,
    },
};
use gdtf_qa_protocol::command::CommandOutcome;

use crate::support::{args, outcome, plain};

/// An unknown field is refused, and the attached schema is the argument type's own.
#[test]
fn an_unknown_field_is_answered_bad_arguments_with_the_command_s_schema() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("{\"nope\":1}"),
        &plain(),
    );

    app.update();

    let answered = outcome(&channel);
    let CommandOutcome::BadArguments { detail, schema } = answered else {
        unreachable!("an unknown field must answer BadArguments, got {answered:?}");
    };
    assert!(
        detail.as_str().contains("nope"),
        "the decoder names the offending field: {}",
        detail.as_str()
    );
    assert!(
        schema.as_str().contains("\"additionalProperties\":false"),
        "the attached schema is FakePhaseArgs' own closed record: {}",
        schema.as_str()
    );
}

/// A wrongly-typed field is refused with the schema of the type it failed against.
#[test]
fn a_wrongly_typed_field_is_answered_with_that_command_s_schema() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakeCell::NAME,
        &args("{\"cell\":\"over there\"}"),
        &plain(),
    );

    app.update();

    let answered = outcome(&channel);
    let CommandOutcome::BadArguments { schema, .. } = answered else {
        unreachable!("a wrongly-typed field must answer BadArguments, got {answered:?}");
    };
    assert!(
        schema.as_str().contains("\"cell\""),
        "the attached schema is FakeCellArgs', not FakePhaseArgs': {}",
        schema.as_str()
    );
}

/// The refusal happens at CLAIM time — the handler never runs, so nothing is left queued
/// and no later frame produces a second answer.
#[test]
fn a_bad_payload_never_reaches_the_handler() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("{\"nope\":1}"),
        &plain(),
    );

    app.update();
    let first = outcome(&channel);
    assert!(matches!(first, CommandOutcome::BadArguments { .. }));

    // Several more frames: a queued call would be swept and answered again, a handled one
    // would answer `Ran`. Neither happens, because the call was answered and dropped.
    for _ in 0..8 {
        app.update();
    }
    assert!(
        channel.try_recv().is_err(),
        "the refused call must produce exactly one answer"
    );
}
