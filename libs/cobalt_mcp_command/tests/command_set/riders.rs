//! The capture rider, answered through the drain the host registers with the hold.

use std::{cell::Cell, path::PathBuf, sync::mpsc::Receiver};

use bevy::prelude::*;
use cobalt_mcp_command::{
    command::McpCommand,
    dispatch::{CaptureHolds, CaptureTicket},
    test_support::{
        FAKE_COMMANDS, FakeFacts, FakePhase, FakePoint, fake_app, fake_facts_loaded,
        fake_facts_unloaded, run_fake_command,
    },
};
use cobalt_mcp_protocol::{
    command::{
        ArtifactPath, AttachmentKind, AwaitBudget, CaptureRider, CommandOutcome, RunOptions,
    },
    ids::ShotName,
    message::{McpResponse, McpSessionError},
};
use cobalt_screenshot::{
    CaptureQueue, CaptureSystems, PollCap, SettleFrames, ShotDir, ShotDirName,
};
use cobalt_test_utils::advance_until;

use crate::support::{POINT_ARGS, args, no_answer_yet, outcome, plain};

/// Frames the capture pump settles for before it spawns its readback.
const SHORT_SETTLE: u32 = 2;

/// Frames the capture pump polls for the PNG before it gives up.
const SHORT_POLL: u32 = 4;

/// Frames the drain is given to queue a shot no rider asked for.
const DRAIN_TURNS: u32 = 3;

/// Stem a named capture rider asks the host to write its shot under.
const RIDER_STEM: &str = "rider";

/// Stem the capture pipeline falls back to when a rider names none.
const DEFAULT_STEM: &str = "shot";

