//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc
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

const SHOT_NAME: &str = "editor_shell";

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
    let lit = decoded
        .to_rgba8()
        .pixels()
        .filter(|pixel| pixel.0[0] > 0 || pixel.0[1] > 0 || pixel.0[2] > 0)
        .count();
    assert!(
        lit > 0,
        "the capture at {} is a fully black {}x{} frame — the editor's camera is not rendering \
         into the render target the capture reads, so the PNG shows nothing",
        png.display(),
        decoded.width(),
        decoded.height(),
    );
    Ok(())
}

#[test]
fn the_reply_waits_for_the_capture_that_never_lands() -> TestResult {
    let tmp = tempfile::TempDir::new()?;
    let (mut app, _port) = headless_editor_app(tmp.path().to_path_buf())?;
    advance_to_editing(&mut app);

    let reply_rx = enqueue_capture(&mut app, SHOT_NAME);

    let mut spawned_at = None;
    for frame in 1..=DRIVE_UPDATES {
        app.update();
        if spawned_captures(&mut app) > 0 {
            spawned_at = Some(frame);
            break;
        }
    }
    let spawned_at = spawned_at.ok_or("the pump never spawned a capture")?;
    assert!(
        spawned_at >= TEST_SETTLE + 2,
        "the capture must wait out the {TEST_SETTLE}-frame settle window before it is \
         spawned; it appeared on frame {spawned_at}",
    );

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

    let reply = drive_until_reply(&mut app, &reply_rx)?;
    assert!(
        matches!(reply, QaResponse::Error(QaError::Timeout)),
        "a capture whose PNG never lands must be answered Timeout, never an attachment; got \
         {reply:?}",
    );
    Ok(())
}
