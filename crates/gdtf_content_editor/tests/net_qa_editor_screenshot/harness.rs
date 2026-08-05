use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::{RenderPlugin, view::window::screenshot::Screenshot},
    window::{ExitCondition, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use bevy_egui::EguiPlugin;
use gdtf_content_editor::{EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_screenshot::{PollCap, SettleFrames, ShotDir};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::support::{EDITING_UPDATES, TEST_POLL_BUDGET, TEST_SETTLE, TestError};

const WINDOW_PX: UVec2 = UVec2::new(320, 180);

const GPU_SCALE_FACTOR: f32 = 1.75;

fn pin_tunables(app: &mut App, shot_dir: PathBuf) {
    app.insert_resource(ShotDir::new(shot_dir));
    app.insert_resource(SettleFrames::new(TEST_SETTLE));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
}

pub(crate) fn headless_editor_app(shot_dir: PathBuf) -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    pin_tunables(&mut app, shot_dir);
    Ok((app, port))
}

pub(crate) fn gpu_editor_app(shot_dir: PathBuf) -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut window = Window {
        resolution: WindowResolution::new(WINDOW_PX.x, WINDOW_PX.y),
        ..default()
    };
    window.resolution.set_scale_factor(GPU_SCALE_FACTOR);
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
            .set(bevy::asset::AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>(),
    );
    app.set_error_handler(bevy::ecs::error::warn);
    app.add_plugins(EguiPlugin::default());
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    pin_tunables(&mut app, shot_dir);
    app.finish();
    app.cleanup();
    Ok((app, port))
}

fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

pub(crate) fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|state| *state.get() == EditorState::Editing)
        },
        EDITING_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         registries (a genuine load failure, not a frame-budget shortfall)",
    );
    for _ in 0..8 {
        app.update();
    }
}

pub(crate) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}
