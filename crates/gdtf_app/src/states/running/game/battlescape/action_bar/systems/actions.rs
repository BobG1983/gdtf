use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_ui::DisabledButton;

use crate::states::running::game::battlescape::action_bar::components::{
    EndTurnButton, LevelDownButton, LevelUpButton,
};

pub(in crate::states::running::game::battlescape::action_bar) type PressedButton<M> =
    (Changed<Interaction>, With<M>, Without<DisabledButton>);

pub(in crate::states::running::game::battlescape::action_bar) const fn is_press(
    interaction: Interaction,
) -> bool {
    matches!(interaction, Interaction::Pressed)
}

pub(in crate::states::running::game::battlescape) fn action_bar_button_intents(
    mut pending: ResMut<PendingActIntent>,
    level_up: Query<&Interaction, PressedButton<LevelUpButton>>,
    level_down: Query<&Interaction, PressedButton<LevelDownButton>>,
    end_turn: Query<&Interaction, PressedButton<EndTurnButton>>,
) {
    if level_up.iter().copied().any(is_press) {
        pending.push(ActIntent::LevelUp);
    }
    if level_down.iter().copied().any(is_press) {
        pending.push(ActIntent::LevelDown);
    }
    if end_turn.iter().copied().any(is_press) {
        pending.push(ActIntent::EndTurn);
    }
}
