use bevy::prelude::*;
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::{
    InputSystems,
    gamepad::{gamepad_click_act, gamepad_turn},
    intent::dispatch_act_intents,
    keybinds::Keybinds,
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    selection::{left_click_act, right_click_turn_to_face},
};

pub(super) fn register_keyboard_acts(app: &mut App) {
    app.add_systems(
        Update,
        (level_keys, full_view_key)
            .in_set(InputSystems::Gather)
            .run_if(resource_exists::<BattleInProgress>.and_then(resource_exists::<Keybinds>)),
    )
    .add_systems(
        Update,
        (select_clear_key, posture_keys, cycle_selection_keys)
            .in_set(InputSystems::Gather)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<Keybinds>)
                    .and_then(playback_caught_up),
            ),
    );
}

pub(super) fn register_intent_dispatch(app: &mut App) {
    app.add_systems(
        Update,
        dispatch_act_intents
            .in_set(InputSystems::Gather)
            .after(level_keys)
            .after(full_view_key)
            .after(select_clear_key)
            .after(posture_keys)
            .after(cycle_selection_keys)
            .after(left_click_act)
            .after(right_click_turn_to_face)
            .after(gamepad_click_act)
            .after(gamepad_turn)
            .run_if(resource_exists::<BattleInProgress>),
    );
}
