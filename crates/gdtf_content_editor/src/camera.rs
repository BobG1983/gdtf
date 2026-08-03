use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, PrimaryEguiContext};

pub(crate) fn disable_egui_auto_context(mut settings: ResMut<EguiGlobalSettings>) {
    settings.auto_create_primary_context = false;
}

pub(crate) fn spawn_editor_camera(mut commands: Commands) {
    commands.spawn((Camera2d, PrimaryEguiContext));
}
