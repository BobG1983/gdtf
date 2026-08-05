//! Plugin wiring the offscreen target and the blit pass.

use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

use super::{
    blit::spawn_present_pass,
    target::{QaCaptureTarget, ensure_capture_target},
};
use crate::capture::CaptureSystems;

/// System set holding the present path, ordered before the capture pump.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PresentSystems;

/// Renders the app into an offscreen image and blits it back to the window.
pub struct CapturePresentPlugin;

impl Plugin for CapturePresentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings {
            focused_mode:   UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
        app.configure_sets(Update, PresentSystems.before(CaptureSystems));
        app.add_systems(
            Update,
            (
                ensure_capture_target.run_if(not(resource_exists::<QaCaptureTarget>)),
                spawn_present_pass,
            )
                .chain()
                .in_set(PresentSystems),
        );
    }
}
