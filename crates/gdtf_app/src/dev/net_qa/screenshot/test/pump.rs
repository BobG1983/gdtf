use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
};

use bevy::prelude::*;
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_protocol::{
    command::{AttachmentKind, CommandOutcome},
    ids::ShotName,
    message::{QaError, QaResponse},
};
use gdtf_screenshot::CapturePath;

use super::super::{
    path::{QaShotDir, ShotSequence, next_capture_path},
    payload::ScreenshotPayload,
    pump::{InFlightShots, ShotPollBudget, drive_screenshots},
};

fn pump_app(dir: PathBuf, budget: ShotPollBudget) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<PendingQueue<ScreenshotPayload>>();
    app.init_resource::<InFlightShots>();
    app.init_resource::<ShotSequence>();
    app.insert_resource(QaShotDir::new(dir));
    app.insert_resource(budget);
    app.add_systems(Update, drive_screenshots);
    app
}

fn enqueue(app: &mut App, name: Option<ShotName>) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<ScreenshotPayload>>()
        .push_new(ScreenshotPayload::new(name), responder);
    reply_rx
}

fn first_capture_path(dir: &QaShotDir, name: Option<&ShotName>) -> CapturePath {
    let mut seq = ShotSequence::default();
    next_capture_path(dir, name, &mut seq)
}

fn plant_decodable_png(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
    let seed = image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255]));
    let written = seed.save_with_format(&**path, image::ImageFormat::Png);
    assert!(
        written.is_ok(),
        "test setup: writing the seed PNG must succeed: {written:?}",
    );
}

fn plant_garbage(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
    let written = std::fs::write(&**path, b"this is not a png");
    assert!(
        written.is_ok(),
        "test setup: writing the garbage file must succeed: {written:?}",
    );
}

fn is_timeout(reply: &Result<QaResponse, TryRecvError>) -> bool {
    matches!(reply, Ok(QaResponse::Error(QaError::Timeout)))
}

fn attached_png(reply: &Result<QaResponse, TryRecvError>) -> Option<String> {
    let Ok(QaResponse::Outcome(CommandOutcome::Ran { attachments, .. })) = reply else {
        return None;
    };
    let attachment = attachments.first()?;
    if attachment.kind != AttachmentKind::Png {
        return None;
    }
    Some(attachment.path.as_str().to_owned())
}

#[test]
fn capture_that_never_lands_answers_timed_out() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(3));
    let reply_rx = enqueue(&mut app, Some(ShotName::new("never_lands".to_owned())));

    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "the pump must not answer on the claim frame — the reply is deferred",
    );

    for _ in 0..8 {
        app.update();
    }
    let reply = reply_rx.try_recv();
    assert!(
        is_timeout(&reply),
        "a capture whose PNG never lands must answer Timeout, got {reply:?}",
    );
}

#[test]
fn a_png_landing_after_the_claim_frame_is_attached_to_the_reply() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let dir = QaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("landed".to_owned());
    let path = first_capture_path(&dir, Some(&name));

    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(30));
    let reply_rx = enqueue(&mut app, Some(name));

    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "the pump must not answer on the claim frame — the reply is deferred",
    );

    plant_decodable_png(&path);

    app.update();
    let expected = path.to_string_lossy().into_owned();
    let reply = reply_rx.try_recv();
    assert_eq!(
        attached_png(&reply),
        Some(expected),
        "a PNG landing after the claim frame must come back as an attachment at its confined \
         path, got {reply:?}",
    );
}

#[test]
fn a_stale_png_already_at_the_path_is_never_served() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let dir = QaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("stale".to_owned());
    let path = first_capture_path(&dir, Some(&name));

    plant_decodable_png(&path);

    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(3));
    let reply_rx = enqueue(&mut app, Some(name));

    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "a pre-existing PNG must NOT be attached on the claim frame — never a stale guess",
    );

    for _ in 0..8 {
        app.update();
    }
    let reply = reply_rx.try_recv();
    assert!(
        is_timeout(&reply),
        "a stale PNG at the path must be purged, never served — the capture times out, got \
         {reply:?}",
    );
}

#[test]
fn an_undecodable_file_is_never_attached() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let dir = QaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("garbled".to_owned());
    let path = first_capture_path(&dir, Some(&name));

    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(3));
    let reply_rx = enqueue(&mut app, Some(name));

    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "the pump must not answer on the claim frame — the reply is deferred",
    );

    plant_garbage(&path);

    for _ in 0..8 {
        app.update();
    }
    let reply = reply_rx.try_recv();
    assert!(
        is_timeout(&reply),
        "an existing-but-undecodable file must never be attached — it times out, got {reply:?}",
    );
}
