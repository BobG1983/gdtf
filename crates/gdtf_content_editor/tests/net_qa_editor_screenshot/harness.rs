//! The two REAL editor apps this suite drives, plus the shared tunable injection (GTW-880).
//!
//! Both are the REAL editor: [`MapEditorPlugin`] on top of `DefaultPlugins`, with a live
//! [`AssetServer`](bevy::asset::AssetServer) rooted at the workspace `assets/`, reaching
//! [`EditorState::Editing`] through the editor's own `Load` pass and `OnEnter(Editing)`
//! lifecycle. Nothing about the editor's model is hand-inserted, and neither app uses
//! `MinimalPlugins`. On top of each sits the editor's own `NetQaEditorPlugin`, pre-bound
//! through [`NetQaEditorPlugin::listening`] on port `0` — the same constructor
//! `tests/net_qa_editor_query/harness.rs` uses, and for the same reason (the env gate cannot
//! be driven from a test, and an OS-assigned port keeps parallel test binaries apart).
//!
//! They differ in ONE thing, the renderer:
//!
//! - [`headless_editor_app`] is the `no_renderer` configuration
//!   ([`GdtfUiTestAppBuilder`], `WgpuSettings { backends: None }`), so a spawned
//!   [`Screenshot`] never resolves into a written PNG. That is what makes it the right app
//!   for the ordering proof: a capture that can never land must never be answered `Saved`.
//! - [`gpu_editor_app`] builds the same editor on a REAL wgpu device with a camera rendering
//!   into an offscreen image, so a capture genuinely lands a PNG on disk. It is GPU-guarded
//!   (GTW-527): on a runner with no adapter the caller skips before the app is built.

use std::path::PathBuf;

use bevy::{
    camera::RenderTarget,
    image::Image,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{TextureFormat, TextureUsages},
        view::window::screenshot::Screenshot,
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_content_editor::{
    EditorQaShotDir, EditorShotPollBudget, EditorShotSettle, EditorShotSource, EditorState,
    MapEditorPlugin, NetQaEditorPlugin,
};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_screenshot::{PollCap, SettleFrames};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::support::{EDITING_UPDATES, TEST_POLL_BUDGET, TEST_SETTLE, TestError};

/// The offscreen render-target edge, in pixels. Small keeps the readback + PNG encode cheap.
const TARGET_PX: u32 = 64;

/// Pin the capture pump's tunables for a test: a temp output directory (so no run writes into
/// the source tree), a short settle window, and a small poll budget.
fn pin_tunables(app: &mut App, shot_dir: PathBuf) {
    app.insert_resource(EditorQaShotDir::new(shot_dir));
    app.insert_resource(EditorShotSettle::new(SettleFrames::new(TEST_SETTLE)));
    app.insert_resource(EditorShotPollBudget::new(PollCap::new(TEST_POLL_BUDGET)));
}

/// The REAL editor on the no-renderer harness, with its QA listener already bound.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
pub(crate) fn headless_editor_app(shot_dir: PathBuf) -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    pin_tunables(&mut app, shot_dir);
    Ok((app, port))
}

/// The REAL editor on a REAL wgpu device, rendering into an offscreen image the capture pump
/// is pointed at, with its QA listener already bound.
///
/// The offscreen source is what makes a capture possible at all here: this app has no window
/// (winit's event loop cannot be created off the main thread on macOS), so there is no
/// swapchain to read back. A camera renders into an [`Image`] every tick and
/// [`EditorShotSource::Offscreen`] tells the pump to capture that image — the same
/// `Screenshot::image` path `gdtf_screenshot`'s own real-GPU test uses.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
pub(crate) fn gpu_editor_app(shot_dir: PathBuf) -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
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
    // The same skip-a-render-provided-param handling `GdtfUiTestAppBuilder` uses: a
    // windowless app has systems whose params cannot validate, and a hard error would kill
    // the run for reasons unrelated to what is under test.
    app.set_error_handler(bevy::ecs::error::warn);
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    // COPY_SRC so the screenshot readback can copy the rendered image out.
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.1, 0.4, 0.8)),
            ..default()
        },
        RenderTarget::Image(handle.clone().into()),
    ));
    pin_tunables(&mut app, shot_dir);
    app.insert_resource(EditorShotSource::Offscreen(handle));
    app.finish();
    app.cleanup();
    Ok((app, port))
}

/// The workspace-root `assets/` directory, so the GPU app's `AssetServer` reads the same
/// loose assets the running editor does (the harness crate computes the same path for its
/// own builder; this crate's manifest is one level deeper in the same layout).
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Drive the app until the editor's own `Load` pass releases into [`EditorState::Editing`],
/// then a few frames so the `OnEnter(Editing)` inserts apply.
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
    for _ in 0..4 {
        app.update();
    }
}

/// How many [`Screenshot`] entities the app holds — non-zero exactly when the pump has
/// spawned a capture, which is the observable moment the settle window ended.
pub(crate) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}
