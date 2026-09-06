use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_ui::{DisabledButton, focus_nav::FocusActivated};

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

pub(in crate::states::running::game::battlescape) fn reload_button_focus_activated(
    mut activations: MessageReader<FocusActivated>,
    mut pending: ResMut<PendingActIntent>,
    reload: Query<&Visibility, (With<ReloadButton>, Without<DisabledButton>)>,
) {
    for activated in activations.read() {
        if reload
            .get(**activated)
            .is_ok_and(|visibility| *visibility != Visibility::Hidden)
        {
            pending.push(ActIntent::Reload);
        }
    }
}
