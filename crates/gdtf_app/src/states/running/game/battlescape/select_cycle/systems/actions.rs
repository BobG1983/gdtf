//! Routes a Prev/Next cycle button press to the shared act-intent queue (GTW-458).
//!
//! Buttons + keys are PARALLEL surfaces over the ONE
//! [`PendingActIntent`](gdtf_battle_input::PendingActIntent) data queue (ADR-0001 — keys +
//! buttons share one dispatch, never two divergent mappings). Pressing **Next** pushes the
//! SAME [`ActIntent::SelectNext`](gdtf_battle_input::ActIntent::SelectNext) the `Tab` key
//! pushes; pressing **Prev** pushes the SAME
//! [`ActIntent::SelectPrev`](gdtf_battle_input::ActIntent::SelectPrev) `Shift+Tab` pushes — and
//! the SINGLE [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain
//! interprets them (one emission path).
//!
//! The mechanism is the GTW-122 mouse-press read: one disjoint per-marker query filtered
//! `PressedButton<M> = (Changed<Interaction>, With<M>)`, acting only on
//! [`Interaction::Pressed`]. `Changed<Interaction>` limits each query to the frame a press
//! LANDS (a held button does not re-fire). Real pointer production of
//! [`Interaction::Pressed`] is end-to-end for free under `DefaultPlugins` (`ui_focus_system` →
//! the UI camera, `bevy-traps.md` #6); the headless tests inject it.
//!
//! Registered `.before(dispatch_act_intents)` and gated `run_if(resource_exists::<BattleInProgress>)`
//! by the cluster plugin (the action-bar precedent, `bevy-traps.md` #1 / #3): a press queued
//! this update is drained this update, and a press is inert when no battle is live.

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};

use crate::states::running::game::battlescape::select_cycle::components::{
    SelectNextButton, SelectPrevButton,
};

/// Query filter selecting the button carrying marker `M` whose [`Interaction`] became a press
/// this frame (the action-bar `PressedButton<M>` precedent).
///
/// Factored into a named alias to keep [`select_cycle_button_intents`]'s signature legible
/// (clippy `type_complexity`). `Changed<Interaction>` limits each query to the frame a press
/// lands. Neither cycle button is ever a `DisabledButton`, so no exclusion filter is needed.
type PressedButton<M> = (Changed<Interaction>, With<M>);

/// Whether an [`Interaction`] is a fresh press to act on (only
/// [`Interaction::Pressed`]). Takes [`Interaction`] by value (a one-byte `Copy` enum) — the
/// action-bar `is_press` precedent.
const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

/// Routes each cycle button press to its [`ActIntent`] on the shared act-intent queue (GTW-458).
///
/// For the **Next** / **Prev** button whose [`Interaction`] changed to
/// [`Pressed`](Interaction::Pressed) this frame, [`push`](PendingActIntent::push)es the
/// matching intent:
///
/// - [`SelectNextButton`] → [`ActIntent::SelectNext`]
/// - [`SelectPrevButton`] → [`ActIntent::SelectPrev`]
///
/// The two queries are disjoint per marker, so they never conflict. The buttons write NO
/// `*Requested` directly — the ONE
/// [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) drain interprets each
/// pushed intent (it cycles the [`SelectedShooter`](gdtf_battle_input::SelectedShooter) through
/// the player gang, wrapping; an empty player gang is a no-op there).
///
/// Param-only (`bevy-traps.md` #7): two read-only `Query<&Interaction, …>`s + the
/// `ResMut<PendingActIntent>` write — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn select_cycle_button_intents(
    mut pending: ResMut<PendingActIntent>,
    next: Query<&Interaction, PressedButton<SelectNextButton>>,
    prev: Query<&Interaction, PressedButton<SelectPrevButton>>,
) {
    if next.iter().copied().any(is_press) {
        pending.push(ActIntent::SelectNext);
    }
    if prev.iter().copied().any(is_press) {
        pending.push(ActIntent::SelectPrev);
    }
}
