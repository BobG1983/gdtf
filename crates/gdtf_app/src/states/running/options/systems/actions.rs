use bevy::{prelude::*, ui::Interaction, ui_widgets::Activate};
use gdtf_ui::focus_nav::FocusActivated;

use crate::states::{RunningState, running::options::components::ContinueButton};

type ContinuePressedFilter = (Changed<Interaction>, With<ContinueButton>);

pub(in crate::states::running::options) fn bridge_continue_activation(
    mut commands: Commands,
    pressed: Query<(Entity, &Interaction), ContinuePressedFilter>,
    mut activations: MessageReader<FocusActivated>,
    buttons: Query<(), With<ContinueButton>>,
) {
    for (entity, interaction) in &pressed {
        if matches!(interaction, Interaction::Pressed) {
            commands.trigger(Activate { entity });
        }
    }
    for activated in activations.read() {
        if buttons.contains(**activated) {
            commands.trigger(Activate {
                entity: **activated,
            });
        }
    }
}

pub(in crate::states::running::options) fn continue_activated(
    activate: On<Activate>,
    buttons: Query<(), With<ContinueButton>>,
    mut next: ResMut<NextState<RunningState>>,
) {
    if buttons.contains(activate.entity) {
        next.set(RunningState::Menu);
    }
}
