//! Tests for the central [`apply_theme`](super::apply_theme) base-look pass.

use bevy::{
    prelude::*,
    text::{FontSize, FontSource, TextColor as UiTextColor, TextFont},
    ui::{BackgroundColor, BorderColor as UiBorderColor, Node, Val},
};

use super::{ThemeRole, Themed, apply_theme};
use crate::theme::{GdtfTheme, GdtfThemeSpec};

/// Builds a [`GdtfTheme`] from the shipped-shape nested RON with caller-chosen
/// salient values, through the real production path (deserialize a spec, then
/// [`GdtfThemeSpec::resolve`] with a defaulted-font resolver). The theme
/// newtypes have private fields, so this respects encapsulation while
/// exercising the genuine resolution. Returns the `ron` error so a malformed
/// literal surfaces via `?` rather than a denied `unwrap`/`panic`.
fn theme(
    button_color: [f32; 3],
    border: [f32; 3],
    border_w: f32,
    radius: f32,
    button_font_size: f32,
) -> Result<GdtfTheme, ron::error::SpannedError> {
    let [pr, pg, pb] = button_color;
    let [br, bg, bb] = border;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: (0.16, 0.16, 0.18, 0.55), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 3.0, corner_radius: 7.0, \
                  margin: (left: 9.0, right: 9.0, top: 4.0, bottom: 4.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, 1.0), disabled: (0.08, 0.08, 0.10, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
                   text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: {button_font_size}, \
                   border_color: ({br}, {bg}, {bb}, 1.0), \
                   border_width: {border_w}, corner_radius: {radius}, \
                   margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         title: ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 36.0 ), \
         text:  ( text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: 18.0 ))",
    );
    let spec: GdtfThemeSpec = ron::from_str(&ron)?;
    Ok(spec.resolve(|_| Handle::<Font>::default()))
}

/// A minimal app with `apply_theme` registered under the real run condition,
/// so the test drives the actual production schedule path.
fn app_with_apply_theme() -> App {
    let mut app = App::new();
    app.add_systems(Update, apply_theme.run_if(resource_exists::<GdtfTheme>));
    app
}

/// A minimal app with `apply_theme` registered under the FULL production run
/// condition (GTW-144 / GTW-284):
/// `resource_exists::<GdtfTheme>().and_then(resource_changed::<GdtfTheme>.or_else(any_themed_added))`,
/// the exact gate [`UiPlugin`](crate::UiPlugin) installs.
///
/// The incremental-repaint tests need this real gate (not the bare
/// `resource_exists`) so that — on a steady theme frame — a NEWLY-spawned `Themed`
/// entity still triggers `apply_theme` via the `any_themed_added` arm, exactly as
/// in the running app. Mirrors the `gdtf_ui` test idiom (no `gdtf_test_utils` dep).
fn app_with_production_run_condition() -> App {
    let mut app = App::new();
    app.add_systems(
        Update,
        apply_theme.run_if(
            resource_exists::<GdtfTheme>
                .and_then(resource_changed::<GdtfTheme>.or_else(super::any_themed_added)),
        ),
    );
    app
}

