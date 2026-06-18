//! The hover-inspect-panel scene-plugin (GTW-274).
//!
//! Registers the battle-scoped hover panel in the battlescape neighborhood, beside the
//! status panel + action bar. It mirrors the status panel's lifecycle and gating:
//!
//! - **Lifecycle** — `spawn_hover_panel` `OnEnter(BattleScapeState::BattleRunning)`,
//!   `despawn_hover_panel` `OnExit(BattleScapeState::BattleRunning)`.
//! - **Update** — `update_hover_panel` in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)` (`bevy-traps.md` #1), ordered
//!   `.after(InputSystems::Gather)` so it observes the SAME update's `pick_hovered_cell`
//!   write to `HoveredCell` (the GTW-264 ordering rationale — read the hover after it is
//!   gathered, not the previous frame's).
//!
//! Pure view: it reads the input crate's `HoveredCell` + the sim's `OccupancyGrid` /
//! `CoverLedger` / on-entity vital components, owns no combat rule, and writes no sim/input.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_sim::BattleInProgress;

use crate::{
    scenes::running::game::battlescape::hover_panel::systems::{
        despawn_hover_panel, spawn_hover_panel, update_hover_panel,
    },
    states::BattleScapeState,
};

/// The hover-panel scene-plugin — spawns/despawns the panel on the `BattleRunning` boundary
/// and runs its repaint system gated on the live-battle witness.
pub(in crate::scenes::running::game::battlescape) struct GameBattleScapeHoverPanelScenePlugin;

impl Plugin for GameBattleScapeHoverPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_hover_panel)
            .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_hover_panel)
            .add_systems(
                Update,
                update_hover_panel
                    .after(InputSystems::Gather)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
