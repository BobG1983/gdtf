use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

use super::{
    blit::spawn_editor_present_pass,
    retarget::retarget_editor_camera_to_offscreen,
    target::{EditorQaCaptureTarget, ensure_editor_capture_target},
};
use crate::net_qa::schedule::EditorNetQaSystems;

pub(in crate::net_qa) struct EditorCapturePresentPlugin;

impl Plugin for EditorCapturePresentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings {
            focused_mode:   UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
        app.configure_sets(
            Update,
            EditorNetQaSystems::Present.before(EditorNetQaSystems::Gather),
        );
        app.add_systems(
            Update,
            (
                ensure_editor_capture_target.run_if(not(resource_exists::<EditorQaCaptureTarget>)),
                retarget_editor_camera_to_offscreen,
                spawn_editor_present_pass,
            )
                .chain()
                .in_set(EditorNetQaSystems::Present),
        );
    }
}
