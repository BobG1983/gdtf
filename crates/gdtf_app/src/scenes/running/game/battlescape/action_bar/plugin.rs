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
//! - **Stance 3-toggle visual** (GTW-267) — `sync_stance_buttons_active` mirrors the
//!   selected ganger's `Stance` onto the `ActiveButton` paint marker across the three
//!   stance toggles (Stand / Kneel / Prone), mutually exclusive, under the same gate. The
//!   toggles' presses route DIRECT `ActIntent::SetStance` via `action_bar_button_intents`.
//! - **Mode 3-toggle sub-panel** (GTW-265 / GTW-284) — the three FIXED mode toggles are
//!   spawned once with the bar; `rebuild_mode_buttons` MUTATES their `Visibility` to the
//!   selected weapon's offered modes on a selection change (`.after(UiSystems::ApplyTheme)`),
//!   NEVER despawning/respawning them (GTW-284: no `Themed` churn → no spurious global
//!   repaint); `mode_button_pressed` sets `SelectedFireMode` directly to a pressed toggle's
//!   read-back spec; `sync_mode_buttons_active` marks the live mode `ActiveButton`. This
//!   REPLACED the GTW-254 popup picker (no modal, no scrim, no world click-through).
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
use gdtf_ui::themed::UiSystems;

use crate::{
    scenes::running::game::battlescape::action_bar::systems::{
        action_bar_button_intents, despawn_action_bar, flee_button_pressed, mode_button_pressed,
        nowrap_control_labels, rebuild_mode_buttons, spawn_action_bar, sync_aim_button_active,
        sync_mode_buttons_active, sync_stance_buttons_active,
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
            // GTW-267: the Stance 3-toggle active-mark sync, mirroring the selected
            // ganger's `Stance` onto the three stance toggles (mutually exclusive). Same
            // live-battle gate; VISUAL-ONLY (the press path is `action_bar_button_intents`).
            .add_systems(
                Update,
                sync_stance_buttons_active.run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-265: the Mode 3-toggle sub-panel. `mode_button_pressed` sets
            // `SelectedFireMode` directly to a pressed toggle's read-back spec;
            // `sync_mode_buttons_active` marks the live mode `ActiveButton`. They read their
            // own disjoint mode markers, so they do not conflict with the stance/level
            // `action_bar_button_intents`. Same live-battle gate.
            //
            // `sync_mode_buttons_active` is ordered `.after(mode_button_pressed)`
            // (`bevy-traps.md` #3): the sync READS `SelectedFireMode` that the press WRITES, so
            // the active mark must move the SAME update the press lands — without the explicit
            // ordering the two ran in a nondeterministic (executor-chosen) order and the active
            // mark lagged a press by a frame whenever the sync happened to run first (a latent
            // GTW-271-exposed ambiguity: adding any `Update` system can flip the executor's
            // choice). The chain expresses the real read-after-write dependency.
            .add_systems(
                Update,
                (mode_button_pressed, sync_mode_buttons_active)
                    .chain()
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-284: `rebuild_mode_buttons` MUTATES the three FIXED Mode toggles'
            // `Visibility` on a selection change (it no longer despawns/respawns them),
            // ordered `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3) so its visibility
            // writes settle deterministically relative to the theme pass. Because no toggle
            // entity is added/removed, there is no spurious `Added<Themed>` to trigger a
            // `gdtf_ui::apply_theme` repaint — the GTW-284 root-cause is gone.
            .add_systems(
                Update,
                rebuild_mode_buttons
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-298: keep the relocated firemode / aim / stance toggle LABELS on one line so a
            // too-wide caption (e.g. `full-auto`) clips inside its toggle instead of soft-
            // wrapping + overflowing the short firemode cell into the row above (contract item
            // 8). Ordered `.after(UiSystems::ApplyTheme)` so it runs after the theme pass settles
            // the freshly-spawned labels; same live-battle gate as the other control systems.
            .add_systems(
                Update,
                nowrap_control_labels
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
