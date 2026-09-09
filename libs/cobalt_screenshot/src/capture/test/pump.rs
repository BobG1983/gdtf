use std::path::Path;

use bevy::prelude::*;

use super::support::{
    TEST_SETTLE, drive_until_finished, drive_until_spawned, enqueue, outcomes, pump_app,
};
use crate::{
    capture::{CaptureOutcome, ShotDir, ShotStem},
    path::CapturePath,
};

fn first_capture_path(dir: &Path, stem: &str) -> CapturePath {
    CapturePath::new(dir.join(format!("{stem}_0.png")))
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

#[test]
fn the_pump_waits_out_the_whole_settle_window_before_spawning() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    enqueue(&mut app, "settled");

    let spawned_at = drive_until_spawned(&mut app);
    assert_eq!(
        spawned_at,
        TEST_SETTLE + 2,
        "the capture must wait out the {TEST_SETTLE}-frame settle window after its claim frame; \
         it was spawned on frame {spawned_at}",
    );
    assert!(
        outcomes(&app).is_empty(),
        "no capture may be reported finished on the frame it spawns: {:?}",
        outcomes(&app),
    );
}

#[test]
fn the_pump_creates_a_missing_shot_directory_before_it_spawns() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let dir = tmp.path().join("not").join("made").join("yet");
    assert!(
        !dir.exists(),
        "test setup: the shot directory must be missing to start with",
    );

    let mut app = pump_app(dir.clone());
    enqueue(&mut app, "fresh");

    drive_until_spawned(&mut app);
    assert!(
        dir.is_dir(),
        "the pump must create {} before it spawns — the capture's PNG writer cannot write into a \
         directory that is not there",
        dir.display(),
    );
}

#[test]
fn a_stale_png_already_at_the_path_is_purged_and_never_landed() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let path = first_capture_path(tmp.path(), "stale");
    plant_decodable_png(&path);

    let mut app = pump_app(tmp.path().to_path_buf());
    enqueue(&mut app, "stale");

    drive_until_spawned(&mut app);
    assert!(
        !path.exists(),
        "the stale file at {} must be deleted before the capture spawns",
        path.display(),
    );

    let finished = drive_until_finished(&mut app);
    assert!(
        matches!(finished, CaptureOutcome::TimedOut(_)),
        "a stale PNG at the target path must be purged, never served — the capture times out, \
         got {finished:?}",
    );
}

#[test]
fn bytes_that_do_not_decode_as_a_png_are_never_landed() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let path = first_capture_path(tmp.path(), "garbled");

    let mut app = pump_app(tmp.path().to_path_buf());
    enqueue(&mut app, "garbled");

    drive_until_spawned(&mut app);
    plant_garbage(&path);

    let finished = drive_until_finished(&mut app);
    assert!(
        matches!(finished, CaptureOutcome::TimedOut(_)),
        "bytes that do not decode as a PNG must never be reported Landed, got {finished:?}",
    );
}

#[test]
fn a_png_landing_after_the_spawn_is_landed_at_that_exact_path() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let path = first_capture_path(tmp.path(), "landed");

    let mut app = pump_app(tmp.path().to_path_buf());
    enqueue(&mut app, "landed");

    drive_until_spawned(&mut app);
    assert!(
        outcomes(&app).is_empty(),
        "no reply may exist before a PNG has landed: {:?}",
        outcomes(&app),
    );
    plant_decodable_png(&path);

    let finished = drive_until_finished(&mut app);
    assert_eq!(
        finished,
        CaptureOutcome::Landed(path.clone()),
        "a PNG landing after the capture spawned must be reported Landed at {}",
        path.display(),
    );
}

#[test]
fn a_stem_lands_under_the_shot_directory() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = pump_app(tmp.path().to_path_buf());
    app.world_mut()
        .resource_mut::<crate::capture::CaptureQueue<()>>()
        .push(Some(ShotStem::new("named")), ());
    let dir = app.world().resource::<ShotDir>().clone();

    drive_until_spawned(&mut app);
    let planted = CapturePath::new(dir.join("named_0.png"));
    plant_decodable_png(&planted);

    let finished = drive_until_finished(&mut app);
    assert_eq!(
        finished,
        CaptureOutcome::Landed(planted),
        "a stem-named capture must land under the shot directory",
    );
}
