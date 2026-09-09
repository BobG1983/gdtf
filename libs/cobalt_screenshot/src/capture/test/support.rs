use std::path::PathBuf;

use bevy::{prelude::*, render::gpu_readback::Readback};

use crate::{
    capture::{
        CaptureCompletions, CaptureOutcome, CapturePipelinePlugin, CaptureQueue, ShotDir, ShotStem,
    },
    settle::{PollCap, SettleFrames},
};

pub(super) const TEST_SETTLE: u32 = 2;

pub(super) const TEST_POLL_BUDGET: u32 = 3;

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

pub(super) fn enqueue(app: &mut App, stem: &str) {
    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new(stem)), ());
}

pub(super) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Readback>();
    query.iter(world).count()
}

pub(super) fn outcomes(app: &App) -> Vec<CaptureOutcome> {
    app.world()
        .resource::<CaptureCompletions<()>>()
        .iter()
        .map(|completion| completion.outcome().clone())
        .collect()
}

pub(super) fn drive_until_spawned(app: &mut App) -> u32 {
    let mut frame = 0;
    loop {
        app.update();
        frame += 1;
        if spawned_captures(app) > 0 {
            return frame;
        }
    }
}

pub(super) fn drive_until_finished(app: &mut App) -> CaptureOutcome {
    loop {
        app.update();
        if let Some(outcome) = outcomes(app).first() {
            return outcome.clone();
        }
    }
}
