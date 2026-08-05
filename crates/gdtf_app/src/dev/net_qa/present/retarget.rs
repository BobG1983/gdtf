use bevy::{camera::RenderTarget, prelude::*, ui::IsDefaultUiCamera};
use gdtf_battle_presenter::WorldCamera;
use gdtf_screenshot::{QaCaptureTarget, aims_at};

use crate::states::running::UiCamera;

type GameCameras = Or<(With<WorldCamera>, With<UiCamera>)>;

/// Points the game's world and UI cameras at the offscreen capture image.
pub(in crate::dev::net_qa) fn retarget_cameras_to_offscreen(
    target: Option<Res<QaCaptureTarget>>,
    cameras: Query<(Entity, &RenderTarget), GameCameras>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    for (entity, current) in &cameras {
        if aims_at(current, &target) {
            continue;
        }
        commands
            .entity(entity)
            .insert(RenderTarget::Image((**target).clone()));
    }
}

pub(in crate::dev::net_qa) fn mark_ui_default_camera(
    ui_cameras: Query<Entity, (With<UiCamera>, Without<IsDefaultUiCamera>)>,
    mut commands: Commands,
) {
    for entity in &ui_cameras {
        commands.entity(entity).insert(IsDefaultUiCamera);
    }
}
