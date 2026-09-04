use bevy::prelude::*;
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::metric::MAX_LEVELS;
use gdtf_ui::DisabledButton;

use crate::states::running::game::battlescape::action_bar::components::{
    LevelDownButton, LevelUpButton,
};

fn set_button_disabled(commands: &mut Commands, button: Entity, currently: bool, disabled: bool) {
    if disabled && !currently {
        commands.entity(button).insert(DisabledButton);
    } else if !disabled && currently {
        commands.entity(button).remove::<DisabledButton>();
    }
}

pub(in crate::states::running::game::battlescape) fn sync_level_button_bounds(
    mut commands: Commands,
    active: Res<ActiveLevel>,
    level_up: Query<(Entity, Has<DisabledButton>), With<LevelUpButton>>,
    level_down: Query<(Entity, Has<DisabledButton>), With<LevelDownButton>>,
) {
    let storey = *(**active);
    let ceiling = MAX_LEVELS.saturating_sub(1);

    let at_ceiling = storey >= ceiling;
    for (button, currently) in &level_up {
        set_button_disabled(&mut commands, button, currently, at_ceiling);
    }

    let at_floor = storey == 0;
    for (button, currently) in &level_down {
        set_button_disabled(&mut commands, button, currently, at_floor);
    }
}
