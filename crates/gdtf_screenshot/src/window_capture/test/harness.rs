use std::path::Path;

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, gpu_readback::Readback, pipelined_rendering::PipelinedRenderingPlugin},
    window::{ExitCondition, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use gdtf_test_utils::GdtfWindowedTestAppBuilder;

use crate::{
    capture::{CapturePipelinePlugin, CaptureQueue, ShotDir, ShotStem},
    settle::{PollCap, SettleFrames},
    window_capture::WindowCapturePlugin,
};

pub(super) const FRAME_BUDGET: u32 = 64;

const GPU_WINDOW_PX: UVec2 = UVec2::new(320, 180);

const TEST_SETTLE: u32 = 2;

const TEST_POLL_BUDGET: u32 = 4;

fn install_capture(app: &mut App, dir: &Path) {
    app.add_plugins(WindowCapturePlugin);
    app.add_plugins(CapturePipelinePlugin::<()>::new());
    app.insert_resource(ShotDir::new(dir.to_path_buf()));
    app.insert_resource(SettleFrames::new(TEST_SETTLE));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
}

/// A windowed app with no wgpu backend, wired for captures writing under `dir`.
pub(super) fn headless_capture_app(dir: &Path) -> App {
    let mut app = GdtfWindowedTestAppBuilder::new().build();
    install_capture(&mut app, dir);
    app
}

/// A windowed app on the real backend, wired for captures writing under `dir`.
pub(super) fn gpu_capture_app(dir: &Path) -> App {
    let mut window = Window {
        resolution: WindowResolution::new(GPU_WINDOW_PX.x, GPU_WINDOW_PX.y),
        ..default()
    };
    window.resolution.set_scale_factor(1.0);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(window),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>(),
    );
    app.set_error_handler(warn);
    install_capture(&mut app, dir);
    app.finish();
    app.cleanup();
    app
}

pub(super) fn enqueue_capture(app: &mut App, stem: &str) {
    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new(stem)), ());
}

pub(super) fn a_capture_is_in_flight(app: &mut App) -> bool {
    let world = app.world_mut();
    let mut readbacks = world.query::<&Readback>();
    readbacks.iter(world).next().is_some()
}

pub(super) fn the_queue_is_idle(app: &mut App) -> bool {
    app.world().resource::<CaptureQueue<()>>().is_idle()
}

pub(super) fn drive_until<F: FnMut(&mut App) -> bool>(app: &mut App, mut done: F) -> bool {
    for _ in 0..FRAME_BUDGET {
        app.update();
        if done(app) {
            return true;
        }
    }
    false
}
