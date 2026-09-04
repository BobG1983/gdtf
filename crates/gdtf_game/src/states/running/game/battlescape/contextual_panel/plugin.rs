use bevy::prelude::*;
use gdtf_battle_input::contextual::{
    EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct, OpenDoorAct, ShoveAct,
    StabilizeAct, ThrowGrenadeAct,
};
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::{
        bottom_bar::{despawn_bottom_bar, spawn_bottom_bar},
        contextual_panel::{
            acts,
            registrar::{
                ContextualPanelActAppExt, ContextualPanelSpawnSystems, ContextualPanelSystems,
                configure_contextual_panel_sets,
            },
            systems::{
                despawn_contextual_panel, order_contextual_buttons,
                rank_visible_contextual_buttons, spawn_contextual_panel,
                sync_panel_root_visibility,
            },
        },
    },
};

pub(in crate::states::running::game::battlescape) struct ContextualPanelPlugin;

impl Plugin for ContextualPanelPlugin {
    fn build(&self, app: &mut App) {
        configure_contextual_panel_sets(app);

        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            (
                spawn_contextual_panel
                    .in_set(ContextualPanelSpawnSystems::Root)
                    .after(spawn_bottom_bar),
                order_contextual_buttons.in_set(ContextualPanelSpawnSystems::Order),
            ),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            despawn_contextual_panel.before(despawn_bottom_bar),
        )
        .add_systems(
            Update,
            (
                sync_panel_root_visibility.after(ContextualPanelSystems::Toggle),
                rank_visible_contextual_buttons.in_set(ContextualPanelSystems::Rank),
            )
                .run_if(resource_exists::<BattleInProgress>),
        );

        app.add_contextual_act_button::<ExecuteAct, _>(acts::execute::offer_execute)
            .add_contextual_act_button::<StabilizeAct, _>(acts::stabilize::offer_stabilize)
            .add_contextual_act_button::<MeleeAct, _>(acts::melee::offer_melee)
            .add_contextual_act_button::<ShoveAct, _>(acts::shove::offer_shove)
            .add_contextual_act_button::<OpenDoorAct, _>(acts::open_door::offer_open_door)
            .add_contextual_act_button::<EnterEmplacementAct, _>(
                acts::enter_emplacement::offer_enter_emplacement,
            )
            .add_contextual_act_button::<ExitEmplacementAct, _>(
                acts::exit_emplacement::offer_exit_emplacement,
            )
            .add_contextual_act_button::<ThrowGrenadeAct, _>(
                acts::throw_grenade::offer_throw_grenade,
            );
    }
}
