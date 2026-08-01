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
    prelude::*,
    render::{RenderPlugin, view::window::screenshot::Screenshot},
    window::{ExitCondition, WindowPlugin, WindowResolution},
    winit::WinitPlugin,
};
use bevy_egui::EguiPlugin;
use gdtf_content_editor::{
    EditorQaShotDir, EditorShotPollBudget, EditorShotSettle, EditorState, MapEditorPlugin,
    NetQaEditorPlugin,
};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_screenshot::{PollCap, SettleFrames};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};

use crate::support::{EDITING_UPDATES, TEST_POLL_BUDGET, TEST_SETTLE, TestError};

/// The GPU app window's PHYSICAL size, in pixels. Small keeps the readback + PNG encode cheap —
/// the present path sizes the offscreen capture target to exactly this.
const WINDOW_PX: UVec2 = UVec2::new(320, 180);

/// The scale factor the GPU app's window reports.
///
/// Deliberately NOT `1.0`: `ImageRenderTarget`'s `From<Handle<Image>>` — and therefore
/// `Screenshot::image` — hardcodes `1.0`, so a capture built that way addresses a DIFFERENT render
/// target from the editor's camera and copies out a texture nothing drew into. That is the GTW-922
/// black-frame defect, and this is the value that makes it visible in a real GPU readback. It is
/// also distinct from every other scale-factor fixture in the crate (`1.5`, `1.25`, `2.0`, `2.5`,
/// `3.0`), so no single hardcoded constant satisfies this test and any of the others.
const GPU_SCALE_FACTOR: f32 = 1.75;

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

/// The REAL editor on a REAL wgpu device, with a real primary window and the editor's OWN QA
/// present path doing the retargeting, with its QA listener already bound.
///
/// Nothing here inserts a capture source, creates a target, or spawns a camera. The editor's own
/// `NetQaEditorPlugin` builds the present path, which sizes the offscreen target to the window,
/// retargets the editor's egui camera into it once `bevy_egui` has recorded that camera's input
/// mapping, and points the pump at it — so the PNG this app lands holds whatever production code
/// actually drew. That is what makes the pixel assertion in
/// `a_claimed_capture_lands_a_png_on_disk` meaningful: a hand-rigged camera and a
/// hand-inserted source would have proved only that the test wired itself up correctly.
///
/// Two things still differ from the shipped editor, and neither touches the capture: no winit
/// event loop (a cargo test thread cannot create one on macOS), so the window entity exists with
/// no OS surface behind it; and the window is small, to keep the readback and PNG encode cheap.
/// Rendering into an `Image` needs no surface, which is the whole reason the offscreen path
/// exists.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
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
    // The same skip-a-render-provided-param handling `GdtfUiTestAppBuilder` uses: a window with no
    // winit surface behind it has systems whose params cannot validate, and a hard error would
    // kill the run for reasons unrelated to what is under test.
    app.set_error_handler(bevy::ecs::error::warn);
    app.add_plugins(EguiPlugin::default());
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    pin_tunables(&mut app, shot_dir);
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
/// then a few frames so the `OnEnter(Editing)` inserts apply, the editor camera spawns,
/// `bevy_egui` records its input mapping in the NEXT frame's `PreUpdate`, and the present path's
/// chained `Update` systems retarget it.
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

/// How many [`Screenshot`] entities the app holds — non-zero exactly when the pump has
/// spawned a capture, which is the observable moment the settle window ended.
pub(crate) fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}
