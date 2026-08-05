use bevy::{
    ecs::{schedule::IntoScheduleConfigs, system::ScheduleSystem},
    prelude::*,
};
use gdtf_battle_input::{contextual::ContextualActSystems, pick_hovered_cell};
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::contextual_panel::{
        seam::{ContextualOffer, ContextualPanelAct},
        systems::{
            press_contextual_button, press_contextual_button_via_key, spawn_contextual_button,
            sync_contextual_button_visibility,
        },
    },
};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ContextualPanelSystems {
    Offer,
    Toggle,
    Rank,
    Press,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::states::running::game::battlescape) enum ContextualPanelSpawnSystems {
    Root,
    Buttons,
    Order,
}

pub(in crate::states::running::game::battlescape) fn configure_contextual_panel_sets(
    app: &mut App,
) {
    app.configure_sets(
        Update,
        (
            ContextualPanelSystems::Offer,
            ContextualPanelSystems::Toggle,
            ContextualPanelSystems::Rank,
            ContextualPanelSystems::Press,
        )
            .chain(),
    )
    .configure_sets(
        Update,
        ContextualPanelSystems::Offer.before(pick_hovered_cell),
    )
    .configure_sets(
        Update,
        ContextualPanelSystems::Press.before(ContextualActSystems::Drain),
    )
    .configure_sets(
        OnEnter(BattleScapeState::BattleRunning),
        (
            ContextualPanelSpawnSystems::Root,
            ContextualPanelSpawnSystems::Buttons,
            ContextualPanelSpawnSystems::Order,
        )
            .chain(),
    );
}

pub(in crate::states::running::game::battlescape) trait ContextualPanelActAppExt {
    fn add_contextual_act_button<A: ContextualPanelAct, M>(
        &mut self,
        offer: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;
}

impl ContextualPanelActAppExt for App {
    fn add_contextual_act_button<A: ContextualPanelAct, M>(
        &mut self,
        offer: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.init_resource::<ContextualOffer<A>>()
            .add_systems(
                OnEnter(BattleScapeState::BattleRunning),
                spawn_contextual_button::<A>.in_set(ContextualPanelSpawnSystems::Buttons),
            )
            .add_systems(
                Update,
                (
                    offer.in_set(ContextualPanelSystems::Offer),
                    sync_contextual_button_visibility::<A>.in_set(ContextualPanelSystems::Toggle),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                (
                    press_contextual_button::<A>,
                    press_contextual_button_via_key::<A>,
                )
                    .chain()
                    .in_set(ContextualPanelSystems::Press)
                    .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
            )
    }
}
