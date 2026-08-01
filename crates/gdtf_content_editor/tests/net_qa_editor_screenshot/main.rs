//! GTW-880: the editor's capture pump answers only after the capture has landed (GTW-943
//! retargeted the drive off the deleted `TakeScreenshot` request onto the pump's own queue).
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
//! so the doc survives a feature-off build — the GTW-804 `net_qa_hello` precedent) compiles
//! the whole dir-form suite to an empty crate without the feature. Run it with the feature
//! on: `cargo test -p gdtf_content_editor --features net_qa --test net_qa_editor_screenshot`.
//!
//! Everything on the host side goes through the production path: the editor is the REAL
//! `MapEditorPlugin` reaching `EditorState::Editing` through its own `Load` pass and
//! `OnEnter(Editing)` lifecycle, with a live `AssetServer` — no hand-inserted model state, no
//! `MinimalPlugins` — and the capture is claimed off the SAME `PendingQueue` the router used
//! to push onto, by the same pump, answering through the same `Responder`. What is gone is
//! only the socket in front of it: the `TakeScreenshot` request went with the rest of the
//! pre-command vocabulary, and the editor's own capture command is the next editor ticket.
//!
//! Two tests, one per half of the contract:
//!
//! 1. [`a_claimed_capture_lands_a_png_on_disk`] — the capture proof, on a REAL
//!    wgpu device: a capture is queued under a caller-chosen name and a PNG file exists,
//!    non-empty, PNG-decodable, and NOT A BLANK FRAME, at the path the reply attaches. The blank
//!    check is GTW-922: the capture used to land a real, decodable, entirely black PNG because it
//!    read a render target no camera drew into, and every assertion here except that one passed.
//!    GPU-guarded (GTW-527): a runner with no adapter logs a skip and returns before the render
//!    app is built.
//! 2. [`the_reply_waits_for_the_capture_that_never_lands`] — the ordering proof, on the
//!    no-renderer editor: at the moment the capture is spawned, with the app FROZEN so the
//!    pump cannot make progress, no reply exists and no file has been written; and because
//!    that app can never flush a PNG, the pump answers `QaError::Timeout` — never a saved path.
//!    A handler that replied on the claim frame, or that assumed its capture landed, fails
//!    both halves.
//!
//! ## Members (one concern per file)
//!
//! - [`support`] — the shared aliases, loop caps and injected tunables.
//! - [`harness`] — the two real editor apps and the drive-to-`Editing` driver.
//! - [`enqueue`] — the drive: one capture onto the real queue, one reply channel.
//! - [`source`] — GTW-917: the capture source the REAL editor app ends up with, so the
//!   shipped choice cannot change silently.
#![cfg(all(debug_assertions, feature = "net_qa"))]

mod enqueue;
mod harness;
mod source;
mod support;

use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, TryRecvError},
};

use bevy::prelude::*;
use gdtf_qa_protocol::{
    command::{AttachmentKind, CommandOutcome},
    message::{QaError, QaResponse},
};
use gdtf_test_utils::gpu_probe::gpu_adapter_probe;

use crate::{
    enqueue::enqueue_capture,
    harness::{advance_to_editing, gpu_editor_app, headless_editor_app, spawned_captures},
    support::{DRIVE_UPDATES, TEST_SETTLE, TestError, TestResult},
};

/// The file stem both tests ask for — proves the wire-supplied name reaches the path.
const SHOT_NAME: &str = "editor_shell";

/// Drive one frame per iteration until the pump answers.
fn drive_until_reply(app: &mut App, rx: &Receiver<QaResponse>) -> Result<QaResponse, TestError> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        match rx.try_recv() {
            Ok(reply) => return Ok(reply),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => break,
        }
    }
    Err("the editor never answered the queued capture".into())
}

/// The PNG path a landed reply attached, if it is one.
fn attached_png(reply: &QaResponse) -> Option<String> {
    let QaResponse::Outcome(CommandOutcome::Ran { attachments, .. }) = reply else {
        return None;
    };
    let attachment = attachments.first()?;
    if attachment.kind != AttachmentKind::Png {
        return None;
    }
    Some(attachment.path.as_str().to_owned())
}