/// `apply_theme` writes the button text color / font / size onto a
/// `Themed(ButtonText)` entity, and the button fill / border color / border
/// width / corner radius / content padding onto a `Themed(Button)` entity.
///
/// Pin-discriminating: reverting any field's mapping (or dropping a written
/// component) fails the corresponding assert.
#[test]
fn apply_theme_paints_role_appropriate_visuals_from_the_resource()
-> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let text = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::ButtonText), Text::new("x")))
        .id();
    let button = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Node::default()))
        .id();

    app.update();

    let world = app.world();

    assert_eq!(
        world.get::<UiTextColor>(text).map(|c| c.0),
        Some(Color::srgb(0.84, 0.80, 0.73)),
        "button-text entity should get the button text color",
    );
    assert_eq!(
        world.get::<TextFont>(text).map(|f| f.font.clone()),
        Some(FontSource::from(Handle::<Font>::default())),
        "button-text entity should get the button font handle",
    );
    assert!(
        world
            .get::<TextFont>(text)
            .is_some_and(|f| f.font_size == FontSize::Px(18.0)),
        "button-text entity font size should be the button size",
    );

    assert_eq!(
        world.get::<BackgroundColor>(button).map(|c| c.0),
        Some(Color::srgb(0.12, 0.12, 0.15)),
        "button entity should get the button fill",
    );
    assert_eq!(
        world.get::<UiBorderColor>(button).map(|b| b.top),
        Some(Color::srgb(0.20, 0.20, 0.24)),
        "button entity should get the button border color on every edge",
    );
    // GTW-296: the themed box now emits RELATIVE units — Vw for border + radius +
    // horizontal padding, Vh for vertical padding (asserting the unit KIND, never a px
    // magnitude). The numeric values are the test theme's own resolved fractions.
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border.left),
        Some(Val::Vw(2.0)),
        "button border width (Vw)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.padding.left),
        Some(Val::Vw(8.0)),
        "left content padding (Vw)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.padding.top),
        Some(Val::Vh(6.0)),
        "top content padding (Vh)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border_radius.top_left),
        Some(Val::Vw(5.0)),
        "corner radius (Vw)",
    );

    Ok(())
}

/// `apply_theme` paints a `Themed(Background)` node with only the backdrop
/// fill (no border / radius / padding), and a `Themed(Panel)` node with the
/// panel sub-theme's box visuals.
#[test]
fn apply_theme_paints_background_and_panel() -> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let background = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Background), Node::default()))
        .id();
    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();

    app.update();

    let world = app.world();
    // Background gets the backdrop fill and NO border (default 0 width).
    assert_eq!(
        world.get::<BackgroundColor>(background).map(|c| c.0),
        Some(Color::srgb(0.05, 0.05, 0.06)),
        "background entity should get the backdrop fill",
    );
    // The Background arm writes only BackgroundColor — it leaves the spawned
    // node's border at the default (a zero-anchored Px(0.0), the Node default),
    // never the panel/button themed border width (which would be the Vw(3.0)/Vw(2.0)
    // relative-length border the box arms emit).
    assert_eq!(
        world.get::<Node>(background).map(|n| n.border.left),
        Some(Val::Px(0.0)),
        "background node must keep its default border, not get a themed border width",
    );

    // Panel gets the panel sub-theme box — GTW-296: border + radius in Vw, padding in
    // Vw (horizontal) / Vh (vertical), asserting the unit KIND not a px magnitude.
    assert_eq!(
        world.get::<BackgroundColor>(panel).map(|c| c.0),
        Some(Color::srgba(0.16, 0.16, 0.18, 0.55)),
        "panel entity should get the panel fill",
    );
    assert_eq!(
        world.get::<Node>(panel).map(|n| n.border.left),
        Some(Val::Vw(3.0)),
        "panel border width from the panel sub-theme (Vw)",
    );
    assert_eq!(
        world.get::<Node>(panel).map(|n| n.padding.left),
        Some(Val::Vw(9.0)),
        "panel padding from the panel sub-theme (Vw)",
    );

    Ok(())
}

