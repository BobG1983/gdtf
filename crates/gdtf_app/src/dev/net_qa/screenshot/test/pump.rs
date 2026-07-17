//! Headless pump tests (GTW-740): drive the REAL [`drive_screenshots`] pump on a
//! `MinimalPlugins` app, confined to a temp directory so no test artifact reaches the source
//! tree.
//!
//! The ONLY thing stubbed is the genuinely-external render boundary — the OS-level window
//! capture cannot run headlessly, so the spawned
//! [`Screenshot`](bevy::render::view::window::screenshot::Screenshot) never produces a PNG.
//! Everything else runs the real pump: the poll-before-claim ordering, the unique-path
//! derivation, the delete-before-spawn purge, the poll loop, the timeout accounting, and the
//! decode-verify + reply. The tests stand in for the GPU by planting a file at the pump's
//! exact target path — and pin four behaviours: the deferral (never a claim-frame reply), a
//! PNG that lands AFTER the claim frame is reported Saved, a STALE PNG already at the path is
//! purged and never served, and an existing-but-UNDECODABLE file is rejected (not "saved").

use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
};

use bevy::prelude::*;
use gdtf_qa_protocol::{
    envelope::{QaResponse, ScreenshotResult},
    ids::ShotName,
};
use gdtf_screenshot::CapturePath;

use super::super::{
    path::{QaShotDir, ShotSequence, next_capture_path},
    pump::{InFlightShots, ShotPollBudget, drive_screenshots},
};
use crate::dev::net_qa::{
    channel::Responder,
    pending::{PendingQueue, ScreenshotPayload},
};

/// Build a `MinimalPlugins` app with the REAL pump registered, confined to `dir`, and the
/// given poll budget.
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

/// Enqueue a screenshot request exactly as the router would, returning the reply channel.
fn enqueue(app: &mut App, name: Option<ShotName>) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<ScreenshotPayload>>()
        .push_new(ScreenshotPayload::new(name), responder);
    reply_rx
}

/// The path the pump writes the FIRST claimed capture to — computed through the REAL
/// [`next_capture_path`] with a fresh [`ShotSequence`], so it matches what the pump's own
/// sequence resource (starting at its [`Default`]) produces for its first claim.
fn first_capture_path(dir: &QaShotDir, name: Option<&ShotName>) -> CapturePath {
    let mut seq = ShotSequence::default();
    next_capture_path(dir, name, &mut seq)
}

/// Write a real, decodable PNG at `path` — the test's stand-in for the GPU flushing THIS
/// capture's frame to disk.
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

/// Write NON-PNG bytes at `path` — a truncated / garbage file the decode-verify must reject
/// (exists + non-empty is not enough to be "saved").
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

/// A capture whose PNG never lands (no GPU) is answered
/// [`TimedOut`](ScreenshotResult::TimedOut) once its frame budget elapses — and NOT before,
/// proving the reply is genuinely deferred across frames.
#[test]
fn capture_that_never_lands_answers_timed_out() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(3));
    let reply_rx = enqueue(&mut app, Some(ShotName::new("never_lands".to_owned())));

    // Claim frame: poll-before-claim means the request is claimed but NOT polled this frame,
    // so no reply — proving the response is deferred, not answered on the claim frame.
    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "the pump must not answer on the claim frame — the reply is deferred",
    );

    // With no GPU the PNG never lands, so the poll ticks the budget down to a timeout.
    for _ in 0..8 {
        app.update();
    }
    let reply = reply_rx.try_recv();
    assert!(
        matches!(
            reply,
            Ok(QaResponse::Screenshot(ScreenshotResult::TimedOut))
        ),
        "a capture whose PNG never lands must answer TimedOut, got {reply:?}",
    );
}

/// A PNG that lands at the confined path AFTER the claim frame is answered
/// [`Saved`](ScreenshotResult::Saved) at that exact path — the pump's poll + decode-verify +
/// reply all run for real, on a later frame than the claim.
#[test]
fn a_png_landing_after_the_claim_frame_is_reported_saved() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let dir = QaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("landed".to_owned());
    let path = first_capture_path(&dir, Some(&name));

    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(30));
    let reply_rx = enqueue(&mut app, Some(name));

    // Claim frame: the capture is spawned + tracked, but the deferred reply has not fired.
    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "the pump must not answer on the claim frame — the reply is deferred",
    );

    // Simulate the GPU flushing THIS capture's PNG on a LATER frame (the OS-level render is
    // the only stubbed boundary; the poll + decode + reply run for real).
    plant_decodable_png(&path);

    app.update();
    let expected = path.to_string_lossy().into_owned();
    let reply = reply_rx.try_recv();
    assert!(
        matches!(
            &reply,
            Ok(QaResponse::Screenshot(ScreenshotResult::Saved(saved))) if **saved == expected
        ),
        "a PNG landing after the claim frame must be reported Saved at its confined path, got \
         {reply:?}",
    );
}

/// A decodable PNG ALREADY sitting at the pump's target path (a leftover from a prior run at
/// a reused sequence) is PURGED on claim and never served — with no fresh capture, the pump
/// times out rather than replying Saved from stale bytes. Fails if delete-before-spawn (or
/// the poll-before-claim deferral) is removed.
#[test]
fn a_stale_png_already_at_the_path_is_never_served() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let dir = QaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("stale".to_owned());
    let path = first_capture_path(&dir, Some(&name));

    // The stale, decodable PNG is at the EXACT path the pump will target.
    plant_decodable_png(&path);

    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(3));
    let reply_rx = enqueue(&mut app, Some(name));

    // Claim frame: the stale file is purged; the reply is deferred (never a same-frame guess).
    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "a pre-existing PNG must NOT be reported saved on the claim frame — never a stale guess",
    );

    // No fresh capture lands (no GPU), so the pump times out rather than serving stale bytes.
    for _ in 0..8 {
        app.update();
    }
    let reply = reply_rx.try_recv();
    assert!(
        matches!(
            reply,
            Ok(QaResponse::Screenshot(ScreenshotResult::TimedOut))
        ),
        "a stale PNG at the path must be purged, never served — the capture times out, got \
         {reply:?}",
    );
}

/// A file that exists and is non-empty but does NOT decode as a PNG (a truncated / mid-flush
/// write) is rejected — the pump keeps polling and times out, never reporting it Saved. Fails
/// if the decode-verify is weakened to a bare `exists()` check (the clause justifying the
/// `image` dependency).
#[test]
fn an_undecodable_file_is_not_reported_saved() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let dir = QaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("garbled".to_owned());
    let path = first_capture_path(&dir, Some(&name));

    let mut app = pump_app(tmp.path().to_path_buf(), ShotPollBudget::new(3));
    let reply_rx = enqueue(&mut app, Some(name));

    // Claim frame: deferred (poll-before-claim), and the fresh path is purged of nothing.
    app.update();
    assert!(
        matches!(reply_rx.try_recv(), Err(TryRecvError::Empty)),
        "the pump must not answer on the claim frame — the reply is deferred",
    );

    // A non-PNG file appears at the path. `exists()` + non-empty is TRUE, but the decode fails
    // — so the pump must keep polling and time out, never reporting it Saved.
    plant_garbage(&path);

    for _ in 0..8 {
        app.update();
    }
    let reply = reply_rx.try_recv();
    assert!(
        matches!(
            reply,
            Ok(QaResponse::Screenshot(ScreenshotResult::TimedOut))
        ),
        "an existing-but-undecodable file must never be reported Saved — it times out, got \
         {reply:?}",
    );
}
