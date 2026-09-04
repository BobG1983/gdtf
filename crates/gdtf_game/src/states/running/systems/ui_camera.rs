use bevy::{
    camera::ClearColorConfig,
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub(crate) struct UiCamera;

pub(in crate::states::running) fn spawn_ui_camera(mut commands: Commands) {
    let camera = Camera {
        clear_color: ClearColorConfig::None,
        ..default()
    };
    commands.spawn_scene((bsn! { Camera2d UiCamera }, template_value(camera)));
}
