//! The keyboard shortcut flips the live stack in play, through the real windowing-input
//! path — the developer-facing half of the harness.

use gdtf_app::test_support::UiStackId;

use super::harness::{
    AN_UNRELATED_KEY, app_with_swap_harness, bevy_ui_panel_count, live_stack, tap_key, tap_swap_key,
};

/// Tapping the shipped shortcut flips the live stack, and tapping it again flips back —
/// with the on-screen rendering following each time.
///
/// Drives a real `KeyboardInput` press+release pair, so Bevy's own `keyboard_input_system`
/// is what produces the `just_pressed` the production system reads. Nothing is stubbed and
/// no system is called directly.
#[test]
fn the_shortcut_flips_the_live_stack_and_flips_it_back() {
    let mut app = app_with_swap_harness();
    assert_eq!(live_stack(&app), Some(UiStackId::BevyUi));

    tap_swap_key(&mut app);
    assert_eq!(
        live_stack(&app),
        Some(UiStackId::Egui),
        "the shortcut must make the other stack live",
    );
    assert_eq!(
        bevy_ui_panel_count(&mut app),
        0,
        "and the screen must follow the swap, not just the flag",
    );

    tap_swap_key(&mut app);
    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "a second tap must come back — a toggle, not a one-way trip",
    );
    assert_eq!(bevy_ui_panel_count(&mut app), 1);
}

/// How many frames the egui panel is left drawing before the key is tapped — enough that
/// the egui pass has run several times and `EguiWantsInput` has been updated from frames
/// where the egui panel was genuinely on screen.
const DRAWN_FRAMES: usize = 4;

/// The shortcut still swaps while the EGUI panel is the one on screen and really drawing.
///
/// The `not(egui_holds_the_keyboard)` guard exists so a keystroke that belongs to a focused
/// egui widget is not stolen for a swap. A guard that instead read "egui is drawing at all"
/// would deaden the shortcut exactly when the candidate stack is live — leaving a developer
/// stuck on the stack they just swapped to. This is the pin against that reading: the key is
/// tapped after the egui panel has been drawn for several frames, with the egui plugin, its
/// context and its input state all live.
#[test]
fn the_shortcut_still_works_while_the_egui_panel_is_drawing() {
    let mut app = app_with_swap_harness();
    tap_swap_key(&mut app);
    assert_eq!(
        live_stack(&app),
        Some(UiStackId::Egui),
        "precondition: the egui rendering is the live one",
    );
    for _ in 0..DRAWN_FRAMES {
        app.update();
    }

    tap_swap_key(&mut app);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "the shortcut must keep working while egui draws — egui merely being on screen is \
         not egui holding the keyboard",
    );
    assert_eq!(
        bevy_ui_panel_count(&mut app),
        1,
        "and the rendering must follow it back",
    );
}

/// Only the bound key swaps: an unrelated keystroke leaves the live stack alone.
///
/// Discriminating against a shortcut wired to "any key pressed", and against a `just_pressed`
/// read that was actually a `pressed` read (which would keep firing while held).
#[test]
fn an_unrelated_key_does_not_swap() {
    let mut app = app_with_swap_harness();

    tap_key(&mut app, AN_UNRELATED_KEY);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "a key that is not the shortcut must not swap the UI stack",
    );
}
