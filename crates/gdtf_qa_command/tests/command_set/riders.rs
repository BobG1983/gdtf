use std::sync::mpsc::Receiver;

use bevy::prelude::App;
use gdtf_qa_command::{
    command::QaCommand,
    dispatch::{CaptureHolds, RiderShot, ShotRequest},
    test_support::{
        FAKE_COMMANDS, FakeCell, FakePhase, fake_app, fake_facts_loaded, fake_facts_unloaded,
        run_fake_command,
    },
};
use gdtf_qa_protocol::{
    command::{
        ArtifactPath, AttachmentKind, AwaitBudget, CaptureRider, CommandAvailability, CommandName,
        CommandOutcome, RunOptions, UnavailableCode,
    },
    ids::ShotName,
    message::{QaError, QaResponse},
};

use crate::support::{args, no_answer_yet, outcome, plain};

const CELL_ARGS: &str = "(cell:(x:3,y:-4))";

const fn await_for(seconds: u64) -> RunOptions {
    RunOptions::new(Some(AwaitBudget::new(seconds)), None)
}

fn capture_named(stem: &str) -> RunOptions {
    RunOptions::new(
        None,
        Some(CaptureRider::new(Some(ShotName::new(stem.to_owned())))),
    )
}

fn shot_path(stem: &str) -> ArtifactPath {
    ArtifactPath::new(format!("target/{stem}_0.png"))
}

// The refusal `fake.cell` publishes for itself with nothing loaded.
fn cell_refuses_unloaded() -> (UnavailableCode, String) {
    let availability = <FakeCell as QaCommand>::availability(&fake_facts_unloaded());
    let CommandAvailability::Unavailable { code, note } = availability else {
        unreachable!("fake.cell refuses itself with nothing loaded, got {availability:?}");
    };
    (code, note.as_str().to_owned())
}

fn one_request(app: &mut App) -> ShotRequest {
    let mut holds = app.world_mut().resource_mut::<CaptureHolds>();
    let mut requests = holds.drain_requests();
    assert_eq!(
        requests.len(),
        1,
        "a capture rider asks the host for exactly one shot, got {requests:?}"
    );
    let Some(request) = requests.pop() else {
        unreachable!("the length check above already found the one request");
    };
    request
}

fn report_shot(app: &mut App, request: &ShotRequest, shot: RiderShot) {
    app.world_mut()
        .resource_mut::<CaptureHolds>()
        .complete(request.ticket(), shot);
}

fn ran_body(channel: &Receiver<QaResponse>) -> String {
    let answered = outcome(channel);
    let CommandOutcome::Ran { reply, attachments } = answered else {
        unreachable!("a plain call must run, got {answered:?}");
    };
    assert!(
        attachments.is_empty(),
        "a call with no capture rider carries no attachment: {attachments:?}"
    );
    reply.as_str().to_owned()
}

fn ran_with_one_png(channel: &Receiver<QaResponse>) -> (String, ArtifactPath) {
    let answered = outcome(channel);
    let CommandOutcome::Ran { reply, attachments } = answered else {
        unreachable!("a landed capture rider must run, got {answered:?}");
    };
    let Some(attachment) = attachments.first() else {
        unreachable!("a landed capture rider attaches its PNG: {attachments:?}");
    };
    assert_eq!(
        attachments.len(),
        1,
        "the rider appends one attachment, not several: {attachments:?}"
    );
    assert_eq!(
        attachment.kind,
        AttachmentKind::Png,
        "the rider's attachment is declared a PNG: {attachment:?}"
    );
    (reply.as_str().to_owned(), attachment.path.clone())
}

#[test]
fn an_await_ready_rider_holds_the_call_until_the_command_admits() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_unloaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakeCell::NAME,
        &args(CELL_ARGS),
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
        &FakeCell::NAME,
        &args(CELL_ARGS),
        &await_for(0),
    );

    let answered = outcome(&channel);
    let CommandOutcome::Unavailable { code, note } = answered else {
        unreachable!("a budget of zero decides once, got {answered:?}");
    };
    assert_eq!(
        (code, note.as_str().to_owned()),
        cell_refuses_unloaded(),
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
fn a_capture_rider_holds_the_reply_until_its_shot_lands() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let bare = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &plain(),
    );
    let shot = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &capture_named("rider"),
    );

    app.update();

    let bare_body = ran_body(&bare);
    no_answer_yet(&shot);

    let path = shot_path("rider");
    let request = one_request(&mut app);
    assert_eq!(
        request.name(),
        Some(&ShotName::new("rider".to_owned())),
        "the stem the caller asked for reaches the host"
    );
    report_shot(&mut app, &request, RiderShot::Landed(path.clone()));

    let (body, attached) = ran_with_one_png(&shot);
    assert_eq!(
        body, bare_body,
        "the rider leaves the command's own reply body alone"
    );
    assert_eq!(attached, path, "the attachment names the PNG that landed");
}

#[test]
fn a_capture_rider_whose_shot_never_lands_answers_timeout() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &capture_named("lost"),
    );

    app.update();
    no_answer_yet(&channel);
    let request = one_request(&mut app);
    report_shot(&mut app, &request, RiderShot::Lost);

    let answered = channel.try_recv().ok();
    assert!(
        matches!(answered, Some(QaResponse::Error(QaError::Timeout))),
        "a shot that never lands answers Timeout rather than the reply without it, got \
         {answered:?}"
    );
}

#[test]
fn a_capture_rider_on_a_command_that_did_not_run_answers_without_a_shot() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_loaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("(nope:1)"),
        &capture_named("unparsed"),
    );

    app.update();

    let answered = outcome(&channel);
    assert!(
        matches!(answered, CommandOutcome::BadArguments { .. }),
        "an outcome that is not Ran goes straight back to the caller, got {answered:?}"
    );

    let mut holds = app.world_mut().resource_mut::<CaptureHolds>();
    let requests = holds.drain_requests();
    assert!(
        requests.is_empty(),
        "the rider shoots after the command has run, so a command that did not run takes no shot, \
         got {requests:?}"
    );
    assert!(
        holds.is_empty(),
        "the answered call is released rather than left waiting for a shot"
    );
}

#[test]
fn both_riders_on_one_call_wait_for_admission_and_then_attach() {
    let mut app = fake_app(FAKE_COMMANDS, fake_facts_unloaded());
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakeCell::NAME,
        &args(CELL_ARGS),
        &RunOptions::new(Some(AwaitBudget::new(60)), Some(CaptureRider::new(None))),
    );

    app.update();
    no_answer_yet(&channel);

    app.insert_resource(fake_facts_loaded());
    app.update();
    no_answer_yet(&channel);

    let path = shot_path("qa_shot");
    let request = one_request(&mut app);
    report_shot(&mut app, &request, RiderShot::Landed(path.clone()));

    let (_body, attached) = ran_with_one_png(&channel);
    assert_eq!(
        attached, path,
        "one call carrying both riders waits, runs, and comes back with the PNG"
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
        &FakeCell::NAME,
        &args(CELL_ARGS),
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
