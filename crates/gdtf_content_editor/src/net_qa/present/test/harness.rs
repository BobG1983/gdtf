//! Shared harness for the GTW-918 editor present-path tests: a headless `DefaultPlugins` app
//! with a real PRIMARY WINDOW but no GPU backend and no winit event loop.
//!
//! `DefaultPlugins` gives a live `Assets<Image>` (so the offscreen target can be created) and a
//! spawned `PrimaryWindow` entity (so it can be sized); `backends: None` means no GPU and
//! `WinitPlugin` is disabled so the app drives off the test thread. Same recipe as the game's
//! GTW-764 present harness (`crates/gdtf_app/src/dev/net_qa/present/test/harness.rs`).

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::{ImageRenderTarget, RenderTarget},
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};

use crate::net_qa::present::target::EditorQaCaptureTarget;

/// The scale factor the harness window reports — deliberately NOT `1.0`, so a retarget that
/// took `ImageRenderTarget`'s `From<Handle<Image>>` shortcut (which hardcodes `1.0`) is visible.
///
/// It is also deliberately NOT the `2.0` the real-editor suite's window reports
/// (`tests/net_qa_editor_present/harness.rs`). The two scale-factor assertions used to share one
/// fixture value, so a hardcoded `2.0` anywhere in the retarget would have satisfied both
/// (GTW-922 clause 6). With `1.5` here and `2.0` there, only a value actually read from the
/// window can pass both.
pub(super) const HARNESS_SCALE_FACTOR: f32 = 1.5;

/// Build a headless `DefaultPlugins` app with a real primary window at
/// [`HARNESS_SCALE_FACTOR`], no GPU backend, and no winit event loop.
pub(super) fn headless_windowed_app() -> App {
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
            }),
    );
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler (default
    // panics); with no render backend some render-provided params cannot validate. `warn`
    // restores skip-with-a-log (the shared DefaultPlugins-headless harness precedent).
    app.set_error_handler(warn);
    // The input map `bevy_egui` would own. These tests do NOT add `EguiPlugin` — the retarget
    // gate is about the presence of a MAP ENTRY, and inserting the resource directly is what
    // lets a test pin BOTH the entry-present and entry-absent cases deterministically.
    app.init_resource::<WindowToEguiContextMap>();
    app
}

/// Spawn a camera shaped like the editor's own (`crate::camera::spawn_editor_camera`): a
/// `Camera2d` carrying [`PrimaryEguiContext`], window-targeted at spawn.
pub(super) fn spawn_editor_like_camera(app: &mut App) -> Entity {
    app.world_mut().spawn((Camera2d, PrimaryEguiContext)).id()
}

/// Record the `bevy_egui` input-map entry that `on_egui_context_added_system` would record for a
/// window-targeted context — the gate [`retarget_editor_camera_to_offscreen`](super::super::retarget::retarget_editor_camera_to_offscreen)
/// waits for.
pub(super) fn record_egui_window_mapping(app: &mut App, camera: Entity) {
    let mut windows = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>();
    let window = windows.iter(app.world()).next();
    let Some(window) = window else {
        return;
    };
    let mut map = app.world_mut().resource_mut::<WindowToEguiContextMap>();
    map.context_to_window.insert(camera, window);
    map.window_to_contexts
        .entry(window)
        .or_default()
        .insert(camera);
}

/// Drive enough frames for the chained present systems and their deferred commands to settle.
pub(super) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// The offscreen render target the present path created — the image AND its scale factor — or
/// [`None`] if it never created one.
pub(super) fn capture_target(app: &App) -> Option<ImageRenderTarget> {
    app.world()
        .get_resource::<EditorQaCaptureTarget>()
        .map(|target| (**target).clone())
}

/// The render target of the single camera holding the primary egui context, or [`None`] unless
/// there is exactly one.
pub(super) fn egui_camera_target(app: &mut App) -> Option<RenderTarget> {
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<&RenderTarget, With<PrimaryEguiContext>>();
    let targets: Vec<RenderTarget> = cameras.iter(world).cloned().collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}
