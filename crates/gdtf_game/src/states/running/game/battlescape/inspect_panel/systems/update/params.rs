use bevy::{prelude::*, text::TextColor as UiTextColor};
use gdtf_battle_input::InspectTarget;
use gdtf_battle_presenter::ShownSquadVisibility;
use gdtf_battle_sim::{
    battle::PlayerFaction, cover::CoverLedger, prelude::OccupancyGrid, visibility::SquadVisibility,
};
use gdtf_ui::{ProgressBarFill, theme::GdtfTheme};

use crate::states::running::game::battlescape::inspect_panel::{
    components::{
        InspectObjectBar, InspectObjectBlock, InspectObjectHardness, InspectObjectHeight,
        InspectObjectProtection, InspectObjectText, InspectPanelRoot, InspectStatBlockHost,
    },
    decide::ShownBattle,
    shadow::{ShownCoverLedger, ShownEmplacements, ShownOccupancyGrid},
};

#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct InspectNodes<'w, 's> {
    pub root:         Query<'w, 's, Entity, With<InspectPanelRoot>>,
    pub host:         Query<'w, 's, Entity, With<InspectStatBlockHost>>,
    pub object_block: Query<'w, 's, Entity, With<InspectObjectBlock>>,
    pub object_title: Query<'w, 's, Entity, With<InspectObjectText>>,
    pub hardness:     Query<'w, 's, Entity, With<InspectObjectHardness>>,
    pub protection:   Query<'w, 's, Entity, With<InspectObjectProtection>>,
    pub height:       Query<'w, 's, Entity, With<InspectObjectHeight>>,
    pub object_bar:   Query<'w, 's, Entity, With<InspectObjectBar>>,
    pub display:      Query<'w, 's, &'static mut Node, Without<ProgressBarFill>>,
}

/// reads as ONE param (the [`too_many_arguments`](clippy::too_many_arguments) idiom). The grid
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct InspectReads<'w> {
    pub target:       Res<'w, InspectTarget>,
    pub grid:         Option<Res<'w, ShownOccupancyGrid>>,
    pub ledger:       Option<Res<'w, ShownCoverLedger>>,
    pub emplacements: Option<Res<'w, ShownEmplacements>>,
    pub squad:        Option<Res<'w, ShownSquadVisibility>>,
    pub player:       Option<Res<'w, PlayerFaction>>,
}

impl InspectReads<'_> {
    #[must_use]
    pub(in crate::states::running::game::battlescape) fn occupancy(
        &self,
    ) -> Option<&OccupancyGrid> {
        self.grid.as_deref().map(ShownOccupancyGrid::grid)
    }

    #[must_use]
    pub(in crate::states::running::game::battlescape) fn cover(&self) -> Option<&CoverLedger> {
        self.ledger.as_deref().map(ShownCoverLedger::ledger)
    }

    #[must_use]
    pub(in crate::states::running::game::battlescape) fn emplacements(
        &self,
    ) -> Option<&ShownEmplacements> {
        self.emplacements.as_deref()
    }

    #[must_use]
    pub(in crate::states::running::game::battlescape) fn fog(&self) -> Option<&SquadVisibility> {
        self.squad.as_deref().map(ShownSquadVisibility::visibility)
    }

    #[must_use]
    pub(in crate::states::running::game::battlescape) fn shown(&self) -> ShownBattle<'_> {
        ShownBattle::new(
            self.occupancy(),
            self.cover(),
            self.emplacements(),
            self.fog(),
            self.player.as_deref(),
        )
    }
}

/// (the [`too_many_arguments`](clippy::too_many_arguments) idiom — system analogue of the
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct FactionTint<'w, 's> {
    pub player: Option<Res<'w, PlayerFaction>>,
    pub theme:  Option<Res<'w, GdtfTheme>>,
    pub colors: Query<'w, 's, &'static mut UiTextColor>,
}
