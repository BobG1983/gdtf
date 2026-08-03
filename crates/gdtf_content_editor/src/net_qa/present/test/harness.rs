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

pub(super) const HARNESS_SCALE_FACTOR: f32 = 1.5;

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
    app.set_error_handler(warn);
    app.init_resource::<WindowToEguiContextMap>();
    app
}

pub(super) fn spawn_editor_like_camera(app: &mut App) -> Entity {
    app.world_mut().spawn((Camera2d, PrimaryEguiContext)).id()
}

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

pub(super) fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

pub(super) fn capture_target(app: &App) -> Option<ImageRenderTarget> {
    app.world()
        .get_resource::<EditorQaCaptureTarget>()
        .map(|target| (**target).clone())
}

pub(super) fn egui_camera_target(app: &mut App) -> Option<RenderTarget> {
    let world = app.world_mut();
    let mut cameras = world.query_filtered::<&RenderTarget, With<PrimaryEguiContext>>();
    let targets: Vec<RenderTarget> = cameras.iter(world).cloned().collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}
