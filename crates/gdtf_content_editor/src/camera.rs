//! The editor's standalone camera spawn (GTW-512 C1.2).
//!
//! The pre-egui `bevy_ui` shell spawned the `Camera2d` inside `spawn_editor_shell`. That shell is
//! gone (the egui swap), so the camera moves to its own tiny `OnEnter(Editing)` system. egui still
//! needs a camera rendering to the window (its render target), and the screenshot capture needs a
//! `Camera2d` to read back the primary window.

use bevy::prelude::*;

/// `OnEnter(Editing)`: spawn the editor's own 2D camera (the editor never reuses a game camera).
/// egui renders against this camera's window target, and the QA capture reads it back.
pub(crate) fn spawn_editor_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}
