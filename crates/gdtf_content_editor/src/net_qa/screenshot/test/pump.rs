//! The pump behaviours the wire-level integration suite cannot reach (GTW-880).
//!
//! `tests/net_qa_editor_screenshot/` proves the reply waits for a capture on the REAL editor
//! over the REAL socket, but both of its tests run against a FRESH temp directory, so neither
//! can ever put a file at the pump's target path before the capture spawns. These three fill
//! that hole, driving the REAL [`drive_editor_screenshots`](super::super::pump) pump:
//!
//! 1. [`a_stale_png_already_at_the_path_is_never_served`] — a decodable PNG left at the exact
//!    target path by an earlier run is DELETED before the capture spawns and never answered
//!    `Saved`. This is the one mechanism the ordering contract rests on that the wire suite
//!    cannot exercise, and it is reachable in production: the confinement directory defaults
//!    to `target/editor_qa_screenshots` and the sequence counter restarts at zero every
//!    process, so a second editor run asking for the same name targets the first run's file.
//! 2. [`an_undecodable_file_at_the_path_is_not_reported_saved`] — bytes that exist and are
//!    non-empty but do not decode are rejected, so the reply still waits for a real PNG.
//! 3. [`a_png_landing_after_the_capture_is_spawned_is_reported_saved`] — the positive half,
//!    on a runner with no GPU at all: the pump answers `Saved` once, and only once, the PNG is
//!    actually there.

use std::sync::mpsc::{Receiver, TryRecvError};

use bevy::prelude::*;
use gdtf_qa_protocol::{
    command::{AttachmentKind, CommandOutcome},
    ids::ShotName,
    message::{QaError, QaResponse},
};

use super::{
    super::path::EditorQaShotDir,
    support::{
        DRIVE_UPDATES, TEST_SETTLE, drive_until_spawned, enqueue, first_capture_path,
        plant_decodable_png, plant_garbage, pump_app,
    },
};

/// Assert nothing has been answered yet — the pump's reply is genuinely deferred.
fn assert_no_reply_yet(reply_rx: &Receiver<QaResponse>, when: &str) {
    let received = reply_rx.try_recv();
    assert!(
        matches!(received, Err(TryRecvError::Empty)),
        "the pump answered {received:?} {when} — no reply may exist before a PNG has landed",
    );
}

/// Drive frames until a reply arrives, reporting what it was.
fn drive_until_reply(app: &mut App, reply_rx: &Receiver<QaResponse>) -> Option<QaResponse> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Ok(reply) = reply_rx.try_recv() {
            return Some(reply);
        }
    }
    None
}

/// A decodable PNG ALREADY at the pump's target path — an earlier run's leftover at a reused
/// sequence number — is purged when the capture spawns and is never served as this capture's
/// output. With no fresh capture landing, the pump reports the typed timeout instead.
///
/// Fails if the delete-before-spawn purge is removed: the very first poll would find those
/// stale bytes decodable and answer `Saved` with the previous run's image.
#[test]
fn a_stale_png_already_at_the_path_is_never_served() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let dir = EditorQaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("stale".to_owned());
    let path = first_capture_path(&dir, &name);
    // The stale, decodable PNG sits at the EXACT path this capture will target.
    plant_decodable_png(&path);

    let mut app = pump_app(tmp.path().to_path_buf());
    let reply_rx = enqueue(&mut app, name);

    let Some(spawned_at) = drive_until_spawned(&mut app) else {
        unreachable!("the pump never spawned a capture");
    };
    assert!(
        !path.exists(),
        "the stale file at {} must be deleted before the capture spawns",
        path.display(),
    );
    assert_no_reply_yet(&reply_rx, "on the frame its capture was spawned");
    // The request is claimed on frame 1 and deliberately not advanced that frame, the settle
    // countdown then costs `TEST_SETTLE` frames, and the spawn happens on the frame the
    // countdown expires — so the capture appears on frame `TEST_SETTLE + 2`, never earlier.
    assert_eq!(
        spawned_at,
        TEST_SETTLE + 2,
        "the capture must wait out the {TEST_SETTLE}-frame settle window after its claim \
         frame; it was spawned on frame {spawned_at}",
    );

    // Nothing writes a PNG here (no render device), so the pump must time out rather than
    // reach back for the bytes it purged.
    let reply = drive_until_reply(&mut app, &reply_rx);
    assert!(
        matches!(reply, Some(QaResponse::Error(QaError::Timeout))),
        "a stale PNG at the target path must be purged, never served — the capture times out, \
         got {reply:?}",
    );
}

/// A file that exists and is non-empty but does NOT decode as a PNG (a truncated or
/// mid-flush write) is rejected: the pump keeps polling and times out rather than reporting a
/// capture no client can read.
#[test]
fn an_undecodable_file_at_the_path_is_never_attached() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let dir = EditorQaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("garbled".to_owned());
    let path = first_capture_path(&dir, &name);

    let mut app = pump_app(tmp.path().to_path_buf());
    let reply_rx = enqueue(&mut app, name);

    let Some(_spawned_at) = drive_until_spawned(&mut app) else {
        unreachable!("the pump never spawned a capture");
    };
    // Planted AFTER the spawn, so the purge cannot remove it: this is the half-written file
    // case, not the stale-leftover case.
    plant_garbage(&path);

    let reply = drive_until_reply(&mut app, &reply_rx);
    assert!(
        matches!(reply, Some(QaResponse::Error(QaError::Timeout))),
        "bytes that do not decode as a PNG must never come back attached, got {reply:?}",
    );
}

/// The positive half, with no GPU involved: once a real PNG is at the target path the pump
/// answers `Saved` naming that exact path — and not one frame before the file was there.
///
/// The wire suite's landing proof needs a wgpu adapter and skips without one; this keeps the
/// "a PNG lands, then the reply comes" half covered on every runner.
#[test]
fn a_png_landing_after_the_capture_is_spawned_is_reported_saved() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let dir = EditorQaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("landed".to_owned());
    let path = first_capture_path(&dir, &name);

    let mut app = pump_app(tmp.path().to_path_buf());
    let reply_rx = enqueue(&mut app, name);

    let Some(_spawned_at) = drive_until_spawned(&mut app) else {
        unreachable!("the pump never spawned a capture");
    };
    assert_no_reply_yet(&reply_rx, "on the frame its capture was spawned");

    // The stand-in for the GPU flushing THIS capture's frame, on a later frame than the spawn.
    plant_decodable_png(&path);

    let expected = path.to_string_lossy().into_owned();
    let reply = drive_until_reply(&mut app, &reply_rx);
    let attached = match &reply {
        Some(QaResponse::Outcome(CommandOutcome::Ran { attachments, .. })) => attachments
            .first()
            .filter(|attachment| attachment.kind == AttachmentKind::Png)
            .map(|attachment| attachment.path.as_str().to_owned()),
        _ => None,
    };
    assert_eq!(
        attached,
        Some(expected),
        "a PNG landing after the capture was spawned must come back attached at that exact \
         path, got {reply:?}",
    );
}
