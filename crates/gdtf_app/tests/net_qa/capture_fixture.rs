use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    prelude::*,
    render::RenderPlugin,
    window::{ExitCondition, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use gdtf_app::test_support::{self, AppState, NetQaPlugin, RunningState};
use gdtf_qa_protocol::ports::NetQaPort;
use gdtf_screenshot::{CaptureSystems, PollCap, SettleFrames, ShotDir, ShotDirName};
use gdtf_test_utils::advance_until;

use super::socket_support::{TestError, capture_app_listening};

/// Stem the windowed capture asks for.
pub(crate) const GPU_SHOT_NAME: &str = "socket_shot";

/// Stem the windowless capture asks for.
pub(crate) const HEADLESS_SHOT_NAME: &str = "headless_shot";

const GPU_WINDOW_PX: UVec2 = UVec2::new(320, 180);

const GPU_SETTLE: u32 = 4;

const GPU_POLL_BUDGET: u32 = 240;

const HEADLESS_SETTLE: u32 = 2;

const HEADLESS_POLL_BUDGET: u32 = 16;

const DRIVE_BUDGET: u32 = 512;

/// Where the windowed capture writes.
pub(crate) fn gpu_shot_dir() -> ShotDir {
    ShotDir::under_workspace_target(&ShotDirName::new(format!(
        "qa_screenshot_gpu_{}",
        std::process::id()
    )))
}

/// Where the windowless capture writes.
pub(crate) fn headless_shot_dir() -> ShotDir {
    ShotDir::under_workspace_target(&ShotDirName::new(format!(
        "qa_screenshot_headless_{}",
        std::process::id()
    )))
}

fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

fn stand_in_shot_name() -> String {
    format!("{HEADLESS_SHOT_NAME}_0.png")
}

// A windowless, adapter-less app has no swapchain to read, so Bevy's save_to_disk never fires
// and this is the only thing that can put a PNG under the shot directory.
fn stand_in_for_the_renderer(dir: Res<ShotDir>) {
    if !dir.is_dir() {
        return;
    }
    let path = dir.join(stand_in_shot_name());
    if path.exists() {
        return;
    }
    let frame = image::RgbaImage::from_pixel(2, 2, image::Rgba([7, 9, 11, 255]));
    drop(frame.save_with_format(&path, image::ImageFormat::Png));
}

/// The headless menu app, listening, with a PNG arriving at the path the pump picked.
pub(crate) fn landing_capture_app_listening() -> Result<(App, NetQaPort), TestError> {
    let dir = headless_shot_dir();
    drop(std::fs::remove_dir_all(dir.as_path()));
    let (mut app, port) = capture_app_listening()?;
    app.insert_resource(dir);
    app.insert_resource(SettleFrames::new(HEADLESS_SETTLE));
    app.insert_resource(PollCap::new(HEADLESS_POLL_BUDGET));
    app.add_systems(Update, stand_in_for_the_renderer.after(CaptureSystems));
    Ok((app, port))
}

/// A real-render windowed game app on the menu, listening, writing into [`gpu_shot_dir`].
pub(crate) fn gpu_game_app_listening() -> Result<(App, NetQaPort), TestError> {
    let dir = gpu_shot_dir();
    drop(std::fs::remove_dir_all(dir.as_path()));
    let (plugin, port) = NetQaPlugin::listening(NetQaPort::new(0))?;
    let mut window = Window {
        resolution: WindowResolution::new(GPU_WINDOW_PX.x, GPU_WINDOW_PX.y),
        ..default()
    };
    window.resolution.set_scale_factor(1.0);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(window),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>(),
    );
    app.set_error_handler(bevy::ecs::error::warn);
    test_support::register_scenes_with_default_plugins(&mut app);
    app.add_plugins(plugin);
    app.insert_resource(dir);
    app.insert_resource(SettleFrames::new(GPU_SETTLE));
    app.insert_resource(PollCap::new(GPU_POLL_BUDGET));
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Load);
    app.finish();
    app.cleanup();
    if !advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<RunningState>>()
                .is_some_and(|state| *state.get() == RunningState::Menu)
        },
        DRIVE_BUDGET,
    ) {
        return Err("the windowed game never rested at RunningState::Menu".into());
    }
    Ok((app, port))
}
