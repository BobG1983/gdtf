//! GTW-416 headless integration test for the [`Accordion`](gdtf_ui::Accordion) widget,
//! driving the REAL widget + the REAL [`drive_accordions`](gdtf_ui::drive_accordions)
//! system on the [`GdtfUiTestAppBuilder`] `DefaultPlugins` layout harness with
//! [`gdtf_ui::UiPlugin`] added (verification rule 3).
//!
//! The discriminating assertion is C1: after ONE update the toggled row's content
//! height is STRICTLY PARTWAY between its collapsed and expanded targets — proving it
//! LERPS over frames rather than snapping. `Time` is advanced a fixed step per update
//! ([`TimeUpdateStrategy::ManualDuration`]) so the progression is deterministic with no
//! sleeps. C2 asserts the same lerp-not-snap on the collapse direction.

use core::time::Duration;

use bevy::{
    prelude::*,
    time::TimeUpdateStrategy,
    ui::{Node, Val},
};
use gdtf_test_utils::GdtfUiTestAppBuilder;
use gdtf_ui::{
    Accordion, AccordionAnim, AccordionColors, AccordionContent, AccordionHeader,
    AccordionProgress, AccordionTarget, UiPlugin, spawn_accordion, spawn_accordion_row,
};

/// Fixed per-update time step: at the widget's `4.0`/s lerp speed, one `20 ms` tick
/// advances progress `0.08` — strictly partway, the snap discriminator.
const STEP: Duration = Duration::from_millis(20);

/// Distinct (non-default) colors so the build runs with real values.
const COLORS: AccordionColors = AccordionColors {
    area:    Color::srgb(0.10, 0.10, 0.14),
    track:   Color::srgb(0.18, 0.18, 0.22),
    thumb:   Color::srgb(0.55, 0.55, 0.62),
    header:  Color::srgb(0.20, 0.22, 0.26),
    content: Color::srgb(0.12, 0.13, 0.16),
};

/// Builds the harness: the headless `DefaultPlugins` UI app + `UiPlugin` (which registers
/// `drive_accordions`), with `Time` advancing a fixed [`STEP`] per `update()`.
fn harness() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin)
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP));
    app
}

/// Spawns an accordion with one row, settles the spawn, and returns `(header, content)`.
fn spawn_one_row(app: &mut App) -> (Entity, Entity) {
    let content = {
        let mut commands = app.world_mut().commands();
        let stack = spawn_accordion(&mut commands, COLORS, ());
        spawn_accordion_row(&mut commands, stack, COLORS, (), ())
    };
    // Warm-up update: with `TimeUpdateStrategy::ManualDuration`, the FIRST `app.update()`
    // produces a zero delta (no prior reference instant) — updates AFTER it advance by the
    // manual step. This baseline makes the later press-then-tick a real, non-zero tick.
    app.update();
    let mut header = Entity::PLACEHOLDER;
    {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &AccordionTarget), With<AccordionHeader>>();
        for (entity, target) in q.iter(app.world()) {
            if target.content() == content {
                header = entity;
            }
        }
    }
    (header, content)
}

/// The content node's `Vh` height magnitude, or `None` if it is not a `Vh` value (which
/// would itself be a C3 relative-sizing failure).
fn content_height_vh(app: &App, content: Entity) -> Option<f32> {
    match app.world().get::<Node>(content)?.height {
        Val::Vh(v) => Some(v),
        _ => None,
    }
}

/// Toggles the row open/closed by setting its content [`AccordionAnim`] to the toggled
/// state, then runs ONE update so the production `drive_accordions` advances the lerp one
/// real frame.
///
/// Under this `DefaultPlugins` harness `bevy_ui`'s built-in `ui_focus_system` (`PreUpdate`)
/// REWRITES every `Interaction` from raw mouse state each frame, so a test-set
/// `Interaction::Pressed` is clobbered back to `None` before `drive_accordions` (`Update`)
/// reads it — the press edge cannot be faked without a real cursor / window (bevy-traps:
/// in-engine input is local-only evidence). So this harness exercises the LERP half of
/// `drive_accordions` directly (setting the same `AccordionAnim` a press would set); the
/// press-detection half (`Changed<Interaction> == Pressed` → `toggled()`) is covered by
/// the in-crate `MinimalPlugins` unit test, which has no `ui_focus_system` to clobber it.
fn toggle_and_tick(app: &mut App, content: Entity) {
    if let Some(mut anim) = app.world_mut().get_mut::<AccordionAnim>(content) {
        *anim = anim.toggled();
    }
    app.update();
}

/// The collapsed / expanded `Vh` endpoints, re-derived so the test is value-agnostic.
fn endpoints() -> (f32, f32) {
    (
        AccordionProgress::new(0.0).height_vh(),
        AccordionProgress::new(1.0).height_vh(),
    )
}

/// C1 — toggling a row open LERPS: after ONE update the height is strictly partway
/// (> collapsed, < expanded), proving animation not snap; further updates reach + settle
/// at the expanded target.
#[test]
fn expand_lerps_partway_then_reaches_target() {
    let mut app = harness();
    let (_header, content) = spawn_one_row(&mut app);
    let (collapsed, expanded) = endpoints();

    assert_eq!(
        content_height_vh(&app, content),
        Some(collapsed),
        "a fresh row starts collapsed",
    );

    toggle_and_tick(&mut app, content);

    let partway = content_height_vh(&app, content);
    assert!(
        partway.is_some_and(|h| h > collapsed && h < expanded),
        "after one update the height must be STRICTLY partway (lerp, not snap): {partway:?} \
         not in ({collapsed}, {expanded})",
    );

    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(expanded),
        "the lerp snaps to the exact expanded height and settles",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Expanded),
        "the settled row is Expanded",
    );
}

/// C2 — from expanded, toggling closed LERPS back: after one update strictly partway
/// (< expanded, > collapsed), then reaches + settles at the collapsed target.
#[test]
fn collapse_lerps_back_partway_then_reaches_collapsed() {
    let mut app = harness();
    let (_header, content) = spawn_one_row(&mut app);
    let (collapsed, expanded) = endpoints();

    toggle_and_tick(&mut app, content);
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(expanded),
        "row is fully expanded before the collapse toggle",
    );

    toggle_and_tick(&mut app, content);

    let partway = content_height_vh(&app, content);
    assert!(
        partway.is_some_and(|h| h < expanded && h > collapsed),
        "after one update the height must be STRICTLY partway back (lerp, not snap): {partway:?} \
         not in ({collapsed}, {expanded})",
    );

    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(collapsed),
        "the lerp snaps to the exact collapsed height and settles",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Collapsed),
        "the settled row is Collapsed",
    );
}

/// Sanity — the accordion stack root exists and a row holds a header over its targeted
/// content (the spawn wiring the lerp depends on).
#[test]
fn spawn_wires_header_to_content() {
    let mut app = harness();
    let (header, content) = spawn_one_row(&mut app);

    assert_eq!(
        app.world()
            .get::<AccordionTarget>(header)
            .map(|t| t.content()),
        Some(content),
        "header targets its content",
    );
    assert!(
        app.world().get::<AccordionContent>(content).is_some(),
        "content carries the AccordionContent marker",
    );
    let mut q = app.world_mut().query_filtered::<Entity, With<Accordion>>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "one accordion row-stack root",
    );
}
