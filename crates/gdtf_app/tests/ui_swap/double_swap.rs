//! The multipass double-swap guard: one click on the EGUI swap button must produce exactly
//! ONE swap.
//!
//! egui's multipass runs the panel closure up to twice per frame with the same pointer
//! input (`bevy-traps.md` #8b). A swap performed inside that closure would therefore run
//! twice for one click — flipping to the other stack and straight back, leaving the live
//! stack exactly where it started, so the swap button would appear dead. This file is the
//! test that catches precisely that: it asserts the stack CHANGED after one egui click, and
//! a double fire lands it back on the starting value and fails the assertion.

use gdtf_app::test_support::UiStackId;

use super::harness::{
    app_with_swap_harness, bevy_ui_panel_count, click_egui_swap_button, live_stack, tap_swap_key,
};

/// One click on the egui swap button swaps exactly once.
///
/// Reaching the egui stack via the keyboard first (a single, already-proven swap), so the
/// egui rendering is the one on screen and the click under test is genuinely an egui-side
/// click. A double fire would return the live stack to `Egui`; a zero fire would leave it
/// there too — so the assertion pins the one correct count from both sides.
#[test]
fn one_egui_click_swaps_exactly_once() {
    let mut app = app_with_swap_harness();
    tap_swap_key(&mut app);
    assert_eq!(
        live_stack(&app),
        Some(UiStackId::Egui),
        "precondition: the egui rendering is the live one",
    );

    click_egui_swap_button(&mut app);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "one egui click must swap exactly ONCE — still Egui here means the multipass ran \
         the swap twice (or the click never landed)",
    );
    assert_eq!(
        bevy_ui_panel_count(&mut app),
        1,
        "and the single swap must be reflected on screen",
    );
}

/// Two egui clicks make two swaps — the counterpart pin, so "exactly once" cannot be
/// satisfied by a latch that swallows every swap after the first.
#[test]
fn two_egui_clicks_swap_twice() {
    let mut app = app_with_swap_harness();
    tap_swap_key(&mut app);

    click_egui_swap_button(&mut app);
    assert_eq!(live_stack(&app), Some(UiStackId::BevyUi));

    // Back to egui so the second click has an egui widget to hit.
    tap_swap_key(&mut app);
    click_egui_swap_button(&mut app);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "a second egui click must swap again — the latch collapses swaps WITHIN a frame, \
         never across frames",
    );
}
