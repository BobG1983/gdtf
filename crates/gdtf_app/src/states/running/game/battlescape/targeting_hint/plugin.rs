//! The targeting-hint scene-plugin (GTW-11).
//!
//! Registers the battle-scoped targeting-fog HINT Text node in the battlescape neighborhood,
//! beside the status panel + reticle. It surfaces the canon "unseen — hold your fire" string
//! when the targeted cell is non-VISIBLE — UI/view only (no sim/input change, no new act).
//!
//! - **Lifecycle** (mirrors the sibling status panel) — `spawn_targeting_hint`
//!   `OnEnter(BattleScapeState::BattleRunning)`, `despawn_targeting_hint`
//!   `OnExit(BattleScapeState::BattleRunning)`, so the hint exists only during the live tactical
//!   layer.
//! - **Update** — `update_targeting_hint` runs in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)` (the SAME live-battle witness the action-bar /
//!   status-panel / input gate on, `bevy-traps.md` #1) and `.after(InputSystems::Gather)` so it
//!   observes the SAME update's resolved `InspectTarget` hovered cell (`bevy-traps.md` #3). It
//!   reads the input crate's `InspectTarget` + the sim's `SquadVisibility` / `OccupancyGrid` /
//!   `PlayerFaction` and mutates the one hint Text node from the SHARED `cell_squad_visible` read.
//!
//! The hint reads only DATA — the hovered cell + the squad fog — never a reverse edge into the
//! sim / input. It is a pure view; it owns no combat rule.

use bevy::prelude::*;
use gdtf_battle_input::InputSystems;
use gdtf_battle_sim::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::targeting_hint::systems::{
        despawn_targeting_hint, spawn_targeting_hint, update_targeting_hint,
    },
};

/// The targeting-hint scene-plugin — spawns/despawns the hint on the `BattleRunning` boundary and
/// runs its update system gated on the live-battle witness.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeTargetingHintScenePlugin;

impl Plugin for GameBattleScapeTargetingHintScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            spawn_targeting_hint,
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            despawn_targeting_hint,
        )
        .add_systems(
            Update,
            update_targeting_hint
                // Run AFTER the input crate's `InputSystems::Gather` band, where `pick_hovered_cell`
                // resolves the `InspectTarget` this hint reads — otherwise it would read last
                // frame's hovered cell (`bevy-traps.md` #3).
                .after(InputSystems::Gather)
                .run_if(resource_exists::<BattleInProgress>),
        );
    }
}
