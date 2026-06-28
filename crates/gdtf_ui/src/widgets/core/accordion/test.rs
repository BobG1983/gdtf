//! In-crate tests for the [`Accordion`](super::Accordion) lerp logic (GTW-416).
//!
//! These drive the REAL [`drive_accordions`](super::drive_accordions) system on a real
//! [`Update`] schedule with a MANUALLY advanced [`Time`] (via
//! [`TimeUpdateStrategy::ManualDuration`]), so the lerp is asserted deterministically
//! over frames without sleeps. The headless `DefaultPlugins`-layout integration tests
//! live in `gdtf_test_utils/tests/accordion_widget.rs`.

use core::time::Duration;

use bevy::{
    app::App,
    prelude::*,
    time::TimeUpdateStrategy,
    ui::{BackgroundColor, Interaction, Node, Val},
};

use super::{
    Accordion, AccordionAnim, AccordionColors, AccordionContent, AccordionContentFit,
    AccordionExpandedVh, AccordionHeader, AccordionProgress, drive_accordions, spawn_accordion,
    spawn_accordion_row,
};

/// A fixed per-update time step. Chosen so ONE update advances the lerp PARTWAY (not to
/// the end): at `ACCORDION_LERP_PER_SEC = 4.0`, one `0.02 s` tick advances progress by
/// `0.08` — strictly between 0 and 1, the discriminator a snap would fail.
const STEP: Duration = Duration::from_millis(20);

/// Distinct (non-default) colors so the build is exercised with real values.
const COLORS: AccordionColors = AccordionColors {
    area:    Color::srgb(0.10, 0.10, 0.14),
    track:   Color::srgb(0.18, 0.18, 0.22),
    thumb:   Color::srgb(0.55, 0.55, 0.62),
    header:  Color::srgb(0.20, 0.22, 0.26),
    content: Color::srgb(0.12, 0.13, 0.16),
};

/// Builds a minimal app whose `Update` schedule runs the real `drive_accordions`, with
/// `Time` advancing a fixed [`STEP`] per `update()`. No layout/render is needed — the
/// lerp logic writes `Node.height` directly, which this asserts.
fn lerp_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP))
        .add_systems(Update, drive_accordions);
    app
}

/// Spawns one accordion row and returns its `(header, content)` entities.
fn spawn_one_row(app: &mut App) -> (Entity, Entity) {
    let content = {
        let mut commands = app.world_mut().commands();
        let stack = spawn_accordion(&mut commands, COLORS, ());
        spawn_accordion_row(&mut commands, stack, COLORS, (), ())
    };
    app.world_mut().flush();
    // Warm-up update: with `TimeUpdateStrategy::ManualDuration`, the FIRST `app.update()`
    // produces a zero delta (the time system has no prior reference instant yet) — every
    // update AFTER it advances by the manual step. Running one warm-up here establishes the
    // baseline so the press-then-tick that follows is a real, non-zero advancing tick.
    app.update();
    // The header is the content's row sibling carrying `AccordionTarget`; find it.
    let mut header = Entity::PLACEHOLDER;
    {
        let mut q = app
            .world_mut()
            .query_filtered::<(Entity, &super::AccordionTarget), With<AccordionHeader>>();
        for (entity, target) in q.iter(app.world()) {
            if target.content() == content {
                header = entity;
            }
        }
    }
    (header, content)
}

/// The content node's current height as a raw `Vh` magnitude, or `None` if it is not a
/// `Vh` value (which would itself be a C3 relative-sizing failure).
fn content_height_vh(app: &App, content: Entity) -> Option<f32> {
    match app.world().get::<Node>(content)?.height {
        Val::Vh(v) => Some(v),
        _ => None,
    }
}

/// Presses the header by writing `Interaction::Pressed` (the value `ui_focus_system`
/// would write on a real click), then runs ONE update so `drive_accordions` sees the
/// `Changed<Interaction>` press edge and applies the toggle + the first lerp step.
fn press_and_tick(app: &mut App, header: Entity) {
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(header) {
        *interaction = Interaction::Pressed;
    }
    app.update();
    // Release so the NEXT press is a fresh `Changed` edge (not still-held).
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(header) {
        *interaction = Interaction::None;
    }
}

/// The collapsed / expanded `Vh` endpoints the lerp interpolates between, re-derived
/// from `AccordionProgress` so the test is value-agnostic (it asserts ORDERING /
/// partway against these, never a pinned magnitude).
fn endpoints() -> (f32, f32) {
    (
        AccordionProgress::new(0.0).height_vh(),
        AccordionProgress::new(1.0).height_vh(),
    )
}

