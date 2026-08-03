use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};

use crate::states::running::game::battlescape::weapon_panel::components::ReloadButton;

pub(in crate::states::running::game::battlescape) fn reload_button_pressed(
    mut pending: ResMut<PendingActIntent>,
    reload: Query<&Interaction, (Changed<Interaction>, With<ReloadButton>)>,
) {
    if reload
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Pressed))
    {
        pending.push(ActIntent::Reload);
    }
}
