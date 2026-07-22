//! Routes the weapon panel's LIVE Reload button press to the shared act-intent queue
//! (GTW-275).
//!
//! Mirrors the action-bar `action_bar_button_intents` mechanism: the Reload button is a
//! parallel surface over the ONE [`PendingActIntent`] data queue (keys + buttons share one
//! dispatch, ADR-0001). A press [`push`](PendingActIntent::push)es
//! [`ActIntent::Reload`](gdtf_battle_input::ActIntent::Reload); the SINGLE
//! [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain emits a
//! [`ReloadRequested`](gdtf_battle_sim::acts::ReloadRequested) for the `SelectedShooter`,
//! which the sim's `dispatch_reload` runs (the per-weapon `reload_tu` cost + refill). The
//! panel writes NO `*Requested` directly — it only writes the intent.
//!
//! Registered `run_if(resource_exists::<BattleInProgress>)` + `.before(dispatch_act_intents)`
//! by the weapon-panel plugin (the action-bar precedent, `bevy-traps.md` #1 / #3), so a
//! press queued this update is drained this update — the same-frame guarantee a key press
//! gets, and inert when no battle is live.

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};

use crate::states::running::game::battlescape::weapon_panel::components::ReloadButton;

/// Routes a fresh Reload-button press to [`ActIntent::Reload`] on the shared queue.
///
/// For the [`ReloadButton`] whose [`Interaction`] changed to
/// [`Pressed`](Interaction::Pressed) this frame, [`push`](PendingActIntent::push)es
/// [`ActIntent::Reload`] — the drain then emits a
/// [`ReloadRequested`](gdtf_battle_sim::acts::ReloadRequested) for the `SelectedShooter`
/// (a no-op in the drain with no selection, AC6). `Changed<Interaction>` limits the query
/// to the frame the press lands (a held button does not re-fire — the action-bar
/// precedent). The button is LIVE (no `DisabledButton`), so it is NOT filtered out.
///
/// Param-only (`bevy-traps.md` #7): a read-only `Query<&Interaction, …>` + the
/// `ResMut<PendingActIntent>` write — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn reload_button_pressed(
    mut pending: ResMut<PendingActIntent>,
    reload: Query<&Interaction, (Changed<Interaction>, With<ReloadButton>)>,
) {
    if reload
        .iter()
        .any(|interaction| matches!(interaction, Interaction::Pressed))
    {
        pending.push(ActIntent::Reload);
    }
}
