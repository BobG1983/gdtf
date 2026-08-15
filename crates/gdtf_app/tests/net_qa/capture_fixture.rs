use std::{
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
    render::RenderPlugin,
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use gdtf_app::test_support::{self, AppState, NetQaPlugin, RunningState};
use gdtf_qa_protocol::ports::NetQaPort;
use gdtf_screenshot::{CaptureImage, CaptureSystems, PollCap, SettleFrames, ShotDir, ShotDirName};
use gdtf_test_utils::advance_until;

use super::socket_support::{TestError, capture_app_listening};

/// Stem the windowed capture asks for.
pub(crate) const GPU_SHOT_NAME: &str = "socket_shot";

/// Stem the windowless capture asks for.
pub(crate) const HEADLESS_SHOT_NAME: &str = "headless_shot";

const GPU_WINDOW_PX: UVec2 = UVec2::new(320, 180);

/// Colour the fixture's own camera draws into the capture image.
const FIXTURE_FRAME: Color = Color::srgb(0.9, 0.2, 0.4);

const GPU_SETTLE: u32 = 4;

const GPU_POLL_BUDGET: u32 = 240;

const HEADLESS_SETTLE: u32 = 2;

const HEADLESS_POLL_BUDGET: u32 = 16;

/// Fixed steps a frame runs, so Intro's steps advance per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

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

// One directory per fixture call, counted rather than named, so no two calls pick the same one.
fn next_case_dir_name() -> String {
    static CASES: AtomicU32 = AtomicU32::new(0);
    format!("case_{}", CASES.fetch_add(1, Ordering::Relaxed))
}

fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn stand_in_shot_name() -> String {
    format!("{HEADLESS_SHOT_NAME}_0.png")
}

// Nothing renders here, so this stands in for the capture's own PNG writer.
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

// Without winit there is no window handle, so bevy runs no window camera and draws nothing.
fn draw_into_the_capture_image(app: &mut App) {
    let image = app.world().resource::<CaptureImage>().clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(FIXTURE_FRAME),
            ..default()
        },
        RenderTarget::Image(ImageRenderTarget {
            handle:       (*image).clone(),
            scale_factor: 1.0,
        }),
    ));
}

/// The headless menu app, listening, with a PNG arriving at the path the pump picked.
///
/// Each call writes into its own directory under [`headless_shot_dir`], so two cases running
/// side by side never share a file or delete each other's.
pub(crate) fn landing_capture_app_listening() -> Result<(App, NetQaPort), TestError> {
    let dir = ShotDir::new(headless_shot_dir().join(next_case_dir_name()));
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
    app.insert_resource(TimeUpdateStrategy::FixedTimesteps(ONE_STEP_A_FRAME));
    advance_until(&mut app, |app| {
        app.world()
            .get_resource::<State<RunningState>>()
            .is_some_and(|state| *state.get() == RunningState::Menu)
    });
    draw_into_the_capture_image(&mut app);
    Ok((app, port))
}
