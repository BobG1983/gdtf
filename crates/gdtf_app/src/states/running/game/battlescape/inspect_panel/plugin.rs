//! The inspect-panel scene-plugin (GTW-274).
//!
//! Registers the battle-scoped inspect panel in the battlescape neighborhood, beside the
//! status panel + action bar. It mirrors the status panel's lifecycle and gating:
//!
//! - **Lifecycle** — `spawn_inspect_panel` `OnEnter(BattleScapeState::BattleRunning)`,
//!   `despawn_inspect_panel` `OnExit(BattleScapeState::BattleRunning)`.
//! - **Update** — `update_inspect_panel` in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)` (`bevy-traps.md` #1), ordered
//!   `.after(InputSystems::Gather)` so it observes the SAME update's `pick_hovered_cell`
//!   write to `InspectTarget` (the GTW-264 ordering rationale — read the hover after it is
//!   gathered, not the previous frame's).
//!
//! Pure view: it reads the input crate's `InspectTarget` + the sim's `OccupancyGrid` /
//! `CoverLedger` / on-entity vital components, owns no combat rule, and writes no sim/input.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::inspect_panel::systems::{
        despawn_inspect_panel, spawn_inspect_panel, update_inspect_panel,
    },
};

/// The inspect-panel scene-plugin — spawns/despawns the panel on the `BattleRunning` boundary
/// and runs its repaint system gated on the live-battle witness.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeInspectPanelScenePlugin;

impl Plugin for GameBattleScapeInspectPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            spawn_inspect_panel,
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            despawn_inspect_panel,
        )
        .add_systems(
            Update,
            update_inspect_panel
                .after(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>),
        );
    }
}
