use bevy::prelude::*;
use bevy_egui::PrimaryEguiContext;

use crate::states::running::UiCamera;

pub(super) fn bind_primary_egui_context(
    ui_cameras: Query<Entity, (With<UiCamera>, Without<PrimaryEguiContext>)>,
    mut commands: Commands,
) {
    for entity in &ui_cameras {
        commands.entity(entity).insert(PrimaryEguiContext);
        info!("dev-affordances: primary egui context bound to the captured UI camera");
    }
}
