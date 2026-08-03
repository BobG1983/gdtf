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

pub(super) const TEST_SETTLE: u32 = 2;

pub(super) const TEST_POLL_BUDGET: u32 = 3;

pub(super) const DRIVE_UPDATES: u32 = 64;

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

pub(super) fn enqueue(app: &mut App, name: ShotName) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<EditorScreenshotPayload>>()
        .push_new(EditorScreenshotPayload::new(Some(name)), responder);
    reply_rx
}

pub(super) fn first_capture_path(dir: &EditorQaShotDir, name: &ShotName) -> CapturePath {
    let mut sequence = EditorShotSequence::default();
    next_capture_path(dir, Some(name), &mut sequence)
}

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

pub(super) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}

pub(super) fn drive_until_spawned(app: &mut App) -> Option<u32> {
    for frame in 1..=DRIVE_UPDATES {
        app.update();
        if spawned_captures(app) > 0 {
            return Some(frame);
        }
    }
    None
}
