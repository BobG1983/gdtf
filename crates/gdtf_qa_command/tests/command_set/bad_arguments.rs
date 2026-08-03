use gdtf_qa_command::{
    command::QaCommand,
    test_support::{
        FAKE_COMMANDS, FakeCell, FakePhase, fake_app, fake_facts_loaded, run_fake_command,
    },
};
use gdtf_qa_protocol::command::CommandOutcome;

use crate::support::{args, outcome, plain};

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

    for _ in 0..8 {
        app.update();
    }
    assert!(
        channel.try_recv().is_err(),
        "the refused call must produce exactly one answer"
    );
}
