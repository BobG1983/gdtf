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

fn assert_no_reply_yet(reply_rx: &Receiver<QaResponse>, when: &str) {
    let received = reply_rx.try_recv();
    assert!(
        matches!(received, Err(TryRecvError::Empty)),
        "the pump answered {received:?} {when} — no reply may exist before a PNG has landed",
    );
}

fn drive_until_reply(app: &mut App, reply_rx: &Receiver<QaResponse>) -> Option<QaResponse> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Ok(reply) = reply_rx.try_recv() {
            return Some(reply);
        }
    }
    None
}

#[test]
fn a_stale_png_already_at_the_path_is_never_served() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let dir = EditorQaShotDir::new(tmp.path().to_path_buf());
    let name = ShotName::new("stale".to_owned());
    let path = first_capture_path(&dir, &name);
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
    assert_eq!(
        spawned_at,
        TEST_SETTLE + 2,
        "the capture must wait out the {TEST_SETTLE}-frame settle window after its claim \
         frame; it was spawned on frame {spawned_at}",
    );

    let reply = drive_until_reply(&mut app, &reply_rx);
    assert!(
        matches!(reply, Some(QaResponse::Error(QaError::Timeout))),
        "a stale PNG at the target path must be purged, never served — the capture times out, \
         got {reply:?}",
    );
}

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
    plant_garbage(&path);

    let reply = drive_until_reply(&mut app, &reply_rx);
    assert!(
        matches!(reply, Some(QaResponse::Error(QaError::Timeout))),
        "bytes that do not decode as a PNG must never come back attached, got {reply:?}",
    );
}

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