/// C1 (the key discriminator) — toggling a row open LERPS its content height: after ONE
/// update the height is STRICTLY PARTWAY (> collapsed AND < expanded), proving it
/// animates rather than snapping; after several more updates it REACHES (snaps to
/// within epsilon of) the expanded target and SETTLES.
///
/// Pin-discriminating: a SNAP implementation would already be AT the expanded height
/// after one update, failing the strict-partway assert.
#[test]
fn expand_lerps_partway_then_reaches_target() {
    let mut app = lerp_app();
    let (header, content) = spawn_one_row(&mut app);
    let (collapsed, expanded) = endpoints();

    // Starts collapsed.
    assert_eq!(
        content_height_vh(&app, content),
        Some(collapsed),
        "a fresh row starts at the collapsed height",
    );

    // Toggle open + first lerp step.
    press_and_tick(&mut app, header);

    let partway = content_height_vh(&app, content);
    assert!(
        partway.is_some_and(|h| h > collapsed && h < expanded),
        "after one update the height must be STRICTLY partway (lerp, not snap): {partway:?} \
         not in ({collapsed}, {expanded})",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Expanding),
        "a freshly-toggled-open row is Expanding",
    );

    // A handful more ticks must reach + settle at the expanded target.
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(expanded),
        "after enough ticks the lerp snaps to the exact expanded height",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Expanded),
        "the settled row is Expanded (animation stopped)",
    );
}

/// C2 — from expanded, toggling the row closed LERPS the content height BACK: after one
/// update it is strictly partway (< expanded AND > collapsed), then after several more
/// it reaches + settles at the collapsed target.
///
/// Pin-discriminating: a snap-closed implementation would already be AT the collapsed
/// height after one update, failing the strict-partway assert.
#[test]
fn collapse_lerps_back_partway_then_reaches_collapsed() {
    let mut app = lerp_app();
    let (header, content) = spawn_one_row(&mut app);
    let (collapsed, expanded) = endpoints();

    // Open and let it fully settle.
    press_and_tick(&mut app, header);
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(expanded),
        "row is fully expanded before the collapse toggle",
    );

    // Toggle closed + first lerp step back.
    press_and_tick(&mut app, header);

    let partway = content_height_vh(&app, content);
    assert!(
        partway.is_some_and(|h| h < expanded && h > collapsed),
        "after one update the height must be STRICTLY partway back (lerp, not snap): {partway:?} \
         not in ({collapsed}, {expanded})",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Collapsing),
        "a freshly-toggled-closed row is Collapsing",
    );

    // A handful more ticks must reach + settle at the collapsed target.
    for _ in 0..40 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(collapsed),
        "after enough ticks the lerp snaps to the exact collapsed height",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Collapsed),
        "the settled row is Collapsed (animation stopped)",
    );
}

/// Structural — `spawn_accordion` builds an `Accordion` row-stack and `spawn_accordion_row`
/// builds a header (`AccordionHeader` + `Button`) over a content section
/// (`AccordionContent` + `AccordionAnim` + `AccordionProgress`), the header targeting the
/// content. Pin-discriminating: dropping any marker fails an assert.
#[test]
fn spawn_builds_header_over_targeted_content() {
    let mut app = lerp_app();
    let (header, content) = spawn_one_row(&mut app);

    let world = app.world();
    assert!(
        world.get::<AccordionHeader>(header).is_some(),
        "header carries AccordionHeader",
    );
    assert!(
        world.get::<bevy::ui::widget::Button>(header).is_some(),
        "header is a Button (so ui_focus_system drives its Interaction)",
    );
    assert_eq!(
        world
            .get::<super::AccordionTarget>(header)
            .map(|t| t.content()),
        Some(content),
        "header targets its row's content",
    );
    assert!(
        world.get::<AccordionContent>(content).is_some(),
        "content carries AccordionContent",
    );
    assert!(
        world.get::<AccordionAnim>(content).is_some(),
        "content carries its animation state",
    );
    assert!(
        world.get::<AccordionProgress>(content).is_some(),
        "content carries its lerp progress",
    );
    // Exactly one accordion stack root exists.
    let mut q = app.world_mut().query_filtered::<Entity, With<Accordion>>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "one accordion row-stack root",
    );
}

/// GTW-428 generalization — a `spawn_accordion_row` row carries the SHARED-DEFAULT
/// [`AccordionExpandedVh`] (so existing callers open to the 18vh default unchanged), and
/// `drive_accordions` lerps a content with a LARGER per-instance [`AccordionExpandedVh`] to
/// THAT taller target, not the default.
///
/// Pin-discriminating: a `drive_accordions` that ignored the per-instance target (reverting
/// to the hardcoded default) would settle the taller row at the default height, failing the
/// strict `> default` assert; a `spawn_accordion_row` that dropped the default-target seed
/// would change every existing caller's expanded height, failing the default-equality assert.
#[test]
fn per_instance_expanded_target_overrides_the_shared_default() {
    let mut app = lerp_app();
    let (header, content) = spawn_one_row(&mut app);

    // The row spawned by `spawn_accordion_row` carries the shared default target.
    assert_eq!(
        app.world().get::<AccordionExpandedVh>(content).copied(),
        Some(AccordionExpandedVh::default()),
        "a spawn_accordion_row row carries the shared-default expanded target (existing callers \
         unchanged)",
    );
    let default_expanded = AccordionProgress::new(1.0).height_vh();

    // Override THIS content with a taller per-instance target, then open + settle it.
    let taller = default_expanded + 20.0;
    if let Some(mut target) = app.world_mut().get_mut::<AccordionExpandedVh>(content) {
        *target = AccordionExpandedVh::new(taller);
    }
    press_and_tick(&mut app, header);
    for _ in 0..80 {
        app.update();
    }

    let settled = content_height_vh(&app, content);
    assert!(
        settled.is_some_and(|h| h > default_expanded),
        "the lerp must open the row to its taller PER-INSTANCE target, above the {default_expanded}vh \
         shared default (got {settled:?})",
    );
    // It reached the per-instance target exactly (the lerp snaps at the end).
    assert_eq!(
        settled,
        Some(taller),
        "the settled height must equal the per-instance expanded target",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Expanded),
        "the per-instance-target row settles Expanded like any other",
    );
}

