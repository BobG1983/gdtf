use bevy::{camera::RenderTarget, prelude::*, ui::IsDefaultUiCamera};
use gdtf_battle_presenter::WorldCamera;

use super::target::QaCaptureTarget;
use crate::states::running::UiCamera;

type GameCameras = Or<(With<WorldCamera>, With<UiCamera>)>;

/// A `Camera` `#[require]`s a `RenderTarget` (defaulting to `RenderTarget::Window`), so a
pub(in crate::dev::net_qa) fn retarget_cameras_to_offscreen(
    target: Option<Res<QaCaptureTarget>>,
    cameras: Query<(Entity, &RenderTarget), GameCameras>,
    mut commands: Commands,
) {
    let Some(target) = target else {
        return;
    };
    let handle = &**target;
    for (entity, current) in &cameras {
        if current.as_image() == Some(handle) {
            continue;
        }
        commands
            .entity(entity)
            .insert(RenderTarget::Image(handle.clone().into()));
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