/// File stem the stand-in renderer writes its PNG under.
#[derive(Resource, Deref)]
struct LandingStem(&'static str);

fn capture_named(stem: &str) -> RunOptions {
    RunOptions::new(
        None,
        Some(CaptureRider::new(Some(ShotName::new(stem.to_owned())))),
    )
}

// One emptied directory per case and per process, so parallel cases and parallel runs never
// share a file.
fn case_shot_dir(case: &str) -> ShotDir {
    let dir = ShotDir::under_workspace_target(&ShotDirName::new(format!(
        "qa_command_rider_{case}_{}",
        std::process::id()
    )));
    drop(std::fs::remove_dir_all(dir.as_path()));
    dir
}

// The fake host with both capture tunables small and its own shot directory, so a shot that
// cannot land gives up fast.
fn capture_app(facts: FakeFacts, dir: &ShotDir) -> App {
    let mut app = fake_app(FAKE_COMMANDS, facts);
    app.insert_resource(SettleFrames::new(SHORT_SETTLE));
    app.insert_resource(PollCap::new(SHORT_POLL));
    app.insert_resource(dir.clone());
    app
}

// The same host with a stand-in for the renderer, so a rider's shot can land.
fn landing_app(facts: FakeFacts, dir: &ShotDir, stem: &'static str) -> App {
    let mut app = capture_app(facts, dir);
    app.insert_resource(LandingStem(stem));
    app.add_systems(Update, stand_in_for_the_renderer.after(CaptureSystems));
    app
}

// Nothing renders here, so this stands in for the capture's own PNG writer.
fn stand_in_for_the_renderer(dir: Res<ShotDir>, stem: Res<LandingStem>) {
    if !dir.is_dir() {
        return;
    }
    let path = dir.join(format!("{}_0.png", **stem));
    if path.exists() {
        return;
    }
    let frame = image::RgbaImage::from_pixel(2, 2, image::Rgba([7, 9, 11, 255]));
    drop(frame.save_with_format(&path, image::ImageFormat::Png));
}

// The host's answer to one call, taken off the channel the frame it arrives on. The wait has no
// budget, so a busy machine is never red.
fn wait_for_answer(app: &mut App, channel: &Receiver<McpResponse>) -> McpResponse {
    let landed = Cell::new(None);
    loop {
        advance_until(app, |_| {
            let arrived = channel.try_recv().ok();
            let answered = arrived.is_some();
            landed.set(arrived);
            answered
        });
        if let Some(answered) = landed.take() {
            return answered;
        }
    }
}

fn ran_body(channel: &Receiver<McpResponse>) -> String {
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

fn ran_with_one_png(answered: &McpResponse) -> (String, ArtifactPath) {
    let McpResponse::Outcome(CommandOutcome::Ran { reply, attachments }) = answered else {
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

// Check the attachment against the file the host's own pipeline landed.
fn assert_landed_png(attached: &ArtifactPath, dir: &ShotDir, stem: &str) {
    let png = PathBuf::from(attached.as_str());
    assert_eq!(
        png,
        dir.join(format!("{stem}_0.png")),
        "the attachment names the file the host's own drain landed"
    );
    assert!(
        png.exists(),
        "the rider attached {} but no file is there. The reply must name a PNG that exists",
        png.display()
    );
}

#[test]
fn a_capture_rider_holds_the_reply_until_its_shot_lands() {
    let dir = case_shot_dir("holds");
    let mut app = landing_app(fake_facts_loaded(), &dir, RIDER_STEM);
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
        &capture_named(RIDER_STEM),
    );

    app.update();

    let bare_body = ran_body(&bare);
    no_answer_yet(&shot);

    let answered = wait_for_answer(&mut app, &shot);
    let (body, attached) = ran_with_one_png(&answered);
    assert_eq!(
        body, bare_body,
        "the rider leaves the command's own reply body alone"
    );
    assert_landed_png(&attached, &dir, RIDER_STEM);
    drop(std::fs::remove_dir_all(dir.as_path()));
}

#[test]
fn a_capture_rider_whose_shot_never_lands_answers_timeout() {
    let dir = case_shot_dir("lost");
    let mut app = capture_app(fake_facts_loaded(), &dir);
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePhase::NAME,
        &args("()"),
        &capture_named("lost"),
    );

    app.update();
    no_answer_yet(&channel);

    let answered = wait_for_answer(&mut app, &channel);
    assert!(
        matches!(answered, McpResponse::Error(McpSessionError::Timeout)),
        "nothing writes a PNG here, so the shot times out in the pump and the rider answers \
         Timeout rather than the reply without it, got {answered:?}"
    );
    drop(std::fs::remove_dir_all(dir.as_path()));
}

#[test]
fn a_capture_rider_on_a_command_that_did_not_run_answers_without_a_shot() {
    let dir = case_shot_dir("unparsed");
    let mut app = capture_app(fake_facts_loaded(), &dir);
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

    for _ in 0..DRAIN_TURNS {
        app.update();
    }
    assert!(
        app.world()
            .resource::<CaptureQueue<CaptureTicket>>()
            .is_idle(),
        "the rider shoots after the command has run, so a command that did not run leaves the \
         host's capture queue with nothing to take"
    );
    assert!(
        app.world().resource::<CaptureHolds>().is_empty(),
        "the answered call is released rather than left waiting for a shot"
    );
    drop(std::fs::remove_dir_all(dir.as_path()));
}

#[test]
fn both_riders_on_one_call_wait_for_admission_and_then_attach() {
    let dir = case_shot_dir("both");
    let mut app = landing_app(fake_facts_unloaded(), &dir, DEFAULT_STEM);
    let channel = run_fake_command(
        &mut app,
        FAKE_COMMANDS,
        &FakePoint::NAME,
        &args(POINT_ARGS),
        &RunOptions::new(Some(AwaitBudget::new(60)), Some(CaptureRider::new(None))),
    );

    app.update();
    no_answer_yet(&channel);

    app.insert_resource(fake_facts_loaded());
    app.update();
    no_answer_yet(&channel);

    let answered = wait_for_answer(&mut app, &channel);
    let (_body, attached) = ran_with_one_png(&answered);
    assert_landed_png(&attached, &dir, DEFAULT_STEM);
    drop(std::fs::remove_dir_all(dir.as_path()));
}
