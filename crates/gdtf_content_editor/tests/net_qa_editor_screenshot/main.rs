//! Editor screenshot net-QA integration (debug + `net_qa`).
#![cfg(debug_assertions)]

mod enqueue;
mod harness;
mod source;
mod support;

use std::path::PathBuf;

use bevy::prelude::*;
use gdtf_screenshot::CaptureOutcome;
use gdtf_test_utils::gpu_probe::gpu_adapter_probe;

use crate::{
    enqueue::{enqueue_capture, finished_captures},
    harness::{advance_to_editing, gpu_editor_app, headless_editor_app, spawned_captures},
    support::{DRIVE_UPDATES, TEST_SETTLE, TestError, TestResult},
};

const SHOT_NAME: &str = "editor_shell";

fn drive_until_finished(app: &mut App) -> Result<CaptureOutcome, TestError> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Some(outcome) = finished_captures(app).first() {
            return Ok(outcome.clone());
        }
    }
    Err("the editor never finished the queued capture".into())
}

#[test]
fn a_claimed_capture_lands_a_png_on_disk() -> TestResult {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP a_claimed_capture_lands_a_png_on_disk: no usable wgpu adapter \
             (GPU-less runner). The ordering half of the contract is covered by \
             the_capture_that_never_lands_times_out, which needs no GPU.",
        );
        return Ok(());
    }
    let tmp = tempfile::TempDir::new()?;
    let (mut app, _port) = gpu_editor_app(tmp.path().to_path_buf())?;
    advance_to_editing(&mut app);

    enqueue_capture(&mut app, SHOT_NAME);
    let outcome = drive_until_finished(&mut app)?;

    let CaptureOutcome::Landed(saved) = outcome else {
        return Err(format!("the editor's capture must land a PNG, not {outcome:?}").into());
    };
    let png = PathBuf::from(&*saved);
    assert!(
        png.starts_with(tmp.path()),
        "the capture must land inside the confinement directory, not at {}",
        png.display(),
    );
    assert!(
        png.file_name()
            .is_some_and(|stem| { stem.to_string_lossy().starts_with(SHOT_NAME) }),
        "the requested name must reach the file name, which was {}",
        png.display(),
    );
    assert!(
        png.exists(),
        "the editor reported {} landed but no file is there",
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
fn the_capture_that_never_lands_times_out() -> TestResult {
    let tmp = tempfile::TempDir::new()?;
    let (mut app, _port) = headless_editor_app(tmp.path().to_path_buf())?;
    advance_to_editing(&mut app);

    enqueue_capture(&mut app, SHOT_NAME);

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
        "the capture must wait out the {TEST_SETTLE}-frame settle window before it is spawned; \
         it appeared on frame {spawned_at}",
    );

    let finished = finished_captures(&app);
    assert!(
        finished.is_empty(),
        "the editor reported {finished:?} at the moment its capture was spawned — before any \
         PNG could have landed",
    );
    let written = std::fs::read_dir(tmp.path())?.count();
    assert_eq!(
        written, 0,
        "no file may exist in the confinement directory at the moment the capture is spawned",
    );

    let outcome = drive_until_finished(&mut app)?;
    assert!(
        matches!(outcome, CaptureOutcome::TimedOut(_)),
        "a capture whose PNG never lands must time out, never land; got {outcome:?}",
    );
    Ok(())
}
