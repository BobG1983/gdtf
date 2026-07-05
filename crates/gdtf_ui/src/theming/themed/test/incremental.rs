//! GTW-284 incremental-repaint tests under the full production run condition:
//! a new spawn paints only itself; a real theme change still repaints all.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, Node, widget::Button},
};

use super::{
    super::{ThemeRole, Themed},
    support::{app_with_production_run_condition, theme},
};

/// GTW-284 Test A — on a STEADY theme frame, spawning a new `Themed` widget must
/// paint ONLY the new entity (its initial base look) and must NOT recolor an
/// existing already-painted widget that holds a non-base SENTINEL fill.
///
/// This pins the incremental-repaint fix: the OLD body had a single UNFILTERED
/// query, so any `apply_theme` run (here triggered by the new entity's
/// `any_themed_added`) repainted EVERY `Themed` entity back to the resting base —
/// clobbering the existing button's hover / `ActiveButton` fill for a frame (the
/// sentinel stands in for that interaction fill). Pin-discriminating: it is RED on
/// the old global-repaint body (E reverts to base) and GREEN after.
#[test]
fn incremental_repaint_does_not_recolor_existing_on_new_spawn()
-> Result<(), ron::error::SpannedError> {
    let mut app = app_with_production_run_condition();
    // The base button fill the theme resolves to (the first `theme(..)` arg).
    let base = Color::srgb(0.12, 0.12, 0.15);
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    // Spawn the EXISTING button E and let it settle: first update paints it (theme
    // is_changed on the insert frame → full arm), second update clears `Added` and
    // reaches the steady state (no theme change, no added → `apply_theme` skipped).
    let e = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();
    app.update();
    app.update();

    // Set E's fill to a SENTINEL ≠ base — standing in for a live hover / active fill
    // that the GTW-118 / GTW-253 interaction layer composed on top of the base.
    let sentinel = Color::srgb(0.80, 0.16, 0.19);
    if let Some(mut bg) = app.world_mut().get_mut::<BackgroundColor>(e) {
        bg.0 = sentinel;
    }

    // Now spawn a NEW button F on a steady theme frame. Its `Added<Themed>` makes
    // `apply_theme` run (the production run condition), but the theme did NOT change.
    let f = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();
    app.update();

    let world = app.world();
    // F is the freshly-added entity → it gets its initial base paint.
    assert_eq!(
        world.get::<BackgroundColor>(f).map(|c| c.0),
        Some(base),
        "a newly-spawned Themed widget must get its initial base paint",
    );
    // E is unchanged this frame → the incremental pass must NOT touch it, so its
    // sentinel (interaction) fill survives the unrelated spawn.
    assert_eq!(
        world.get::<BackgroundColor>(e).map(|c| c.0),
        Some(sentinel),
        "an existing widget's fill must NOT be recolored to base by an unrelated Themed spawn",
    );

    Ok(())
}

/// GTW-284 Test B (over-narrowing guard) — a REAL theme change (overwriting the
/// `GdtfTheme` resource with a new palette, no new `Themed` entities) must still
/// repaint EVERY existing widget to the NEW base.
///
/// This guards the incremental fix from over-narrowing into "only ever paint
/// added/changed entities": the `is_changed()` arm must fan the new palette out to
/// the FULL `Themed` set so the GTW-137 hot-reload retheme cannot regress.
/// Pin-discriminating: a body that dropped the full-set theme-change arm would
/// leave E on the OLD palette and fail this.
#[test]
fn theme_change_repaints_all_themed() -> Result<(), ron::error::SpannedError> {
    let mut app = app_with_production_run_condition();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    // Spawn E and settle it on the original palette (paint, then steady).
    let e = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();
    app.update();
    app.update();

    // Overwrite the resource with a NEW palette — no new Themed entities.
    let new_base = Color::srgb(0.50, 0.10, 0.30);
    app.insert_resource(theme(
        [0.50, 0.10, 0.30],
        [0.99, 0.40, 0.00],
        4.0,
        9.0,
        30.0,
    )?);
    app.update();

    // The theme-change arm must fan the new palette out to the whole `Themed` set.
    assert_eq!(
        app.world().get::<BackgroundColor>(e).map(|c| c.0),
        Some(new_base),
        "a real theme change must repaint EVERY existing Themed widget to the new base",
    );

    Ok(())
}
