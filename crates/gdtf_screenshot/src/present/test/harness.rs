use bevy::{
    app::App,
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
};
use gdtf_test_utils::GdtfWindowedTestAppBuilder;

use crate::present::QaCaptureTarget;

pub(super) const HARNESS_SCALE_FACTOR: f32 = 1.5;

pub(super) fn headless_windowed_app() -> App {
    GdtfWindowedTestAppBuilder::new()
        .scale_factor(HARNESS_SCALE_FACTOR)
        .build()
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

/// Aim a fresh camera at the created capture target, the way a host's retarget does.
pub(super) fn aim_a_camera_at_the_target(app: &mut App) -> Option<Entity> {
    let target = capture_target(app)?;
    Some(
        app.world_mut()
            .spawn((Camera2d, RenderTarget::Image(target)))
            .id(),
    )
}
