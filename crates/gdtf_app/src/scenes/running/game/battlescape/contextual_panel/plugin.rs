//! The contextual-panel scene-plugin (GTW-294).
//!
//! Registers the battle-scoped contextual cluster in the battlescape neighborhood, beside the
//! action-bar / bottom-bar / weapon-panel plugins:
//!
//! - **Lifecycle** (mirrors the sibling action bar / bottom bar) — `spawn_contextual_panel`
//!   `OnEnter(BattleScapeState::BattleRunning)`, `despawn_contextual_panel`
//!   `OnExit(BattleScapeState::BattleRunning)`, so the panel exists only during the live
//!   tactical layer (NOT the whole `GameState::BattleScape`).
//! - **Target resource** — `init_resource::<ContextualTargets>` inserts the
//!   [`ContextualTargets`] seam the detection system writes each update (its default is the
//!   `None` / `None` no-offer state).
//! - **Detection** — `detect_contextual_targets` runs in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)` (the SAME live-battle witness the input +
//!   presenter gate on, `bevy-traps.md` #1). It reads the selection, finds the actionable
//!   downed neighbours, writes [`ContextualTargets`], and toggles the panel root + each
//!   button's `Visibility` IN PLACE (never despawning the scaffold — `ui-mutate-not-respawn`).
//! - **Press routing** — `contextual_button_intents` runs in `Update` under the same gate and
//!   ordered `.before(dispatch_act_intents)` (the action-bar precedent, `bevy-traps.md` #3): a
//!   press queued this update is drained this update. It pushes a target-carrying
//!   `ActIntent::Execute` / `ActIntent::Stabilize` onto the shared 222a
//!   [`PendingActIntent`](gdtf_battle_input::PendingActIntent) seam — the ONE
//!   `dispatch_act_intents` drain interprets it (the bar / keys' parallel-surface, single-drain
//!   contract). It is ordered `.after(detect_contextual_targets)` so a press reads the SAME
//!   update's freshly-detected target.
//!
//! View-only — it owns no sim/input state; it reads the input selection + writes the shared
//! intent seam, never a sim component directly.

use bevy::prelude::*;
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_sim::BattleInProgress;

use crate::{
    scenes::running::game::battlescape::contextual_panel::{
        components::ContextualTargets,
        systems::{
            contextual_button_intents, despawn_contextual_panel, detect_contextual_targets,
            spawn_contextual_panel,
        },
    },
    states::BattleScapeState,
};

/// The contextual-panel scene-plugin — inits the [`ContextualTargets`] seam, spawns / despawns
/// the bottom-right contextual cluster on the `BattleRunning` boundary, and runs the live
/// detection + press-routing systems gated on the live-battle witness (GTW-294).
pub(in crate::scenes::running::game::battlescape) struct ContextualPanelPlugin;

impl Plugin for ContextualPanelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ContextualTargets>()
            .add_systems(
                OnEnter(BattleScapeState::BattleRunning),
                spawn_contextual_panel,
            )
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_contextual_panel,
            )
            .add_systems(
                Update,
                (
                    detect_contextual_targets,
                    // Ordered `.after` detection so the press reads the SAME update's target,
                    // and `.before` the ONE intent drain so a press queued this update is
                    // drained this update (`bevy-traps.md` #3 — the action-bar precedent).
                    contextual_button_intents
                        .after(detect_contextual_targets)
                        .before(dispatch_act_intents),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
