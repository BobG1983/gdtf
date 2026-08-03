use bevy::{ecs::message::Messages, prelude::*, window::CursorMoved};
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;

use super::build::battle_act_gate;
use crate::{
    InputSystems,
    contextual::{
        ContextualActAppExt, EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct,
        OpenDoorAct, ShoveAct, StabilizeAct, ThrowGrenadeAct, configure_contextual_act_drains,
    },
    gamepad::{
        emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn, mouse_reclaims_pointer,
        move_gamepad_cursor,
    },
    intent::dispatch_act_intents,
    picking::pick_hovered_cell,
};

pub(super) fn register_contextual_acts(app: &mut App) {
    configure_contextual_act_drains(app);
    app.add_contextual_act::<ExecuteAct>()
        .add_contextual_act::<StabilizeAct>()
        .add_contextual_act::<MeleeAct>()
        .add_contextual_act::<ShoveAct>()
        .add_contextual_act::<OpenDoorAct>()
        .add_contextual_act::<EnterEmplacementAct>()
        .add_contextual_act::<ExitEmplacementAct>()
        .add_contextual_act::<ThrowGrenadeAct>();
}

pub(super) fn register_gamepad_systems(app: &mut App) {
    app.add_systems(
        Update,
        move_gamepad_cursor
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        mouse_reclaims_pointer.in_set(InputSystems::Gather).run_if(
            resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<CursorMoved>>),
        ),
    )
    .add_systems(
        Update,
        (gamepad_click_act, gamepad_turn)
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .before(dispatch_act_intents)
            .run_if(battle_act_gate().and_then(playback_caught_up)),
    )
    .add_systems(
        Update,
        emit_gamepad_cursor_move
            .in_set(InputSystems::Gather)
            .run_if(resource_exists::<BattleInProgress>),
    );
}
