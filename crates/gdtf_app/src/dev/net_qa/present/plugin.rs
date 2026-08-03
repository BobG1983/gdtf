use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

use super::{
    blit::spawn_present_pass,
    retarget::{mark_ui_default_camera, retarget_cameras_to_offscreen},
    target::{QaCaptureTarget, ensure_capture_target},
};

pub(in crate::dev::net_qa) struct CapturePresentPlugin;

impl Plugin for CapturePresentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings {
            focused_mode:   UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
        app.add_systems(
            Update,
            (
                ensure_capture_target.run_if(not(resource_exists::<QaCaptureTarget>)),
                retarget_cameras_to_offscreen,
                mark_ui_default_camera,
                spawn_present_pass,
            )
                .chain(),
        );
    }
}
