//! Shared harness for the GTW-764 present-path tests: a headless `DefaultPlugins` app with a
//! real PRIMARY WINDOW but no GPU backend and no winit event loop.
//!
//! `DefaultPlugins` gives a live `Assets<Image>` (so the offscreen target can be created) and
//! a spawned `PrimaryWindow` entity (so it can be sized); `backends: None` means no GPU (the
//! non-black-pixel proof is in-engine QA, not here) and `WinitPlugin` is disabled so the app
//! drives off the test thread (the `gdtf_screenshot` / editor-mode-test recipe).

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::error::warn,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};

/// Build a headless `DefaultPlugins` app with a real primary window, no GPU backend, and no
/// winit event loop — the base every windowed present-path test adds its plugins onto.
pub(super) fn headless_windowed_app() -> App {
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
                primary_window: Some(Window::default()),
                exit_condition: ExitCondition::DontExit,
                ..default()
            }),
    );
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler (default
    // panics); with no render backend some render-provided params cannot validate. `warn`
    // restores skip-with-a-log (the shared DefaultPlugins-headless harness precedent).
    app.set_error_handler(warn);
    app
}
