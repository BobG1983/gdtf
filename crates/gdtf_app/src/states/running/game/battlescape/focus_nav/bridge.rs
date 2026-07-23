//! GTW-782: the app-side message bridge — the half that WRITES `gdtf_ui`'s focus-nav
//! messages from the typed [`Keybinds`] model while a battlescape panel holds focus.
//!
//! `gdtf_ui` (home of `focus_nav`) must not depend on `gdtf_battle_input`, so the typed
//! [`Keybinds`] table cannot drive [`NavigateRequest`] / [`FocusCancelled`] from inside
//! `focus_nav`. This module is the legal crossing: it lives in `gdtf_app` (which depends
//! on BOTH `gdtf_ui` and `gdtf_battle_input`), reads [`Keybinds`] +
//! [`InputFocus`], and writes the framework messages. The panel-focus CONTEXT predicate
//! ([`focused_panel_button`]) and the [`PanelNavOrder`] marker it reads live in
//! `gdtf_battle_input` (they need only bevy's `InputFocus`, not `gdtf_ui`) so the existing
//! Tab / Escape handlers there can share the same gate.

use bevy::{input_focus::InputFocus, prelude::*};
use gdtf_battle_input::{BoundKey, Keybinds, PanelNavOrder, focused_panel_button};
use gdtf_ui::focus_nav::{FocusCancelled, NavDirection, NavigateRequest};

/// Feeds the focus framework's nav / cancel messages from [`Keybinds`] WHILE a battlescape
/// panel holds focus (GTW-782) — the panel-focus half of the mutually-exclusive Tab /
/// Escape modes.
///
/// It runs only when [`focused_panel_button`] reports a panel-focus context (the same gate
/// the `gdtf_battle_input` [`cycle_selection_keys`](gdtf_battle_input::cycle_selection_keys)
/// / [`select_clear_key`](gdtf_battle_input::select_clear_key) handlers SKIP on); then:
///
/// - **Tab** ([`Keybinds::select_next`]) raises a [`NavigateRequest`] EAST (Shift+Tab — the
///   [`Keybinds::select_prev`] chord — WEST), stepping focus forward / back along the
///   [`rebuild_panel_nav_topology`](super::topology::rebuild_panel_nav_topology)-built Tab
///   chain. `Shift` is read directly as framework modifier input (the
///   [`cycle_selection_keys`](gdtf_battle_input::cycle_selection_keys) precedent).
/// - **Left / Right arrows** ([`BoundKey::KeyArrowLeft`] / [`BoundKey::KeyArrowRight`])
///   raise WEST / EAST navigate requests too — the directional mirror of Shift+Tab / Tab.
///   (Up / Down and Enter stay with the framework's own built-in keyboard bridge.)
/// - **Escape** ([`Keybinds::select_clear`]) raises a [`FocusCancelled`] — the "back out of
///   panel focus" signal [`apply_focus_cancel`] turns into an [`InputFocus::clear`].
///
/// The keyboard buffer is [`Option`] so a headless app with no `InputPlugin` is a no-op;
/// [`InputFocus`] is likewise [`Option`] so the same harness reads as "no panel focus". It
/// writes only framework [`Message`]s — never a sim intent — so it is deliberately NOT
/// playback-gated (a view control, like the level keys), staying responsive while the
/// presenter catches up.
pub(in crate::states::running::game::battlescape) fn bridge_panel_focus_nav(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    binds: Res<Keybinds>,
    focus: Option<Res<InputFocus>>,
    panels: Query<(), With<PanelNavOrder>>,
    mut navigate: MessageWriter<NavigateRequest>,
    mut cancel: MessageWriter<FocusCancelled>,
) {
    let Some(keys) = keys else {
        return;
    };
    // Only the panel-focus mode: a battlescape panel button must currently hold focus.
    if focused_panel_button(focus.as_deref(), &panels).is_none() {
        return;
    }

    // Tab drives panel focus-nav: Shift+Tab steps West (previous), plain Tab East (next).
    // `Shift` is framework modifier input read directly, not a bound act.
    if keys.just_pressed(binds.select_next()) {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        navigate.write(NavigateRequest::new(if shift {
            NavDirection::WEST
        } else {
            NavDirection::EAST
        }));
    }
    // The horizontal arrows mirror Shift+Tab / Tab. Resolved through `BoundKey::key_code`
    // so no `KeyCode` literal is named here (the fixed-vocabulary precedent).
    if keys.just_pressed(BoundKey::KeyArrowLeft.key_code()) {
        navigate.write(NavigateRequest::new(NavDirection::WEST));
    }
    if keys.just_pressed(BoundKey::KeyArrowRight.key_code()) {
        navigate.write(NavigateRequest::new(NavDirection::EAST));
    }
    // Escape backs out of panel focus.
    if keys.just_pressed(binds.select_clear()) {
        cancel.write(FocusCancelled);
    }
}

/// Applies a [`FocusCancelled`] by clearing [`InputFocus`] — the "back out of panel focus"
/// effect (GTW-782).
///
/// [`bridge_panel_focus_nav`] (Escape) raises [`FocusCancelled`] but leaves the concrete
/// effect to a consumer; in the battlescape that effect is dropping keyboard focus so the
/// map/selection context (Tab-cycles-gangers) resumes. Draining any pending cancel this
/// frame and clearing focus once is enough — [`InputFocus::clear`] is idempotent.
/// [`InputFocus`] is [`Option`] so a harness without the focus framework is a no-op.
/// Ordered `.after(bridge_panel_focus_nav)` so a cancel raised this frame is applied the
/// same frame (`bevy-traps.md` #3).
pub(in crate::states::running::game::battlescape) fn apply_focus_cancel(
    mut cancelled: MessageReader<FocusCancelled>,
    focus: Option<ResMut<InputFocus>>,
) {
    if cancelled.read().count() == 0 {
        return;
    }
    if let Some(mut focus) = focus {
        focus.clear();
    }
}
