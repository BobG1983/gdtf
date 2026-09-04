use std::path::PathBuf;

use bevy::prelude::*;
use cobalt_mcp_protocol::{
    command::{AttachmentKind, CaptureRider, CommandOutcome, RunOptions},
    ids::ShotName,
    message::{ProtocolVersion, QaError, QaRequest, QaResponse},
    ports::McpPort,
};
use cobalt_screenshot::{CaptureSystems, PollCap, SettleFrames, ShotDir, ShotDirName};

use crate::{
    harness::{advance_to_editing, editor_app_listening},
    hello::assert_hello_ok,
    names::{CAPTURE_SCREENSHOT, EDITOR_PHASE},
    socket::{Client, run_editor, run_editor_with},
    support::{TestError, TestResult},
};

/// Frames the pump settles for before it spawns the readback.
const SHORT_SETTLE: u32 = 2;

/// Frames the pump polls for the PNG before it gives up.
const SHORT_POLL: u32 = 4;

/// Stem the capture rider asks the editor to write its shot under.
const RIDER_SHOT_NAME: &str = "editor_rider_shot";

// Both capture tunables small, the app advanced into editing, and a negotiated client on its
// listener.
fn settled_client(mut app: App, port: McpPort) -> Result<(App, Client), TestError> {
    app.insert_resource(SettleFrames::new(SHORT_SETTLE));
    app.insert_resource(PollCap::new(SHORT_POLL));
    advance_to_editing(&mut app);
    let mut client = Client::connect(port)?;
    let hello = client.exchange(&mut app, &QaRequest::Hello(ProtocolVersion::CURRENT))?;
    assert_hello_ok(&hello);
    Ok((app, client))
}

// An editing app with both capture tunables small, so a shot that cannot land gives up fast.
// Nothing renders in this harness, so nothing ever writes a PNG.
fn capture_app_and_client() -> Result<(App, Client), TestError> {
    let (app, port) = editor_app_listening()?;
    settled_client(app, port)
}

// The same app writing into this case's own directory, with a stand-in for the renderer, so a
// rider's shot can land.
fn landing_capture_app_and_client() -> Result<(App, Client), TestError> {
    let dir = rider_shot_dir();
    drop(std::fs::remove_dir_all(dir.as_path()));
    let (mut app, port) = editor_app_listening()?;
    app.insert_resource(dir);
    app.add_systems(Update, stand_in_for_the_renderer.after(CaptureSystems));
    settled_client(app, port)
}

// One directory per process, so two runs of this suite never share a file.
fn rider_shot_dir() -> ShotDir {
    ShotDir::under_workspace_target(&ShotDirName::new(format!(
        "qa_screenshot_editor_rider_{}",
        std::process::id()
    )))
}

// Nothing renders here, so this stands in for the capture's own PNG writer.
fn stand_in_for_the_renderer(dir: Res<ShotDir>) {
    if !dir.is_dir() {
        return;
    }
    let path = dir.join(format!("{RIDER_SHOT_NAME}_0.png"));
    if path.exists() {
        return;
    }
    let frame = image::RgbaImage::from_pixel(2, 2, image::Rgba([7, 9, 11, 255]));
    drop(frame.save_with_format(&path, image::ImageFormat::Png));
}

fn rider_named(stem: &str) -> RunOptions {
    RunOptions::new(
        None,
        Some(CaptureRider::new(Some(ShotName::new(stem.to_owned())))),
    )
}

fn ran_body_text(reply: &QaResponse) -> Result<String, TestError> {
    let QaResponse::Outcome(CommandOutcome::Ran { reply: body, .. }) = reply else {
        return Err(format!("expected a Ran outcome for `{EDITOR_PHASE}`, got {reply:?}").into());
    };
    Ok(body.as_str().to_owned())
}

#[test]
fn a_capture_the_headless_editor_cannot_land_reaches_the_pump_and_times_out() -> TestResult {
    let (mut app, mut client) = capture_app_and_client()?;

    let reply = client.exchange(&mut app, &run_editor(CAPTURE_SCREENSHOT, "()"))?;

    assert!(
        matches!(reply, QaResponse::Error(QaError::Timeout)),
        "with nothing writing a PNG the editor's capture must reach the pump and give up there, \
         answering Timeout rather than refusing; got {reply:?}",
    );
    assert!(
        !matches!(reply, QaResponse::Outcome(CommandOutcome::Unknown { .. })),
        "capture.screenshot must be a REGISTERED name on the editor host, not Unknown: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::BadArguments { .. })
        ),
        "`()` must deserialize into the command's args, whose only field is optional: {reply:?}",
    );
    assert!(
        !matches!(
            reply,
            QaResponse::Outcome(CommandOutcome::Unavailable { .. })
        ),
        "the command needs neither the authoring scene nor a tab, so nothing may refuse it: \
         {reply:?}",
    );
    Ok(())
}

#[test]
fn a_capture_rider_on_an_editor_command_attaches_the_png_the_editor_took() -> TestResult {
    let (mut app, mut client) = landing_capture_app_and_client()?;
    let dir = rider_shot_dir();

    client.send(&run_editor(EDITOR_PHASE, "()"))?;
    let plain = client.read(&mut app)?;
    let plain_body = ran_body_text(&plain)?;

    client.send(&run_editor_with(
        EDITOR_PHASE,
        "()",
        rider_named(RIDER_SHOT_NAME),
    ))?;
    let with_rider = client.read(&mut app)?;

    assert!(
        !matches!(with_rider, QaResponse::Error(QaError::Timeout)),
        "the editor drains the capture holds it registers, so a rider must answer rather than \
         wait out the caller's reply timeout; got {with_rider:?}",
    );
    let QaResponse::Outcome(CommandOutcome::Ran { reply, attachments }) = &with_rider else {
        return Err(format!("a capture rider runs its command first, got {with_rider:?}").into());
    };
    assert_eq!(
        reply.as_str(),
        plain_body,
        "the rider leaves the command's own reply body alone",
    );
    assert_eq!(
        attachments.len(),
        1,
        "the rider appends exactly one attachment: {attachments:?}",
    );
    let Some(attachment) = attachments.first() else {
        return Err(format!("the length check above found one: {attachments:?}").into());
    };
    assert_eq!(
        attachment.kind,
        AttachmentKind::Png,
        "the rider's attachment is declared a PNG: {attachment:?}",
    );
    let png = PathBuf::from(attachment.path.as_str());
    assert!(
        png.starts_with(dir.as_path()),
        "the rider's shot must land inside {}, not at {}",
        dir.display(),
        png.display(),
    );
    assert!(
        png.exists(),
        "the editor attached {} but no file is there. The reply must name a PNG that exists",
        png.display(),
    );
    drop(std::fs::remove_dir_all(dir.as_path()));
    Ok(())
}
