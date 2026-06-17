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
use gdtf_battle_input::{dispatch_act_intents, left_click_act, right_click_turn_to_face};
use gdtf_battle_sim::BattleInProgress;
use gdtf_ui::themed::UiSystems;

use crate::{
    scenes::running::game::battlescape::action_bar::systems::{
        action_bar_button_intents, despawn_action_bar, despawn_fire_mode_picker,
        dismiss_fire_mode_picker_on_scrim, flee_button_pressed, select_fire_mode_entry,
        spawn_action_bar, sync_aim_button_active, sync_fire_mode_picker_caption,
        sync_world_click_suppression, toggle_fire_mode_picker,
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
            )
            // GTW-254: the fire-mode popup picker — opener-press toggle, entry-select,
            // scrim-dismiss, the opener-caption sync (the active-mode display), and the
            // cross-crate world-click suppression. All gated on the SAME live-battle
            // witness so they are inert outside a live battle. The picker systems read
            // their own disjoint markers (opener / entry / scrim), so they do not conflict
            // with `action_bar_button_intents` (which reads only the cycle/level markers).
            //
            // Ordered `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3): the picker's
            // entry / panel children are `Themed`, and `gdtf_ui::apply_theme` (change-driven)
            // INSERTs their look the frame after they spawn — the SAME frame a press may
            // CLOSE the picker. Running the despawners after `apply_theme` keeps its inserts
            // ahead of the despawn in the command-apply order, so a close never races a
            // theme-insert onto an about-to-die entity (the despawned-entity command error).
            .add_systems(
                Update,
                (
                    toggle_fire_mode_picker,
                    select_fire_mode_entry,
                    dismiss_fire_mode_picker_on_scrim,
                    sync_fire_mode_picker_caption,
                )
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                sync_world_click_suppression
                    .before(left_click_act)
                    .before(right_click_turn_to_face)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // The picker is spawned at runtime (not in `spawn_action_bar`), so a picker
            // left open when the battle ends needs its own `OnExit` cleanup (the
            // action-bar / status-panel battle-scoped cleanup precedent).
            .add_systems(
                OnExit(BattleScapeState::BattleRunning),
                despawn_fire_mode_picker,
            );
    }
}
