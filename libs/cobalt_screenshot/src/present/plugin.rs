//! Plugin keeping the window on a continuous update schedule.

use bevy::{
    prelude::*,
    winit::{UpdateMode, WinitSettings},
};

/// Keeps an unfocused window drawing, so a capture never reads a stale frame.
pub struct CapturePresentPlugin;

impl Plugin for CapturePresentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WinitSettings {
            focused_mode:   UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        });
    }
}
