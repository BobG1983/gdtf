use bevy::prelude::*;

use crate::scenes::running::game::battlescape::aftermath::animate_out::resources::AfterMathAnimateOutComplete;

pub(in crate::scenes::running::game::battlescape::aftermath::animate_out) fn game_battlescape_aftermath_animate_out_complete(
    mut commands: Commands,
) {
    commands.insert_resource(AfterMathAnimateOutComplete);
}
