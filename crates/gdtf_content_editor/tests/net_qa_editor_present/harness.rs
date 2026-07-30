//! The REAL editor app this suite drives: the binary's own plugin stack on a headless
//! `DefaultPlugins` configuration that still has a real primary window (GTW-918).
//!
//! Everything the editor binary composes is here — `DefaultPlugins` with the workspace `assets/`
//! root, [`EguiPlugin`], the editor's own [`MapEditorPlugin`], and [`NetQaEditorPlugin`] on its
//! listener arm — so the present path is observed on the app the binary builds, not on a
//! hand-assembled one. Two things differ from the binary, and neither touches the present path:
//! no GPU adapter (`backends: None`) and no winit event loop (a cargo test thread cannot create
//! one on macOS), which is why the assertions read ECS state rather than pixels. Pixels belong to
//! GTW-904.
//!
//! The listener is pre-bound through [`NetQaEditorPlugin::listening`] on port `0` — the same
//! constructor the other editor QA suites use, and for the same reason: the env gate cannot be
//! driven from a test (`std::env::set_var` is unsafe in edition 2024 and the workspace forbids
//! `unsafe`), and an OS-assigned port keeps parallel test binaries apart.

use bevy::{
    DefaultPlugins,
    app::PluginGroup,
    asset::AssetPlugin,
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use bevy_egui::EguiPlugin;
use gdtf_assets::WORKSPACE_ASSETS_ROOT;
use gdtf_content_editor::{EditorState, MapEditorPlugin, NetQaEditorPlugin};
use gdtf_net_qa_transport::NetQaPort;
use gdtf_test_utils::advance_until;

use crate::support::TestError;

/// The scale factor the harness window reports. Deliberately NOT `1.0`, so a retarget that took
/// `ImageRenderTarget`'s `From<Handle<Image>>` shortcut (which hardcodes `1.0`) is visible on the
/// real app too.
pub(crate) const HARNESS_SCALE_FACTOR: f32 = 2.0;

/// A generous frame cap for the editor's async `Load` pass under parallel `cargo` contention. A
/// SAFETY NET, not a timing budget — the loop polls the `EditorState::Editing` signal.
const MAX_UPDATES: u32 = 10_000;

/// Build the REAL editor app with a real primary window, its QA listener already bound.
///
/// # Errors
///
/// Any [`std::io::Error`] from binding the loopback listener.
pub(crate) fn windowed_editor_app() -> Result<(App, NetQaPort), TestError> {
    let (plugin, port) = NetQaEditorPlugin::listening(NetQaPort::new(0))?;
    let mut window = Window::default();
    window.resolution.set_scale_factor(HARNESS_SCALE_FACTOR);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: Some(window),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: WORKSPACE_ASSETS_ROOT.to_owned(),
                ..default()
            }),
    );
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler (which
    // panics by default); with no render backend some render-provided params cannot validate.
    // `warn` restores skip-with-a-log, the shared headless-harness precedent.
    app.set_error_handler(warn);
    app.add_plugins(EguiPlugin::default());
    app.add_plugins(MapEditorPlugin);
    app.add_plugins(plugin);
    Ok((app, port))
}

/// Drive the app until it reaches [`EditorState::Editing`] (its `Load` pass resolved the theme +
/// registries), then a few more frames so `OnEnter(Editing)` spawns the editor camera, `bevy_egui`
/// records that camera's input mapping in the NEXT frame's `PreUpdate`, and the present path's
/// chained `Update` systems act on it.
pub(crate) fn advance_to_editing(app: &mut App) {
    let reached = advance_until(
        app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|state| *state.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the theme \
         + registries (a genuine load failure, not a frame-budget shortfall)",
    );
    for _ in 0..8 {
        app.update();
    }
}