/// Clause 1 — a claimed capture LANDS a PNG.
///
/// A capture queued under a caller-chosen name is claimed by the real pump, and the reply
/// attaches a path at which a real file exists, is non-empty, and decodes as a PNG. Every
/// assertion below reads the file the running editor wrote; none of them is satisfied by the
/// reply alone.
#[test]
fn a_claimed_capture_lands_a_png_on_disk() -> TestResult {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP a_claimed_capture_lands_a_png_on_disk: no usable wgpu adapter \
             (GPU-less runner). The ordering half of the contract is covered by \
             the_reply_waits_for_the_capture_that_never_lands, which needs no GPU.",
        );
        return Ok(());
    }
    let tmp = tempfile::TempDir::new()?;
    let (mut app, _port) = gpu_editor_app(tmp.path().to_path_buf())?;
    advance_to_editing(&mut app);

    let reply_rx = enqueue_capture(&mut app, SHOT_NAME);
    let reply = drive_until_reply(&mut app, &reply_rx)?;

    let Some(saved) = attached_png(&reply) else {
        return Err(format!(
            "the editor must attach the PNG once its capture lands, not {reply:?}"
        )
        .into());
    };
    let png = PathBuf::from(saved.as_str());
    assert!(
        png.starts_with(tmp.path()),
        "the capture must land inside the confinement directory, not at {}",
        png.display(),
    );
    assert!(
        png.file_name()
            .is_some_and(|stem| { stem.to_string_lossy().starts_with(SHOT_NAME) }),
        "the wire-supplied name must reach the file name, which was {}",
        png.display(),
    );
    assert!(
        png.exists(),
        "the editor attached {} but no file is there — the reply must name a PNG that exists",
        png.display(),
    );
    let bytes = std::fs::read(&png)?;
    assert!(
        !bytes.is_empty(),
        "the PNG at {} is empty — a zero-byte file is not a landed capture",
        png.display(),
    );
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png);
    let Ok(decoded) = decoded else {
        return Err(format!(
            "the {} bytes at {} do not decode as a PNG: {decoded:?}",
            bytes.len(),
            png.display(),
        )
        .into());
    };
    // GTW-922: the capture must hold PIXELS, not an untouched texture. A `Screenshot` of an
    // offscreen render target no camera renders into copies out zeroed texture memory, and the
    // result is a real, correctly-sized, perfectly decodable PNG in which every pixel is black —
    // which is exactly what the shipped editor returned. Nothing above this line can tell that
    // apart from a screenshot of the app.
    let lit = decoded
        .to_rgba8()
        .pixels()
        .filter(|pixel| pixel.0[0] > 0 || pixel.0[1] > 0 || pixel.0[2] > 0)
        .count();
    assert!(
        lit > 0,
        "the capture at {} is a fully black {}x{} frame — the editor's camera is not rendering \
         into the render target the capture reads (GTW-922), so the PNG shows nothing",
        png.display(),
        decoded.width(),
        decoded.height(),
    );
    Ok(())
}

/// Clause 2 — the reply comes AFTER the capture, never before.
///
/// Two assertions, both of which a reply-first handler fails:
///
/// 1. At the moment the pump spawns its capture — with the app FROZEN, so the pump cannot
///    advance a single frame further — no reply has been sent and the confinement directory
///    holds no file. Anything arriving in that window was sent before any capture could land.
/// 2. This app has no render backend, so its capture can NEVER flush a PNG. The pump must
///    therefore answer `QaError::Timeout`. A handler that replied immediately, or that
///    assumed its capture landed, would attach a path holding nothing.
///
/// The capture also has to wait out the settle window before it is spawned at all, which the
/// frame count asserts.
#[test]
fn the_reply_waits_for_the_capture_that_never_lands() -> TestResult {
    let tmp = tempfile::TempDir::new()?;
    let (mut app, _port) = headless_editor_app(tmp.path().to_path_buf())?;
    advance_to_editing(&mut app);

    let reply_rx = enqueue_capture(&mut app, SHOT_NAME);

    // Drive frame by frame until the pump spawns its capture, counting the frames it took.
    let mut spawned_at = None;
    for frame in 1..=DRIVE_UPDATES {
        app.update();
        if spawned_captures(&mut app) > 0 {
            spawned_at = Some(frame);
            break;
        }
    }
    let spawned_at = spawned_at.ok_or("the pump never spawned a capture")?;
    // The request cannot be claimed before the first frame, and the settle window costs
    // `TEST_SETTLE` more frames after that, with the spawn on the frame the countdown
    // expires — so the earliest possible spawn frame is `TEST_SETTLE + 2`.
    assert!(
        spawned_at >= TEST_SETTLE + 2,
        "the capture must wait out the {TEST_SETTLE}-frame settle window before it is \
         spawned; it appeared on frame {spawned_at}",
    );

    // FROZEN: nothing ticks the app from here, so the pump cannot make progress and no
    // legitimate reply can appear. Anything that arrives was sent earlier than the contract
    // allows.
    match reply_rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        Ok(early) => {
            return Err(format!(
                "the editor replied {early:?} at the moment its capture was spawned — before \
                 any PNG could have landed"
            )
            .into());
        }
        Err(TryRecvError::Disconnected) => {
            return Err("the reply channel closed before the editor answered".into());
        }
    }
    let written = std::fs::read_dir(tmp.path())?.count();
    assert_eq!(
        written, 0,
        "no file may exist in the confinement directory at the moment the capture is spawned",
    );

    // Drive on. This app has no render backend, so the capture can never flush a PNG — and
    // the pump reports that as a typed timeout rather than claiming a save.
    let reply = drive_until_reply(&mut app, &reply_rx)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::Timeout)),
        "a capture whose PNG never lands must be answered Timeout, never an attachment; got \
         {reply:?}",
    );
    Ok(())
}
