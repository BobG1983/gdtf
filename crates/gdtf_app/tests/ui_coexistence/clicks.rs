//! The load-bearing coexistence proof: while BOTH stacks are alive, each button's activation
//! produces its OWN distinct observable effect and leaves the other stack's untouched.
//!
//! A green build proves nothing here — a build where egui swallowed every `bevy_ui` click (or
//! where the `bevy_ui` overlay ate the pointer before egui saw it) still compiles. These tests
//! are what discriminates.

use gdtf_test_utils::press_ui_button;

use super::harness::{
    PendingEguiClick, app_with_both_ui_stacks, bevy_ui_button, clicks as tallies,
};

/// How many frames a queued egui click needs to reach the tally: one `Update` to inject the
/// pointer events, that frame's `PostUpdate` egui pass to latch, and the NEXT `Update` to apply
/// the latch. A couple of spare frames keep the assertion off a one-frame cliff.
const EGUI_CLICK_FRAMES: usize = 4;

/// Clicks the `bevy_ui` button the way `ui_focus_system` does (an `Interaction::Pressed` edge),
/// then runs the `Update` schedule so the production press system sees it.
///
/// Only `Update` — not a whole frame: this harness has no window and no cursor, so `bevy_ui`'s
/// own `ui_focus_system` (`PreUpdate`) would clobber the injected press back to
/// `Interaction::None` before `Update` ever ran (the case `press_ui_button`'s doc calls out).
/// The system under test is the same one a real click drives.
fn click_bevy_ui_button(app: &mut bevy::app::App) -> Result<(), &'static str> {
    let button = bevy_ui_button(app).ok_or("the spike's bevy_ui button must exist in Running")?;
    press_ui_button(app, button);
    app.world_mut().run_schedule(bevy::app::Update);
    Ok(())
}

/// Queues a real egui pointer press+release on the egui button, then settles.
fn click_egui_button(app: &mut bevy::app::App) {
    app.world_mut().insert_resource(PendingEguiClick);
    for _ in 0..EGUI_CLICK_FRAMES {
        app.update();
    }
}

/// A `bevy_ui` click moves ONLY the `bevy_ui` tally — egui, alive in the same app, does not
/// swallow it and does not receive it.
#[test]
fn a_bevy_ui_click_moves_only_the_bevy_ui_tally() -> Result<(), &'static str> {
    let mut app = app_with_both_ui_stacks();
    let before = tallies(&app).ok_or("the spike's tallies must exist")?;
    assert_eq!(before.bevy_ui().get(), 0, "fresh app starts at zero");

    click_bevy_ui_button(&mut app)?;

    let after = tallies(&app).ok_or("the spike's tallies must exist")?;
    assert_eq!(
        after.bevy_ui().get(),
        1,
        "the bevy_ui click must reach the bevy_ui tally with egui alive alongside it",
    );
    assert_eq!(
        after.egui().get(),
        0,
        "a bevy_ui click must NOT register on the egui stack",
    );
    Ok(())
}

/// An egui click moves ONLY the egui tally — the `bevy_ui` overlay, alive in the same app, does
/// not swallow it and does not receive it.
#[test]
fn an_egui_click_moves_only_the_egui_tally() -> Result<(), &'static str> {
    let mut app = app_with_both_ui_stacks();

    click_egui_button(&mut app);

    let after = tallies(&app).ok_or("the spike's tallies must exist")?;
    assert_eq!(
        after.egui().get(),
        1,
        "the egui button's own clicked() branch must reach the egui tally exactly once (a \
         multipass double-count would read 2)",
    );
    assert_eq!(
        after.bevy_ui().get(),
        0,
        "an egui click must NOT register on the bevy_ui stack",
    );
    Ok(())
}

/// Both stacks, clicked in one session, each record their own activation.
///
/// The end-to-end coexistence claim: neither stack swallows the other's input, in either order.
#[test]
fn each_stack_records_its_own_click_when_both_are_used() -> Result<(), &'static str> {
    let mut app = app_with_both_ui_stacks();

    click_egui_button(&mut app);
    click_bevy_ui_button(&mut app)?;
    click_egui_button(&mut app);

    let after = tallies(&app).ok_or("the spike's tallies must exist")?;
    assert_eq!(after.bevy_ui().get(), 1, "one bevy_ui activation");
    assert_eq!(after.egui().get(), 2, "two egui activations");
    Ok(())
}
