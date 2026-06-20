use bevy::prelude::*;

use crate::states::running::game::battlescape::aftermath::animate_out::resources::AfterMathAnimateOutComplete;

pub(in crate::states::running::game::battlescape::aftermath::animate_out) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<AfterMathAnimateOutComplete>();
}
