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
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};

/// The scale factor the harness window reports — deliberately NOT `1.0`, so a retarget that
/// took `ImageRenderTarget`'s `From<Handle<Image>>` shortcut (which hardcodes `1.0`) is visible.
pub(super) const HARNESS_SCALE_FACTOR: f32 = 2.0;

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
