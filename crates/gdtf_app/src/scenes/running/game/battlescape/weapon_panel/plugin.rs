//! The weapon-panel scene-plugin (GTW-275, bottom-left).
//!
//! Registers the battle-scoped weapon HUD panel in the battlescape neighborhood, beside
//! the action-bar / status-panel / presenter / input plugins. The panel shows the selected
//! player ganger's weapon (graphic placeholder + name + magazine `cur/max`) and a LIVE
//! Reload button — UI/view only (it reads the sim's weapon components + the input crate's
//! `SelectedShooter` selection seam; its ONLY write is the input crate's act-intent seam).
//!
//! - **Lifecycle** (mirrors the sibling status panel) — `spawn_weapon_panel`
//!   `OnEnter(BattleScapeState::BattleRunning)`, `despawn_weapon_panel`
//!   `OnExit(BattleScapeState::BattleRunning)`, so the panel exists only during the live
//!   tactical layer (NOT the whole `GameState::BattleScape`).
//! - **Update** — `update_weapon_panel` runs in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)` (the live-battle witness, `bevy-traps.md`
//!   #1), `.after(InputSystems::Gather)` so it observes the SAME update's auto-select write
//!   to `SelectedShooter` (the GTW-264 status-panel ordering precedent). It mutates the
//!   panel widgets from the selection in place ([[ui-mutate-not-respawn]]).
//! - **Reload press** — `reload_button_pressed` runs in `Update` under the same live-battle
//!   gate, ordered `.before(dispatch_act_intents)` so a press queued this update is drained
//!   this update (the action-bar same-frame guarantee, `bevy-traps.md` #3). It WRITES the
//!   shared `PendingActIntent` seam (buttons + keys are PARALLEL surfaces over the ONE
//!   drain) — the only write into the sim/input the panel makes.
//!
//! The panel reaches the sim acts ONLY via the `gdtf_app -> gdtf_battle_input` DATA seam
//! (the act-intent queue), never a reverse edge or a cross-crate fn (ADR-0001). It deps
//! `gdtf_ui` (spawn helpers) + `gdtf_battle_input` (the selection + intent seam) + reads the
//! sim's weapon components, all already on the app's edge; the chain stays acyclic.

use bevy::prelude::*;
use gdtf_battle_input::{InputSystems, dispatch_act_intents};
use gdtf_battle_sim::BattleInProgress;

use crate::{
    scenes::running::game::battlescape::weapon_panel::systems::{
        despawn_weapon_panel, reload_button_pressed, spawn_weapon_panel, update_weapon_panel,
    },
    states::BattleScapeState,
};

/// The weapon-panel scene-plugin — spawns/despawns the panel on the `BattleRunning`
/// boundary and runs its repaint + reload-press systems gated on the live-battle witness.
pub(in crate::scenes::running::game::battlescape) struct GameBattleScapeWeaponPanelScenePlugin;

impl Plugin for GameBattleScapeWeaponPanelScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_weapon_panel)
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_weapon_panel,
            )
            .add_systems(
                Update,
                update_weapon_panel
                    // Run AFTER the input crate's `InputSystems::Gather` band, where the
                    // GTW-255 auto-select writes the initial `SelectedShooter` — else the
                    // panel reads the selection before auto-select fills it (the GTW-264
                    // status-panel ordering precedent, `bevy-traps.md` #3).
                    .after(InputSystems::Gather)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                reload_button_pressed
                    // Ordered `.before` the ONE intent drain (`bevy-traps.md` #3): a press
                    // queued THIS update is drained THIS update (the action-bar same-frame
                    // guarantee). Gated on the live-battle witness so a press is inert when
                    // no battle is live.
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
