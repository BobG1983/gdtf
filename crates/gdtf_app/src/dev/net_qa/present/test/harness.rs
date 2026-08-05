use bevy::{app::App, prelude::*};
use gdtf_screenshot::{CapturePresentPlugin, PresentSystems};
use gdtf_test_utils::GdtfWindowedTestAppBuilder;

use crate::dev::net_qa::present::{mark_ui_default_camera, retarget_cameras_to_offscreen};

pub(super) const HARNESS_SCALE_FACTOR: f32 = 1.75;

pub(super) fn headless_windowed_app() -> App {
    GdtfWindowedTestAppBuilder::new()
        .scale_factor(HARNESS_SCALE_FACTOR)
        .build()
}

/// Add the shared present path plus the game's own retarget systems.
pub(super) fn with_present_path(app: &mut App) {
    app.add_plugins(CapturePresentPlugin);
    app.add_systems(
        Update,
        (retarget_cameras_to_offscreen, mark_ui_default_camera)
            .chain()
            .in_set(PresentSystems),
    );
}

pub(super) fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}
