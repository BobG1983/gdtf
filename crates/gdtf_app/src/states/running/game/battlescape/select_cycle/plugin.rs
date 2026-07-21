//! The selection-cycle scene-plugin (GTW-458).
//!
//! Registers the battle-scoped Prev/Next selection-cycle cluster in the battlescape
//! neighborhood, beside the weapon-panel / action-bar / presenter / input plugins:
//!
//! - **Lifecycle** — `spawn_select_cycle` `OnEnter(BattleScapeState::BattleRunning)` ordered
//!   `.after(spawn_bottom_bar)` (the bar root must exist so the cluster parents INSIDE it — the
//!   weapon-panel Stance-Panel precedent), `despawn_select_cycle`
//!   `OnExit(BattleScapeState::BattleRunning)`, so the cluster exists only during the live
//!   tactical layer. (When the cluster is a child of the bar, the bar's own despawn also tears
//!   it down; the explicit despawn covers the defensive free-root fallback.)
//! - **Interactivity** — `select_cycle_button_intents` runs in `Update` ordered
//!   `.before(dispatch_act_intents)` (the same-frame guarantee, `bevy-traps.md` #3) and gated
//!   `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness the input + other
//!   bars gate on, `bevy-traps.md` #1), so a press is inert when no battle is live.
//!
//! The cluster WRITES the shared [`PendingActIntent`](gdtf_battle_input::PendingActIntent) seam
//! the `gdtf_battle_input` keyboard surface also writes (parallel surfaces, one drain — ADR-0001).
//! It reaches the sim acts ONLY via the `gdtf_app -> gdtf_battle_input` DATA seam, never a
//! reverse edge or a cross-crate fn. It deps `gdtf_ui` (spawn helpers) + `gdtf_battle_input`
//! (the intent seam), both already on the app's edge; the chain stays acyclic.

use bevy::prelude::*;
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::{
        bottom_bar::spawn_bottom_bar,
        select_cycle::systems::{
            despawn_select_cycle, select_cycle_button_intents, spawn_select_cycle,
        },
    },
};

/// The selection-cycle scene-plugin — spawns/despawns the cluster on the `BattleRunning`
/// boundary and runs its press → intent router gated on the live-battle witness.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeSelectCycleScenePlugin;

impl Plugin for GameBattleScapeSelectCycleScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(BattleScapeState::BattleRunning),
            // Ordered `.after(spawn_bottom_bar)` (the Stance-Panel precedent): the bottom-bar
            // root must exist so the cluster parents INSIDE it. Both run on the same `OnEnter`
            // boundary, so the order must be explicit (`bevy-traps.md` #3).
            spawn_select_cycle.after(spawn_bottom_bar),
        )
        .add_systems(
            OnExit(BattleScapeState::BattleRunning),
            despawn_select_cycle,
        )
        .add_systems(
            Update,
            select_cycle_button_intents
                // Ordered `.before` the ONE intent drain (`bevy-traps.md` #3): a press queued
                // THIS update is drained THIS update (the action-bar same-frame guarantee).
                // GTW-727 C24: a selection change is blocked while the presenter catches up.
                .before(dispatch_act_intents)
                .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
        );
    }
}
