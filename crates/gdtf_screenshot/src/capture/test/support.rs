use std::path::PathBuf;

use bevy::{
    asset::Assets,
    camera::{ImageRenderTarget, RenderTarget},
    image::Image,
    prelude::*,
    render::view::window::screenshot::Screenshot,
};

use crate::{
    capture::{
        CaptureCompletions, CaptureOutcome, CapturePipelinePlugin, CaptureQueue, CaptureSource,
        ShotDir, ShotStem,
    },
    present::QaCaptureTarget,
    settle::{PollCap, SettleFrames},
};

pub(super) const TEST_SETTLE: u32 = 2;

pub(super) const TEST_POLL_BUDGET: u32 = 3;

pub(super) const DRIVE_UPDATES: u32 = 64;

/// A `MinimalPlugins` app running the real pump with both tunables pinned.
pub(super) fn pump_app(dir: PathBuf) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(CapturePipelinePlugin::<()>::new());
    app.insert_resource(ShotDir::new(dir));
    app.insert_resource(SettleFrames::new(TEST_SETTLE));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
    app
}

pub(super) fn install_capture_target(app: &mut App, scale_factor: f32) -> ImageRenderTarget {
    app.init_resource::<Assets<Image>>();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let target = ImageRenderTarget {
        handle,
        scale_factor,
    };
    app.insert_resource(QaCaptureTarget::new(target.clone()));
    app.insert_resource(CaptureSource::Offscreen(target.clone()));
    target
}

pub(super) fn spawn_camera_aimed_at(app: &mut App, target: RenderTarget) -> Entity {
    app.world_mut().spawn((Camera::default(), target)).id()
}

pub(super) fn enqueue(app: &mut App, stem: &str) {
    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new(stem)), ());
}

pub(super) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}

pub(super) fn outcomes(app: &App) -> Vec<CaptureOutcome> {
    app.world()
        .resource::<CaptureCompletions<()>>()
        .iter()
        .map(|completion| completion.outcome().clone())
        .collect()
}

/// Drive until the pump either finishes a capture or spawns one.
pub(super) fn drive_until_settled(app: &mut App) {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if !outcomes(app).is_empty() || spawned_captures(app) > 0 {
            return;
        }
    }
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

pub(super) fn drive_until_finished(app: &mut App) -> Option<CaptureOutcome> {
    for _ in 0..DRIVE_UPDATES {
        app.update();
        if let Some(outcome) = outcomes(app).first() {
            return Some(outcome.clone());
        }
    }
    None
}
