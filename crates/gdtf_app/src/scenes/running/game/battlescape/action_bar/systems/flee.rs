//! [`flee_button_pressed`] — the dedicated flee-battle handler (GTW-240).
//!
//! Flee is an APP/LIFECYCLE act, NOT a sim verb. Unlike the five sim-act buttons
//! (`action_bar_button_intents` → the [`PendingActIntent`](gdtf_battle_input::PendingActIntent)
//! seam), a flee press is NOT routed through the
//! [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain: that drain maps each
//! [`ActIntent`](gdtf_battle_input::ActIntent) to a sim `*Requested` against the
//! `SelectedShooter`, and flee has no actor, no TU, and no `*Requested` — routing it through the
//! seam would force a fake non-sim `ActIntent` variant. So this is a SEPARATE handler that ends
//! the battle directly: on a fresh press of the ENABLED [`FleeButton`] it inserts the existing
//! `BattleRunningComplete` end-signal marker (via the typed
//! [`insert_battle_running_complete`](crate::scenes::running::game::battlescape::battle_running::insert_battle_running_complete)
//! door), so the existing marker-gated `move_on` advances
//! `BattleRunning → AnimateOut → AfterMath` — the same lifecycle door the now-removed turn-budget
//! auto-exit used (GTW-236) and the GTW-239 victory census uses. Flee is one new WRITER of an
//! existing signal, NOT a new transition path.
//!
//! ## The press-read MECHANISM (shared with the sim-act buttons)
//!
//! It reuses the SAME [`PressedButton<M>`](super::actions::PressedButton) filter +
//! [`is_press`](super::actions::is_press) test the sim-act buttons use (`bevy-traps.md` #6):
//! `Changed<Interaction>` limits the query to
//! the frame a press LANDS, and `Without<DisabledButton>` (part of `PressedButton`) INCLUDES the
//! enabled flee button (flee carries NO `DisabledButton`, unlike the deferred reload / end-turn
//! buttons). Real pointer production of [`Interaction::Pressed`] is end-to-end for free under
//! `DefaultPlugins` (`ui_focus_system` → the GTW-120 UI camera); the headless tests inject it.
//!
//! ## Gating (`bevy-traps.md` #1)
//!
//! Registered `run_if(resource_exists::<BattleInProgress>)` by the action-bar plugin — the SAME
//! live-battle witness the sim-act button system gates on — so a press is inert when no battle is
//! live (it never inserts the marker). Unlike `action_bar_button_intents` it needs NO
//! `.before(dispatch_act_intents)` ordering: it touches the lifecycle marker directly, never the
//! intent seam.

use bevy::prelude::*;

use super::actions::{PressedButton, is_press};
use crate::scenes::running::game::battlescape::{
    action_bar::components::FleeButton, battle_running::insert_battle_running_complete,
};

/// On a fresh press of the ENABLED [`FleeButton`], end the live battle by inserting the
/// `BattleRunningComplete` end-signal marker (so the marker-gated `move_on` advances
/// `BattleRunning → AnimateOut`).
///
/// A flee press is the player's explicit "I'm leaving" out (requirement 5(b)). The handler is
/// the dedicated lifecycle path — it does NOT push an [`ActIntent`](gdtf_battle_input::ActIntent)
/// and does NOT emit any `gdtf_battle_sim::acts::*Requested` (flee is not a sim verb). The plugin
/// gates it `run_if(resource_exists::<BattleInProgress>)`, so it never runs outside a live battle
/// (AC3). Inserting the marker is idempotent — a repeated press is a harmless re-insert of the
/// unit marker.
///
/// Param-only (`bevy-traps.md` #7): one read-only `Query<&Interaction, …>` + [`Commands`] — no
/// `&mut World`. The `insert_battle_running_complete` door takes `&mut Commands`, keeping the
/// `BattleRunningComplete` resource encapsulated in its `battle_running` module.
pub(in crate::scenes::running::game::battlescape::action_bar) fn flee_button_pressed(
    mut commands: Commands,
    flee: Query<&Interaction, PressedButton<FleeButton>>,
) {
    if flee.iter().copied().any(is_press) {
        insert_battle_running_complete(&mut commands);
    }
}
