use bevy::prelude::*;

use crate::states::running::game::battlescape::aftermath::animate_in::resources::AfterMathAnimateInComplete;

pub(in crate::states::running::game::battlescape::aftermath::animate_in) fn cleanup(
    mut commands: Commands,
) {
    commands.remove_resource::<AfterMathAnimateInComplete>();
}
