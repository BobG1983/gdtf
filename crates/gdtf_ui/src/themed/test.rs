//! Tests for the central [`apply_theme`](super::apply_theme) base-look pass.

use bevy::{
    prelude::*,
    text::{TextColor as UiTextColor, TextFont},
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
                  border_width_px: 3.0, corner_radius_px: 7.0, \
                  margin: (left: 9.0, right: 9.0, top: 4.0, bottom: 4.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, 1.0), disabled: (0.08, 0.08, 0.10, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: (0.80, 0.16, 0.19, 0.96), pressed: (0.10, 0.10, 0.12, 0.96), \
                   text_color: (0.84, 0.80, 0.73, 1.0), font_size_pt: {button_font_size}, \
                   border_color: ({br}, {bg}, {bb}, 1.0), \
                   border_width_px: {border_w}, corner_radius_px: {radius}, \
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
        .spawn((Themed(ThemeRole::ButtonText), Text::new("x")))
        .id();
    let button = app
        .world_mut()
        .spawn((Themed(ThemeRole::Button), Node::default()))
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
        Some(Handle::<Font>::default()),
        "button-text entity should get the button font handle",
    );
    assert!(
        world
            .get::<TextFont>(text)
            .is_some_and(|f| (f.font_size - 18.0).abs() < f32::EPSILON),
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
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border.left),
        Some(Val::Px(2.0)),
        "button border width",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.padding.left),
        Some(Val::Px(8.0)),
        "left content padding",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.padding.top),
        Some(Val::Px(6.0)),
        "top content padding",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border_radius.top_left),
        Some(Val::Px(5.0)),
        "corner radius",
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
        .spawn((Themed(ThemeRole::Background), Node::default()))
        .id();
    let panel = app
        .world_mut()
        .spawn((Themed(ThemeRole::Panel), Node::default()))
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
    // node's border at the default (Px(0.0)), never the panel/button themed
    // border width (which would be Px(3.0)/Px(2.0)).
    assert_eq!(
        world.get::<Node>(background).map(|n| n.border.left),
        Some(Val::Px(0.0)),
        "background node must keep its default border, not get a themed border width",
    );

    // Panel gets the panel sub-theme box (border 3.0, radius 7.0, padding 9.0).
    assert_eq!(
        world.get::<BackgroundColor>(panel).map(|c| c.0),
        Some(Color::srgba(0.16, 0.16, 0.18, 0.55)),
        "panel entity should get the panel fill",
    );
    assert_eq!(
        world.get::<Node>(panel).map(|n| n.border.left),
        Some(Val::Px(3.0)),
        "panel border width from the panel sub-theme",
    );
    assert_eq!(
        world.get::<Node>(panel).map(|n| n.padding.left),
        Some(Val::Px(9.0)),
        "panel padding from the panel sub-theme",
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
        .spawn((Themed(ThemeRole::Title), Text::new("GRIMDARK TURFWAR")))
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
        Some(Handle::<Font>::default()),
        "title must use the title font handle",
    );
    // The title sub-theme size is 36.0 — its own value, not 18 * a scale —
    // and strictly larger than the resolved text sub-theme's body size (18).
    let title_size = world.get::<TextFont>(title).map(|f| f.font_size);
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
        .spawn((Themed(ThemeRole::ButtonText), Text::new("x")))
        .id();
    let button = app
        .world_mut()
        .spawn((Themed(ThemeRole::Button), Node::default()))
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
            .is_some_and(|f| (f.font_size - 30.0).abs() < f32::EPSILON),
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
        Some(Val::Px(4.0)),
        "border width must reflect the NEW theme after re-run",
    );
    assert_eq!(
        world.get::<Node>(button).map(|n| n.border_radius.top_left),
        Some(Val::Px(9.0)),
        "corner radius must reflect the NEW theme after re-run",
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
        .spawn((Themed(ThemeRole::Panel), Node::default()));

    // No GdtfTheme inserted. The guard must keep apply_theme from running.
    app.update();

    // Reaching here without a panic is the assertion; make it explicit.
    assert!(
        app.world().get_resource::<GdtfTheme>().is_none(),
        "test precondition: GdtfTheme must be absent for this guard check",
    );
}
