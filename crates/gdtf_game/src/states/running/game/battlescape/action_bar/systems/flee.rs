use bevy::prelude::*;

use super::actions::{PressedButton, is_press};
use crate::states::running::game::battlescape::{
    action_bar::components::FleeButton, battle_running::insert_battle_running_complete,
};

pub(in crate::states::running::game::battlescape::action_bar) fn flee_button_pressed(
    mut commands: Commands,
    flee: Query<&Interaction, PressedButton<FleeButton>>,
) {
    if flee.iter().copied().any(is_press) {
        insert_battle_running_complete(&mut commands);
    }
}
