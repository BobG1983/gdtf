//! A pip press drives the accordion expansion target (C1).

use bevy::{
    app::App,
    ecs::entity::Entity,
    ui::{Interaction, Node, Val},
};
use gdtf_app::test_support::{ExpandPip, MemberStatPanel, PipExpanded};
use gdtf_ui::{AccordionAnim, AccordionProgress};

use super::harness::*;

/// The viewport-height (`Vh`) magnitude of a [`Node`]'s `height`, or `None` if it is not a
/// `Val::Vh` — the stat panel's height is a relative `Vh` while the accordion lerp is in flight
/// (C1). Once a content-fit panel settles fully open its height becomes [`Val::Auto`], so a
/// settled member panel returns `None` here (see [`panel_height_is_auto`]).
fn panel_height_vh(app: &App, panel: Entity) -> Option<f32> {
    match app.world().get::<Node>(panel).map(|node| node.height) {
        Some(Val::Vh(vh)) => Some(vh),
        _ => None,
    }
}

/// C1: toggling the `+` pip flips `PipExpanded`, drives the matching member stat panel's
/// `AccordionAnim` toward expanding, AND the shared `drive_accordions` lerp then actually opens the
/// panel — its `AccordionProgress` and its `Node.height` both move strictly OFF zero over time
/// (the height growth is what pushes the rows below it down).
///
/// Pin: a no-op pip (no `PipExpanded` flip) fails the flag assert; a pip that flips its own state
/// but does NOT drive the panel leaves the panel `Collapsed` and fails the `AccordionAnim` assert;
/// a panel that is NOT an `AccordionContent` (so `drive_accordions` phase 2 never iterates it)
/// leaves `AccordionProgress` at `0.0` and `Node.height` at `Vh(0.0)` and fails the lerp asserts —
/// this is the half of C1 the pre-fix code regressed.
#[test]
fn pip_press_drives_the_accordion_target() {
    let mut app = editor_app();
    press_add_member(&mut app);

    let pip = control_for_row::<ExpandPip>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);
    let panel = control_for_row::<MemberStatPanel>(&mut app, 0).unwrap_or(Entity::PLACEHOLDER);

    // Precondition: a fresh row's pip is collapsed and its panel accordion is at rest collapsed,
    // at zero progress and zero height.
    assert_eq!(
        app.world().get::<PipExpanded>(pip).map(|p| **p),
        Some(false),
        "a fresh pip starts collapsed",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(panel),
        Some(&AccordionAnim::Collapsed),
        "a fresh stat panel starts at rest collapsed",
    );
    assert_eq!(
        app.world().get::<AccordionProgress>(panel).map(|p| **p),
        Some(0.0),
        "a fresh stat panel starts at zero accordion progress",
    );
    assert_eq!(
        panel_height_vh(&app, panel),
        Some(0.0),
        "a fresh stat panel starts at zero height (Vh(0.0))",
    );

    // Press the pip (the headless idiom — set Pressed then update).
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(pip) {
        *interaction = Interaction::Pressed;
    }
    app.update();

    assert_eq!(
        app.world().get::<PipExpanded>(pip).map(|p| **p),
        Some(true),
        "pressing the pip flips PipExpanded to expanded (C1)",
    );
    assert_eq!(
        app.world().get::<AccordionAnim>(panel),
        Some(&AccordionAnim::Expanding),
        "pressing the pip must drive the matching stat panel's accordion toward Expanding (C1)",
    );

    // Advance the lerp (the harness `FixedTimesteps(1)` steps `Time` a fixed delta per update, so a
    // few updates move the height-lerp measurably). The panel must actually OPEN — both the
    // progress parameter and the written `Node.height` move strictly off zero.
    for _ in 0..6 {
        app.update();
    }

    let progress = app
        .world()
        .get::<AccordionProgress>(panel)
        .map_or(0.0, |p| **p);
    assert!(
        progress > 0.0,
        "the shared drive_accordions lerp must advance the panel's AccordionProgress off zero \
         (C1: the panel must be an AccordionContent the height lerp iterates), got {progress}",
    );
    let height = panel_height_vh(&app, panel).unwrap_or(0.0);
    assert!(
        height > 0.0,
        "the lerp must write a growing Node.height onto the panel — the height growth is what \
         pushes the rows below it down (C1), got Vh({height})",
    );
}
