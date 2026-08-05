use bevy::{
    app::App,
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};
use gdtf_screenshot::{CapturePresentPlugin, PresentSystems, QaCaptureTarget};
use gdtf_test_utils::GdtfWindowedTestAppBuilder;

use crate::net_qa::present::retarget_editor_camera_to_offscreen;

pub(super) const HARNESS_SCALE_FACTOR: f32 = 1.5;

pub(super) fn headless_windowed_app() -> App {
    let mut app = GdtfWindowedTestAppBuilder::new()
        .scale_factor(HARNESS_SCALE_FACTOR)
        .build();
    app.init_resource::<WindowToEguiContextMap>();
    app
}

/// Add the shared present path plus the editor's own retarget, the way the QA plugin does.
pub(super) fn with_present_path(app: &mut App) {
    app.add_plugins(CapturePresentPlugin);
    app.add_systems(
        Update,
        retarget_editor_camera_to_offscreen.in_set(PresentSystems),
    );
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
        .get_resource::<QaCaptureTarget>()
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
