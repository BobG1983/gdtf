use gdtf_qa_command::{
    command::QaCommand,
    test_support::{FAKE_COMMANDS, FakePhase, fake_app, fake_facts_loaded, run_fake_command},
};
use gdtf_qa_protocol::command::{
    AwaitBudget, CaptureRider, CommandOutcome, RunOptions, UnavailableCode,
};

use crate::support::{args, outcome, plain};

fn refusal_for(options: RunOptions) -> (UnavailableCode, String) {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &options,
    );

    let answered = outcome(&channel);
    let CommandOutcome::Unavailable { code, note } = answered else {
        unreachable!("a rider must be refused, got {answered:?}");
    };
    (code, note.as_str().to_owned())
}

#[test]
fn an_await_ready_rider_is_refused_not_built() {
    let (code, note) = refusal_for(RunOptions::new(Some(AwaitBudget::new(5)), None));
    assert_eq!(code, UnavailableCode::NotBuilt);
    assert_eq!(note, "this build has no await_ready rider");
}

#[test]
fn a_capture_rider_is_refused_not_built() {
    let (code, note) = refusal_for(RunOptions::new(None, Some(CaptureRider::new(None))));
    assert_eq!(code, UnavailableCode::NotBuilt);
    assert_eq!(note, "this build has no capture rider");
}

#[test]
fn both_riders_together_are_refused_not_built() {
    let (code, note) = refusal_for(RunOptions::new(
        Some(AwaitBudget::new(1)),
        Some(CaptureRider::new(None)),
    ));
    assert_eq!(code, UnavailableCode::NotBuilt);
    assert_eq!(
        note,
        "this build has neither the await_ready nor the capture rider"
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
fn a_refused_rider_never_queues_the_call() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &RunOptions::new(Some(AwaitBudget::new(5)), None),
    );

    let first = outcome(&channel);
    assert!(matches!(first, CommandOutcome::Unavailable { .. }));

    for _ in 0..8 {
        app.update();
    }
    assert!(
        channel.try_recv().is_err(),
        "a route-time refusal produces exactly one answer"
    );
}
