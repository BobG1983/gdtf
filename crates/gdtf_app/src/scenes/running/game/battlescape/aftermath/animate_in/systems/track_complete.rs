use bevy::prelude::*;

use crate::scenes::running::game::battlescape::aftermath::animate_in::resources::AfterMathAnimateInComplete;

pub(in crate::scenes::running::game::battlescape::aftermath::animate_in) fn game_battlescape_aftermath_animate_in_complete(
    mut commands: Commands,
) {
    commands.insert_resource(AfterMathAnimateInComplete);
}
