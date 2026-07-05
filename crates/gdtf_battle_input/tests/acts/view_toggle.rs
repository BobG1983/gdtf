//! The full-view `ViewMode` toggle through the intent seam (GTW-521).

use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_presenter::ViewMode;
use gdtf_test_utils::{clear_keys, press_key};

use super::harness::*;

// =================================================================================
// GTW-521 — the full-view toggle input seam: the ViewMode flip through the ONE
// `dispatch_act_intents` drain, and the bound key that pushes `ActIntent::ToggleFullView`.
// =================================================================================

/// The presenter [`ViewMode`], or [`ViewMode::DownToActive`] if the resource is somehow
/// absent (the harness always inserts it — this keeps the read panic-free).
fn view_mode(app: &App) -> ViewMode {
    app.world()
        .get_resource::<ViewMode>()
        .copied()
        .unwrap_or(ViewMode::DownToActive)
}

/// GTW-521 C4 — pushing [`ActIntent::ToggleFullView`] and running ONE update FLIPS the
/// presenter-owned [`ViewMode`] through the REAL `dispatch_act_intents` drain, and a second
/// toggle flips it back (the round-trip). It does NOT touch [`ActiveLevel`] (C5).
///
/// Drives the genuine intent -> drain -> presenter-resource path (the shared act-intent seam,
/// the same one the level keys / buttons use), not a direct resource write — so it proves the
/// drain arm is wired end-to-end.
#[test]
fn toggle_full_view_intent_flips_view_mode_and_round_trips() {
    let mut app = acts_app();

    // The harness seeds the default (DownToActive) and the ground floor.
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the harness seeds the default ViewMode (DownToActive)",
    );
    let active_before = active_storey(&app);

    // Push the toggle intent and drain it: DownToActive -> FullView.
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::ToggleFullView);
    app.update();
    assert_eq!(
        view_mode(&app),
        ViewMode::FullView,
        "one ToggleFullView drain must flip ViewMode DownToActive -> FullView",
    );
    // C5 — the toggle must NOT move the active storey.
    assert_eq!(
        active_storey(&app),
        active_before,
        "toggling FullView must NOT change ActiveLevel (C5)",
    );

    // Push it again: FullView -> DownToActive (the round-trip).
    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::ToggleFullView);
    app.update();
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "a second ToggleFullView drain must flip ViewMode back FullView -> DownToActive",
    );
    assert_eq!(
        active_storey(&app),
        active_before,
        "the round-trip toggle still must NOT change ActiveLevel (C5)",
    );
}

/// GTW-521 C4 — a `just_pressed` of the BOUND full-view key (`toggle_full_view`) PUSHES
/// exactly one [`ActIntent::ToggleFullView`] onto the shared [`PendingActIntent`] seam
/// (the keyboard-reader precedent — the level keys' `press_key` -> intent test).
///
/// Reads the bound key off [`Keybinds`] (NO `KeyCode` literal), presses it, and — because
/// the drain runs the SAME update — asserts the resulting `ViewMode` flip (the reader pushed
/// the intent, the drain consumed it). Then clears the key so no phantom re-press lingers.
#[test]
fn bound_full_view_key_press_pushes_toggle_full_view() {
    let mut app = acts_app();
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the harness starts in DownToActive",
    );

    // Press the DATA-DRIVEN bound key (read off Keybinds — no literal). The keyboard band
    // (`full_view_key`) pushes ToggleFullView, and `dispatch_act_intents` (ordered `.after`
    // it) drains it the same update, flipping the ViewMode.
    press_key(&mut app, test_keybinds().toggle_full_view());
    app.update();
    clear_keys(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewMode::FullView,
        "pressing the bound full-view key must push ToggleFullView (drained same update -> \
         FullView)",
    );

    // A second press of the same bound key round-trips back to DownToActive — proving the key
    // pushes the toggle each press (not a one-shot latch).
    press_key(&mut app, test_keybinds().toggle_full_view());
    app.update();
    clear_keys(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "a second bound-key press must push ToggleFullView again (drained -> DownToActive)",
    );
}
