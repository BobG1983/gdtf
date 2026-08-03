use bevy::{camera::RenderTarget, prelude::*};
use bevy_egui::{PrimaryEguiContext, input::WindowToEguiContextMap};

use super::target::{EditorQaCaptureTarget, aims_at};

pub(in crate::net_qa) fn retarget_editor_camera_to_offscreen(
    target: Option<Res<EditorQaCaptureTarget>>,
    map: Option<Res<WindowToEguiContextMap>>,
    cameras: Query<(Entity, &RenderTarget), With<PrimaryEguiContext>>,
    mut commands: Commands,
) {
    let (Some(target), Some(map)) = (target, map) else {
        return;
    };
    for (entity, current) in &cameras {
        if aims_at(current, &target) {
            continue;
        }
        if !map.context_to_window.contains_key(&entity) {
            continue;
        }
        commands
            .entity(entity)
            .insert(RenderTarget::Image((**target).clone()));
    }
}