/// Color application (AC4 root cause) — a row's HEADER node paints
/// [`AccordionColors::header`] and its CONTENT node paints
/// [`AccordionColors::content`] as their [`BackgroundColor`], so the caller-supplied
/// fills actually render (the caller-colored `scroll_list`/`dropdown` pattern, NOT the
/// `Themed`/`apply_theme` seam).
///
/// Value-agnostic: it re-reads the exact [`AccordionColors`] fields the row was spawned
/// with and asserts each node's `BackgroundColor` EQUALS the matching field — so a revert
/// to an unapplied (default-transparent) or wrong color fails the assert. Uses DISTINCT
/// header / content colors so a mix-up (header color on content, or vice versa) also fails.
#[test]
fn rows_paint_header_and_content_from_colors() {
    let mut app = lerp_app();
    let (header, content) = spawn_one_row(&mut app);

    let world = app.world();
    assert_eq!(
        world.get::<BackgroundColor>(header).map(|bg| bg.0),
        Some(COLORS.header),
        "the header node BackgroundColor equals AccordionColors::header",
    );
    assert_eq!(
        world.get::<BackgroundColor>(content).map(|bg| bg.0),
        Some(COLORS.content),
        "the content node BackgroundColor equals AccordionColors::content",
    );
    // Distinctness guard: header and content fills differ, so neither assert could pass by
    // both nodes accidentally sharing one color.
    assert_ne!(
        COLORS.header, COLORS.content,
        "the test's header/content colors are distinct (so a mix-up would fail above)",
    );
}

/// GTW-428 round-2 content-fit — a content carrying [`AccordionContentFit`] LERPS open through `Vh`
/// (a partway `Vh` mid-flight, never a snap) but, once it SETTLES fully open, adopts a
/// [`Val::Auto`] height so the rest-open section sizes to its exact content (rather than clipping to
/// a fixed `Vh` ceiling). Toggling it CLOSED leaves the `Auto` rest state and lerps back through
/// `Vh` to the collapsed height.
///
/// Pin-discriminating: a `drive_accordions` that ignored the content-fit marker would settle the
/// row at a `Val::Vh` height (the per-instance target) and fail the `is Auto` assert; one that set
/// `Auto` mid-flight (not only on settle) would fail the strict-partway `Vh` assert; one that left
/// `Auto` stuck on collapse would fail the collapsed-`Vh` assert.
#[test]
fn content_fit_settles_open_to_auto_then_lerps_back_on_collapse() {
    let mut app = lerp_app();
    let (header, content) = spawn_one_row(&mut app);
    let (collapsed, _expanded) = endpoints();

    // Opt this content into content-fit.
    app.world_mut()
        .entity_mut(content)
        .insert(AccordionContentFit);

    // Toggle open: the FIRST lerp step is a partway `Vh` (a visible animation, not a snap to Auto).
    press_and_tick(&mut app, header);
    let partway = content_height_vh(&app, content);
    assert!(
        partway.is_some_and(|h| h > collapsed),
        "while OPENING, a content-fit row's height is still a partway Vh (lerp, not an instant \
         Auto): {partway:?}",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Expanding),
        "a freshly-toggled-open content-fit row is Expanding",
    );

    // Settle fully open — the rest-open height switches to content-fit Auto.
    for _ in 0..80 {
        app.update();
    }
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Expanded),
        "the content-fit row settles Expanded",
    );
    assert!(
        matches!(
            app.world().get::<Node>(content).map(|n| n.height),
            Some(Val::Auto)
        ),
        "a SETTLED content-fit row adopts a Val::Auto content-fit height (got {:?})",
        app.world().get::<Node>(content).map(|n| n.height),
    );

    // Toggle CLOSED: it leaves Auto and lerps back through Vh to the collapsed height.
    press_and_tick(&mut app, header);
    let closing = content_height_vh(&app, content);
    assert!(
        closing.is_some(),
        "on collapse the height returns to a lerped Vh (it leaves the rest-open Auto): {:?}",
        app.world().get::<Node>(content).map(|n| n.height),
    );
    for _ in 0..80 {
        app.update();
    }
    assert_eq!(
        content_height_vh(&app, content),
        Some(collapsed),
        "the content-fit row lerps back to the exact collapsed Vh height",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(content),
        Some(&AccordionAnim::Collapsed),
        "the collapsed content-fit row settles Collapsed",
    );
}
