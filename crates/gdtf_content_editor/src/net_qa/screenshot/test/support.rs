//! The headless app the pump tests drive, and the helpers that stand in for the GPU (GTW-880).
//!
//! The ONLY thing stubbed here is the genuinely-external render boundary: an app with no
//! render device never resolves a spawned
//! [`Screenshot`](bevy::render::view::window::screenshot::Screenshot) into a written PNG, so a
//! test writes the file itself, at the exact path the REAL [`next_capture_path`] hands the
//! REAL pump. Everything else is production code: the advance-before-claim ordering, the
//! settle window, the delete-before-spawn purge, the poll loop, the decode check and the
//! reply all run for real. This is the same arrangement the game's pump tests use
//! (`crates/gdtf_app/src/dev/net_qa/screenshot/test/pump.rs`).

use std::{path::PathBuf, sync::mpsc::Receiver};

use bevy::{prelude::*, render::view::window::screenshot::Screenshot};
use gdtf_net_qa_transport::{PendingQueue, Responder};
use gdtf_qa_protocol::{ids::ShotName, message::QaResponse};
use gdtf_screenshot::{CapturePath, PollCap, SettleFrames};

use super::super::{
    config::{EditorShotPollBudget, EditorShotSettle, EditorShotSource},
    path::{EditorQaShotDir, EditorShotSequence, next_capture_path},
    payload::EditorScreenshotPayload,
    pump::{EditorInFlightShots, drive_editor_screenshots},
};

/// The settle window these tests pin, in frames. Short so a test does not spin the default
/// egui-calibrated 30, non-zero so the settle-then-capture ordering is still observable.
pub(super) const TEST_SETTLE: u32 = 2;

/// The poll budget these tests pin, in frames. Small so a capture that can never land reaches
/// its typed timeout in a handful of updates.
pub(super) const TEST_POLL_BUDGET: u32 = 3;

/// A SAFETY NET on the frame loops below. Not a timing budget: each loop exits as soon as its
/// condition holds.
pub(super) const DRIVE_UPDATES: u32 = 64;

/// Build a headless app carrying the REAL pump, confined to `dir`, with the short settle
/// window and small poll budget above.
///
/// The resources are exactly the seven the editor's own `NetQaEditorPlugin` inserts for the
/// pump; nothing about the pump itself is substituted.
pub(super) fn pump_app(dir: PathBuf) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<PendingQueue<EditorScreenshotPayload>>();
    app.init_resource::<EditorInFlightShots>();
    app.init_resource::<EditorShotSequence>();
    app.init_resource::<EditorShotSource>();
    app.insert_resource(EditorQaShotDir::new(dir));
    app.insert_resource(EditorShotSettle::new(SettleFrames::new(TEST_SETTLE)));
    app.insert_resource(EditorShotPollBudget::new(PollCap::new(TEST_POLL_BUDGET)));
    app.add_systems(Update, drive_editor_screenshots);
    app
}

/// Queue a capture on the REAL [`PendingQueue`] and hand back the reply channel.
///
/// [`PendingQueue::push_new`] is the one way a payload reaches the pump, so this is the
/// production entry point even though no request routes onto the queue today: GTW-943 removed
/// the request that did, and the editor's capture COMMAND (the editor-host ticket) pushes
/// through this same call from inside the drain. Everything under the push — the queue, the
/// pump, the responder — is the real thing.
pub(super) fn enqueue(app: &mut App, name: ShotName) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<EditorScreenshotPayload>>()
        .push_new(EditorScreenshotPayload::new(Some(name)), responder);
    reply_rx
}

/// The path the pump will write its FIRST claimed capture to — computed through the REAL
/// [`next_capture_path`] with a fresh [`EditorShotSequence`], so it matches what the pump's
/// own sequence resource (starting at its [`Default`]) produces for its first claim.
///
/// That a fresh process starts the sequence over is precisely why delete-before-spawn exists:
/// a second editor run asking for the same name targets the file the first run left behind.
pub(super) fn first_capture_path(dir: &EditorQaShotDir, name: &ShotName) -> CapturePath {
    let mut sequence = EditorShotSequence::default();
    next_capture_path(dir, Some(name), &mut sequence)
}

/// Write a real, decodable PNG at `path` — the stand-in for a flushed capture, or for the
/// leftover a previous run wrote at the same path.
pub(super) fn plant_decodable_png(path: &CapturePath) {
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

/// Write NON-PNG bytes at `path` — a truncated / garbage file that exists and is non-empty but
/// must never be reported saved.
pub(super) fn plant_garbage(path: &CapturePath) {
    if let Some(parent) = path.parent() {
        drop(std::fs::create_dir_all(parent));
    }
    let written = std::fs::write(&**path, b"this is not a png");
    assert!(
        written.is_ok(),
        "test setup: writing the garbage file must succeed: {written:?}",
    );
}

/// How many [`Screenshot`] entities the app holds — non-zero exactly when the pump has spawned
/// its capture, which is the observable moment the settle window ended and the purge ran.
pub(super) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}

/// Drive one frame at a time until the pump spawns its capture, reporting the frame it
/// appeared on (or [`None`] if it never did within [`DRIVE_UPDATES`]).
pub(super) fn drive_until_spawned(app: &mut App) -> Option<u32> {
    for frame in 1..=DRIVE_UPDATES {
        app.update();
        if spawned_captures(app) > 0 {
            return Some(frame);
        }
    }
    None
}
