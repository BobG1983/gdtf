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
//! - **Aim toggle visual** (GTW-253) — `sync_aim_button_active` runs in `Update` under
//!   the SAME `run_if(resource_exists::<BattleInProgress>)` gate; it mirrors the selected
//!   ganger's `Aiming` onto the `gdtf_ui` `ActiveButton` paint marker on the
//!   `AimToggleButton`, so the Aim button visibly shows ON/OFF. It is VISUAL-ONLY — it
//!   does NOT change how the aim toggle works (the existing `AimToggle` intent path is
//!   untouched).
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
        action_bar_button_intents, despawn_action_bar, flee_button_pressed, spawn_action_bar,
        sync_aim_button_active,
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
            )
            // The flee button (GTW-240): an ENABLED app/lifecycle button whose press ends the
            // persisting battle. Gated on the SAME live-battle witness so a press is inert
            // outside a live battle (AC3). It needs NO `.before(dispatch_act_intents)` ordering
            // — it writes the `BattleRunningComplete` lifecycle marker directly (via the typed
            // `insert_battle_running_complete` door), never the intent seam — so it is ordered
            // independently.
            .add_systems(
                Update,
                flee_button_pressed.run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-253: drive the Aim button's active (toggled-on) look from the selected
            // ganger's `Aiming`. Gated on the SAME live-battle witness so it is inert
            // outside a live battle. It is VISUAL-ONLY — it inserts/removes the gdtf_ui
            // `ActiveButton` paint marker via `Commands`, never touching the act/intent
            // seam or how the aim toggle works. Unordered relative to the press systems:
            // it reflects the CURRENT `Aiming` (after the sim applied a toggle), so the
            // button look may lag a press by at most one frame, which is fine for a visual
            // indicator.
            .add_systems(
                Update,
                sync_aim_button_active.run_if(resource_exists::<BattleInProgress>),
            );
    }
}
