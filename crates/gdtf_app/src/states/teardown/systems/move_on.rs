use bevy::{prelude::*, window::PrimaryWindow};

pub(in crate::states::teardown) fn move_on(
    mut commands: Commands,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    for window in &windows {
        commands.entity(window).despawn();
    }
    exit.write(AppExit::Success);
}
