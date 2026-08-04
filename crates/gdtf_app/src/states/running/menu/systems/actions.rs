use bevy::{prelude::*, ui::Interaction};
use gdtf_ui::{DisabledButton, focus_nav::FocusActivated};

use crate::states::{
    RunningState,
    running::menu::{
        StartBattleRequested,
        components::{BattlescapeButton, OptionsButton, QuitButton},
    },
};

#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
struct MenuActionTarget(RunningState);

impl MenuActionTarget {
    const OPTIONS: Self = Self(RunningState::Options);
    const QUIT: Self = Self(RunningState::Quit);
}

type PressedButton<M> = (Changed<Interaction>, With<M>, Without<DisabledButton>);

const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

pub(in crate::states::running::menu) fn mouse_button_actions(
    mut next: ResMut<NextState<RunningState>>,
    mut start_battle: MessageWriter<StartBattleRequested>,
    battlescape: Query<&Interaction, PressedButton<BattlescapeButton>>,
    options: Query<&Interaction, PressedButton<OptionsButton>>,
    quit: Query<&Interaction, PressedButton<QuitButton>>,
) {
    if battlescape.iter().copied().any(is_press) {
        start_battle.write(StartBattleRequested::new(None));
    }
    if options.iter().copied().any(is_press) {
        next.set(*MenuActionTarget::OPTIONS);
    }
    if quit.iter().copied().any(is_press) {
        next.set(*MenuActionTarget::QUIT);
    }
}

pub(in crate::states::running::menu) fn focus_activated_actions(
    mut activations: MessageReader<FocusActivated>,
    mut next: ResMut<NextState<RunningState>>,
    mut start_battle: MessageWriter<StartBattleRequested>,
    battlescape: Query<(), (With<BattlescapeButton>, Without<DisabledButton>)>,
    options: Query<(), (With<OptionsButton>, Without<DisabledButton>)>,
    quit: Query<(), (With<QuitButton>, Without<DisabledButton>)>,
) {
    for activated in activations.read() {
        let entity = **activated;
        if battlescape.contains(entity) {
            start_battle.write(StartBattleRequested::new(None));
        } else if options.contains(entity) {
            next.set(*MenuActionTarget::OPTIONS);
        } else if quit.contains(entity) {
            next.set(*MenuActionTarget::QUIT);
        }
    }
}
