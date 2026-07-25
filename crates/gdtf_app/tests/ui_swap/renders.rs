//! A swap changes WHICH STACK IS RENDERING, not merely which value a flag holds.
//!
//! Each assertion below is about the screen: entities that exist or do not, and a pointer
//! click that finds a widget or finds nothing. A build where the swap flipped the resource
//! and left both stacks drawing — or neither — still compiles and still passes a
//! flag-reading test; these are what discriminate.

use gdtf_app::test_support::{UiStackId, UiSwapBevyUiButton};
use gdtf_test_utils::press_ui_button;

use super::harness::{
    app_with_swap_harness, bevy_ui_panel_count, bevy_ui_swap_button, click_egui_swap_button,
    count_with, live_stack,
};

/// With `bevy_ui` live, the `bevy_ui` rendering is really on screen — the panel entity and
/// its button both exist — and the egui rendering is NOT: a real egui pointer click at the
/// egui panel's pinned position hits nothing, so the live stack does not change.
///
/// That negative half is the load-bearing one. It fails on a build that draws the egui panel
/// unconditionally (both stacks on screen at once), which is exactly the mistake a
/// resource-only toggle makes.
#[test]
fn only_the_bevy_ui_rendering_is_on_screen_while_it_is_live() {
    let mut app = app_with_swap_harness();

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "the default live stack"
    );
    assert_eq!(
        bevy_ui_panel_count(&mut app),
        1,
        "the bevy_ui rendering must be spawned while it is the live stack",
    );
    assert_eq!(
        count_with::<UiSwapBevyUiButton>(&mut app),
        1,
        "its swap button must be on screen with it",
    );

    click_egui_swap_button(&mut app);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "an egui pointer click must hit NOTHING while the egui rendering is not drawn — a \
         stack change here means both stacks are on screen at once",
    );
}

/// Swapping to egui takes the whole `bevy_ui` rendering off screen (no leaked entities) and
/// puts a real, hit-testable egui widget in its place: a pointer click at the egui panel's
/// position now swaps back.
#[test]
fn swapping_moves_the_rendering_from_one_stack_to_the_other() -> Result<(), &'static str> {
    let mut app = app_with_swap_harness();

    // Swap through the on-screen `bevy_ui` control — a real Interaction::Pressed edge.
    let button = bevy_ui_swap_button(&mut app).ok_or("the bevy_ui swap button must exist")?;
    press_ui_button(&mut app, button);
    app.world_mut().run_schedule(bevy::app::Update);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::Egui),
        "the click must swap"
    );
    assert_eq!(
        bevy_ui_panel_count(&mut app),
        0,
        "the inactive bevy_ui rendering must leak NO entities",
    );
    assert_eq!(
        count_with::<UiSwapBevyUiButton>(&mut app),
        0,
        "its children must go with it (a recursive despawn, not just the root)",
    );

    click_egui_swap_button(&mut app);

    assert_eq!(
        live_stack(&app),
        Some(UiStackId::BevyUi),
        "the egui rendering must be genuinely drawn and hit-testable once it is live — a \
         click at its pinned position swaps back",
    );
    assert_eq!(
        bevy_ui_panel_count(&mut app),
        1,
        "and the bevy_ui rendering must come back with it",
    );
    Ok(())
}
