//! The keyboard slot-binding systems (GTW-563) — the act-agnostic ranking pass that
//! numbers the currently-visible contextual buttons 1..N, plus the per-act generic
//! digit-key press router that mirrors the mouse press onto the SAME per-act intent
//! queue.
//!
//! The user's UX rule: digit key N activates the Nth CURRENTLY-VISIBLE contextual button
//! (1-based, left-to-right / [`PanelSlot`](super::super::seam::PanelSlot) order) — the
//! binding is per-SLOT, not per-action, so which act a digit fires shifts as buttons
//! appear and disappear. Two phases keep this inside the per-act-generic architecture (no
//! runtime act enum): ONE non-generic pass ranks every visible button (the rank lives on
//! each button as a [`VisibleSlotRank`]), then each per-act generic press system reads its
//! own button's rank and checks whether the digit key bound to THAT rank was just pressed.

use bevy::prelude::*;
use gdtf_battle_input::{Keybinds, SlotRank, contextual::PendingContextualIntents};

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualActButton, ContextualOffer, ContextualPanelAct, PanelSlot, VisibleSlotRank,
};

/// Ranks every CURRENTLY-VISIBLE contextual button 1..N in
/// [`PanelSlot`](super::super::seam::PanelSlot) order — the act-count-independent digit
/// binding pass (GTW-563).
///
/// A single NON-generic system (the [`ContextualActButton`] tag is carried by every
/// button of every act, so one query reaches them all): it gathers the buttons whose
/// [`Visibility`] is `Visible`, sorts them by their carried slot (entity id as the
/// tie-break, matching `order_contextual_buttons`), and writes each its 1-based position
/// as a [`VisibleSlotRank`]; every hidden button is left unranked. Runs in the
/// [`ContextualPanelSystems::Rank`](super::super::registrar::ContextualPanelSystems) set —
/// AFTER the per-act toggles wrote this frame's visibility, BEFORE the per-act press
/// routers read the ranks. Mutate-in-place, write-on-change (an unchanged rank never trips
/// `Changed<VisibleSlotRank>`).
pub(in crate::states::running::game::battlescape) fn rank_visible_contextual_buttons(
    mut buttons: Query<(
        Entity,
        &Visibility,
        &ContextualActButton,
        &mut VisibleSlotRank,
    )>,
) {
    // Phase 1 — the currently-visible buttons in deterministic (slot, entity) order.
    let mut visible: Vec<(PanelSlot, Entity)> = buttons
        .iter()
        .filter(|(_, visibility, ..)| **visibility == Visibility::Visible)
        .map(|(entity, _, button, _)| (**button, entity))
        .collect();
    visible.sort_by_key(|(slot, entity)| (*slot, *entity));

    // Phase 2 — write each button its 1-based visible rank (unranked when not visible),
    // write-on-change. N <= 9 buttons, so the per-button position scan stays trivial.
    for (entity, _, _, mut rank) in &mut buttons {
        // The button's 1-based position among the visible set (`index + 1`), guarded
        // through `try_from` so the usize→u8 narrowing never panics (N <= 9 in practice).
        let want = VisibleSlotRank::new(
            visible
                .iter()
                .position(|(_, candidate)| *candidate == entity)
                .and_then(|index| u8::try_from(index + 1).ok().map(SlotRank::new)),
        );
        if *rank != want {
            *rank = want;
        }
    }
}

/// Routes act `A`'s DIGIT-KEY press to its buffered per-act intent queue — the per-act
/// generic keyboard slot-binding, the mirror of the mouse
/// [`press_contextual_button`](super::press_contextual_button) (GTW-563).
///
/// Reads THIS act's button's [`VisibleSlotRank`], resolves the digit
/// [`KeyCode`](bevy::input::keyboard::KeyCode) bound to that rank via
/// [`Keybinds::contextual_slot_key`], and — on a fresh `just_pressed` with a target
/// offered — pushes the target onto [`PendingContextualIntents<A>`], the exact same
/// write-point a click uses (ADR-0001: keys and buttons share one dispatch). An unranked
/// (hidden) button, or a rank past the digit vocabulary, resolves to no key and dispatches
/// nothing — a digit beyond the visible count is a no-op. Fail-closed when the keyboard
/// buffer is absent (`Option<Res<ButtonInput<KeyCode>>>`): a headless app with no
/// `InputPlugin` never registers it, so the system simply does nothing rather than panic.
///
/// Registered per act in the [`Press`](super::super::registrar::ContextualPanelSystems)
/// set (after [`Rank`](super::super::registrar::ContextualPanelSystems), before the input
/// layer's `ContextualActSystems::Drain`), gated on the live battle + a caught-up
/// presenter exactly like the mouse press — so a digit press queued this update drains to
/// its `*Requested` this update (the Q5 same-frame guarantee).
pub(in crate::states::running::game::battlescape) fn press_contextual_button_via_key<
    A: ContextualPanelAct,
>(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    offer: Res<ContextualOffer<A>>,
    ranks: Query<&VisibleSlotRank, With<A::Marker>>,
    mut pending: ResMut<PendingContextualIntents<A>>,
) {
    let Some(keys) = keys else {
        return;
    };
    let Ok(visible_rank) = ranks.single() else {
        return;
    };
    let Some(rank) = visible_rank.rank() else {
        return;
    };
    let Some(key) = Keybinds::contextual_slot_key(rank) else {
        return;
    };
    if keys.just_pressed(key)
        && let Some(target) = offer.target()
    {
        pending.push(target);
    }
}