/// `apply_theme` paints a `Themed(Title)` node with the title sub-theme's
/// color, font handle, and its **own** `font_size_pt` (36) — distinct from the
/// button/body size, and no longer a scaled body size.
///
/// Pin-discriminating: if the Title arm used the button/body size, or a
/// hardcoded scale, the size assert fails; if it stopped sourcing color/font
/// from the title sub-theme, those asserts fail.
#[test]
fn apply_theme_uses_title_sub_theme_font_size() -> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    let theme_res = theme([0.12, 0.12, 0.15], [0.20, 0.20, 0.24], 2.0, 5.0, 18.0)?;
    app.insert_resource(theme_res.clone());

    let title = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Title), Text::new("GRIMDARK TURFWAR")))
        .id();

    app.update();

    let world = app.world();
    assert_eq!(
        world.get::<UiTextColor>(title).map(|c| c.0),
        Some(Color::srgb(0.84, 0.80, 0.73)),
        "title must use the title text color",
    );
    assert_eq!(
        world.get::<TextFont>(title).map(|f| f.font.clone()),
        Some(FontSource::from(Handle::<Font>::default())),
        "title must use the title font handle",
    );
    // The title sub-theme size is 36.0 — its own value, not 18 * a scale —
    // and strictly larger than the resolved text sub-theme's body size (18).
    // `FontSize` is now an enum (Bevy 0.19); pull the logical-pixel f32 back out of
    // the `Px` variant so the size assertions stay numeric (and skip non-`Px` units).
    let title_size = world
        .get::<TextFont>(title)
        .and_then(|f| match f.font_size {
            FontSize::Px(px) => Some(px),
            _ => None,
        });
    assert!(
        title_size.is_some_and(|s| (s - 36.0).abs() < f32::EPSILON),
        "title font size must be the title sub-theme's own font_size_pt (36)",
    );
    let body_size = *theme_res.text.font_size_pt;
    assert!(
        title_size.is_some_and(|s| s > body_size),
        "title must be strictly larger than the body/button text size",
    );

    Ok(())
}

/// Mutating the in-memory `GdtfTheme` and re-running `apply_theme` re-themes
/// the already-spawned entities with the NEW palette — proving the system
/// reads the resource live each run, never snapshots at spawn.
///
/// Pin-discriminating: if `apply_theme` captured values at spawn instead of
/// reading `Res<GdtfTheme>` per run, the colors after the second update would
/// still be the first palette and these asserts would fail.
#[test]
fn re_theme_after_resource_mutation_repaints_with_new_palette()
-> Result<(), ron::error::SpannedError> {
    let mut app = app_with_apply_theme();
    app.insert_resource(theme(
        [0.12, 0.12, 0.15],
        [0.20, 0.20, 0.24],
        2.0,
        5.0,
        18.0,
    )?);

    let text = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::ButtonText), Text::new("x")))
        .id();
    let button = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Node::default()))
        .id();

    app.update();

    // Swap to a deliberately different button palette and scalars.
    let new_button = Color::srgb(0.50, 0.10, 0.30);
    let new_border = Color::srgb(0.99, 0.40, 0.00);
    app.insert_resource(theme(
        [0.50, 0.10, 0.30],
        [0.99, 0.40, 0.00],
        4.0,
        9.0,
        30.0,
    )?);

    app.update();

    let world = app.world();
    assert!(
        world
            .get::<TextFont>(text)
            .is_some_and(|f| f.font_size == FontSize::Px(30.0)),
        "button-text size must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<BackgroundColor>(button).map(|c| c.0),
        Some(new_button),
        "button fill must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<UiBorderColor>(button).map(|b| b.top),
        Some(new_border),
        "border color must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border.left),
        Some(Val::Vw(4.0)),
        "border width must reflect the NEW theme after re-run (Vw)",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border_radius.top_left),
        Some(Val::Vw(9.0)),
        "corner radius must reflect the NEW theme after re-run (Vw)",
    );

    Ok(())
}

/// With NO `GdtfTheme` resource inserted, an `app.update()` must not panic:
/// the `run_if(resource_exists)` guard keeps `apply_theme` from running, so
/// the absent theme (pre-`Load`) is safe (bevy-traps rule 1).
///
/// Pin-discriminating: dropping the run condition (or taking `Res<GdtfTheme>`
/// without the guard) would make this update panic on the missing resource.
#[test]
fn absent_theme_does_not_panic() {
    let mut app = app_with_apply_theme();
    app.world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()));

    // No GdtfTheme inserted. The guard must keep apply_theme from running.
    app.update();

    // Reaching here without a panic is the assertion; make it explicit.
    assert!(
        app.world().get_resource::<GdtfTheme>().is_none(),
        "test precondition: GdtfTheme must be absent for this guard check",
    );
}

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
