use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};

use crate::states::running::game::battlescape::select_cycle::components::{
    SelectNextButton, SelectPrevButton,
};

type PressedButton<M> = (Changed<Interaction>, With<M>);

const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

pub(in crate::states::running::game::battlescape) fn select_cycle_button_intents(
    mut pending: ResMut<PendingActIntent>,
    next: Query<&Interaction, PressedButton<SelectNextButton>>,
    prev: Query<&Interaction, PressedButton<SelectPrevButton>>,
) {
    if next.iter().copied().any(is_press) {
        pending.push(ActIntent::SelectNext);
    }
    if prev.iter().copied().any(is_press) {
        pending.push(ActIntent::SelectPrev);
    }
}
