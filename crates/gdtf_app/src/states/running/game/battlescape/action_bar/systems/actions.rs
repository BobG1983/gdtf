//! Routes an action-bar button press to the shared 222a act-intent seam (GTW-228 /
//! GTW-48 S9 / 222c).
//!
//! This is the pure interaction→intent layer: buttons + keys are PARALLEL surfaces
//! over the ONE [`PendingActIntent`] data seam, not a re-derived inline mapping. Each
//! per-act button, when pressed, [`push`](PendingActIntent::push)es the SAME
//! [`ActIntent`] the equivalent KEY pushes (the `gdtf_battle_input` keyboard surface),
//! and the SINGLE [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents)
//! drain interprets it — so there is exactly ONE emission path, never two divergent
//! marker→`*Requested` mappings (the user's exact KEYS+BUTTONS-SHARE-ONE-DISPATCH rule,
//! ADR-0001). The `gdtf_app` button systems do NOT independently emit any
//! `gdtf_battle_sim::acts::*Requested`; they only write the intent.
//!
//! ## The GTW-122 MECHANISM (mouse-press read), routed to the intent seam
//!
//! Each action system is one disjoint per-marker query filtered
//! [`PressedButton<M>`] `= (Changed<Interaction>, With<M>, Without<DisabledButton>)`,
//! acting only on [`Interaction::Pressed`] ([`is_press`]) — the menu
//! `mouse_button_actions` mechanism. `Changed<Interaction>` limits each query to the
//! frame a press LANDS (a held button does not re-fire); `Without<DisabledButton>`
//! excludes the DEFERRED reload / end-turn buttons (AC5 — they emit no intent). Real
//! pointer production of [`Interaction::Pressed`] is end-to-end for free under
//! `DefaultPlugins` (`ui_focus_system` → the GTW-120 UI camera, `bevy-traps.md` #6);
//! the headless tests inject it.
//!
//! Unlike the menu, the per-act mapping is NOT a `match`/`NextState` and is NOT a
//! marker→`*Requested` table — every press routes to the 222a [`ActIntent`] queue. One
//! system spans all enabled act buttons (its queries are disjoint per marker, so they
//! never conflict), keeping the marker→intent mapping in one place.
//!
//! ## Gating + ordering (`bevy-traps.md` #1 / #3)
//!
//! Registered `run_if(resource_exists::<BattleInProgress>)` by the action-bar plugin —
//! the same live-battle witness the S7 input + presenter draws gate on — so a press is
//! inert when no battle is live (it never even reaches the seam). It runs in `Update`
//! ordered explicitly `.before` `gdtf_battle_input`'s `dispatch_act_intents` drain
//! (`bevy-traps.md` #3), so a button press queued this update is drained this update —
//! the same-frame guarantee the keyboard writers get (they too are registered `.before`
//! the drain in `gdtf_battle_input`).

use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_ui::DisabledButton;

use crate::states::running::game::battlescape::action_bar::components::{
    EndTurnButton, LevelDownButton, LevelUpButton,
};

/// Query filter selecting the ENABLED button carrying marker `M` whose
/// [`Interaction`] became a press this frame.
///
/// Factored into a named alias both to keep [`action_bar_button_intents`]'s signature
/// legible (clippy `type_complexity`) and to make the exclusion explicit:
/// `Without<DisabledButton>` skips the DEFERRED reload / end-turn buttons (AC5), and
/// `Changed<Interaction>` limits each query to the frame a press lands. The menu
/// `PressedButton<M>` precedent.
///
/// `pub(in ...action_bar)` so the sibling `flee` system (GTW-240) reuses the SAME
/// press-read filter rather than re-deriving it — flee is `PressedButton<FleeButton>`
/// (the `Without<DisabledButton>` filter INCLUDES the enabled flee button).
pub(in crate::states::running::game::battlescape::action_bar) type PressedButton<M> =
    (Changed<Interaction>, With<M>, Without<DisabledButton>);

/// Whether an [`Interaction`] is a fresh press to act on.
///
/// Centralizes the "an activation happened" test so each per-marker query reads it
/// identically (the menu `is_press` precedent). Only [`Interaction::Pressed`] counts.
/// Takes [`Interaction`] by value (a one-byte `Copy` enum).
///
/// `pub(in ...action_bar)` so the sibling `flee` system (GTW-240) reuses the SAME
/// fresh-press test rather than re-deriving it.
pub(in crate::states::running::game::battlescape::action_bar) const fn is_press(
    interaction: Interaction,
) -> bool {
    matches!(interaction, Interaction::Pressed)
}

/// Routes each enabled action-bar button press to its [`ActIntent`] on the shared
/// 222a seam.
///
/// For each enabled act button whose [`Interaction`] changed to
/// [`Pressed`](Interaction::Pressed) this frame, [`push`](PendingActIntent::push)es the
/// matching [`ActIntent`]:
///
/// - [`LevelUpButton`] → [`ActIntent::LevelUp`]
/// - [`LevelDownButton`] → [`ActIntent::LevelDown`]
/// - [`EndTurnButton`] → [`ActIntent::EndTurn`] (GTW-309 — the fieldless GLOBAL turn signal;
///   no selection needed, like `SelectionClear` / `LevelUp`)
///
/// GTW-277 — the AIM and STANCE controls are now `gdtf_ui` widgets (a
/// [`Switch`](gdtf_ui::Switch) / a [`SegmentedControl`](gdtf_ui::SegmentedControl)), so
/// their press → intent mapping moved to the widget-native MESSAGE listeners
/// (`aim_switch_flip_intent` reads [`ToggleFlipped`](gdtf_ui::ToggleFlipped) → `AimToggle`;
/// `stance_segment_intent` reads [`SegmentSelected`](gdtf_ui::SegmentSelected) → `SetStance`),
/// NOT this `Changed<Interaction>` button-press router — which now handles the two LEVEL
/// buttons and the LIVE end-turn button (GTW-309). The MODE control sets
/// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) directly via its own
/// `mode_panel` message listener, NOT the intent seam (unchanged from GTW-265). The three
/// queries are disjoint per marker (each filtered to one role marker and
/// `Without<DisabledButton>`), so they never conflict. The buttons write NO `*Requested`
/// directly — the ONE [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents)
/// drain interprets each pushed intent (the end-turn intent emits `EndTurnRequested`
/// unconditionally — it needs no selection — while a per-actor intent with no selection is a
/// no-op in the drain, AC6).
///
/// Param-only (`bevy-traps.md` #7): three read-only `Query<&Interaction, …>`s + the
/// `ResMut<PendingActIntent>` write — no `&mut World`.
pub(in crate::states::running::game::battlescape) fn action_bar_button_intents(
    mut pending: ResMut<PendingActIntent>,
    level_up: Query<&Interaction, PressedButton<LevelUpButton>>,
    level_down: Query<&Interaction, PressedButton<LevelDownButton>>,
    end_turn: Query<&Interaction, PressedButton<EndTurnButton>>,
) {
    if level_up.iter().copied().any(is_press) {
        pending.push(ActIntent::LevelUp);
    }
    if level_down.iter().copied().any(is_press) {
        pending.push(ActIntent::LevelDown);
    }
    if end_turn.iter().copied().any(is_press) {
        pending.push(ActIntent::EndTurn);
    }
}
