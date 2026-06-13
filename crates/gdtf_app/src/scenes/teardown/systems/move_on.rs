use bevy::prelude::*;

pub(in crate::scenes::teardown) fn move_on(mut exit: MessageWriter<AppExit>) {
    exit.write(AppExit::Success);
}
