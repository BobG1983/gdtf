//! The action-bar scene-plugin (GTW-228 / GTW-48 S9 / 222c).
//!
//! Registers the battle-scoped themed action-bar in the battlescape neighborhood,
//! beside the presenter + input plugins:
//!
//! - **Lifecycle** — `spawn_action_bar` `OnEnter(BattleScapeState::BattleRunning)`,
//!   `despawn_action_bar` `OnExit(BattleScapeState::BattleRunning)`, so the bar exists
//!   only during the live tactical layer (NOT the whole `GameState::BattleScape`, which
//!   spans the `Generation` → `AnimateIn` → `BattleRunning` → `AnimateOut` → `AfterMath`
//!   walk). This is the contract's chosen lifecycle clause.
//! - **Interactivity** — `action_bar_button_intents` runs in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)`, the SAME live-battle witness the S7
//!   input + presenter draws gate on (`bevy-traps.md` #1), so a press is inert when no
//!   battle is live — no press is a silent no-op during generation/aftermath because the
//!   bar is only spawned in `BattleRunning` AND the action system only runs while the
//!   battle witness is present.
//!
//! The bar WRITES the shared 222a [`PendingActIntent`](gdtf_battle_input::PendingActIntent)
//! seam that `gdtf_battle_input`'s keyboard surface also writes (parallel surfaces, one
//! drain). The action system runs UPSTREAM of
//! `gdtf_battle_input::dispatch_act_intents` (registered `.after` its keyboard writers,
//! draining the same update's pushes), so a button press queued this update is drained
//! this update — exactly like a key press. The bar reaches the sim acts ONLY via the
//! `gdtf_app -> gdtf_battle_input` DATA seam, never a reverse edge or a cross-crate fn
//! (ADR-0001).

use bevy::prelude::*;
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_sim::BattleInProgress;

use crate::{
    scenes::running::game::battlescape::action_bar::systems::{
        action_bar_button_intents, despawn_action_bar, spawn_action_bar,
    },
    states::BattleScapeState,
};

/// The action-bar scene-plugin — spawns/despawns the bar on the `BattleRunning`
/// boundary and runs its button-action system gated on the live-battle witness.
pub(in crate::scenes::running::game::battlescape) struct GameBattleScapeActionBarScenePlugin;

impl Plugin for GameBattleScapeActionBarScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_action_bar)
            .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_action_bar)
            .add_systems(
                Update,
                action_bar_button_intents
                    // Ordered `.before` the ONE intent drain (`bevy-traps.md` #3): a button
                    // press queued THIS update is drained THIS update — the same-frame
                    // guarantee the keyboard writers get (they are registered `.before`
                    // the drain in `gdtf_battle_input`). Without this the press could be
                    // queued AFTER the drain ran and only take effect a frame late.
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
