//! Tests for the live-retheme system.

use bevy::{
    MinimalPlugins,
    asset::{AssetApp, AssetEvent, AssetPlugin, AssetServer, Assets, Handle},
    input::InputPlugin,
    prelude::*,
    text::Font,
    ui::{BackgroundColor, Interaction, Node, widget::Button},
};
use gdtf_assets::{RonAsset, RonAssetAppExt};

use crate::{
    UiPlugin,
    theme::{ActiveThemeHandle, GdtfTheme, GdtfThemeSpec},
    themed::{ThemeRole, Themed},
};

/// Builds a nested [`GdtfThemeSpec`] from caller-chosen body-text + panel
/// colors plus a button hover color (the button resting fill tracks the panel
/// color so the existing `Themed(Panel)` repaint asserts still read it),
/// mirroring the shipped `grimdark.ron` shape. Returns the `ron` error so a
/// malformed literal surfaces via `?` rather than a denied `unwrap`/`panic`.
fn spec(
    text: [f32; 3],
    panel: [f32; 3],
    hover: [f32; 3],
) -> Result<GdtfThemeSpec, ron::error::SpannedError> {
    let [tr, tg, tb] = text;
    let [pr, pg, pb] = panel;
    let [hr, hg, hb] = hover;
    let ron = format!(
        "(\
         default_font: \"fonts/test.ttf\", \
         background: ( color: (0.05, 0.05, 0.06, 1.0) ), \
         panel: ( color: ({pr}, {pg}, {pb}, 1.0), border_color: (0.20, 0.20, 0.24, 1.0), \
                  border_width: 1.0, corner_radius: 2.0, \
                  margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         button: ( color: ({pr}, {pg}, {pb}, 1.0), disabled: (0.16, 0.16, 0.18, 0.55), \
                   active: (0.45, 0.62, 0.30, 0.96), \
                   hover: ({hr}, {hg}, {hb}, 1.0), pressed: ({pr}, {pg}, {pb}, 1.0), \
                   text_color: ({tr}, {tg}, {tb}, 1.0), font_size_pt: 18.0, \
                   border_color: (0.20, 0.20, 0.24, 1.0), \
                   border_width: 1.0, corner_radius: 2.0, \
                   margin: (left: 8.0, right: 8.0, top: 6.0, bottom: 6.0) ), \
         title: ( text_color: ({tr}, {tg}, {tb}, 1.0), font_size_pt: 36.0 ), \
         text:  ( text_color: ({tr}, {tg}, {tb}, 1.0), font_size_pt: 18.0 ))",
    );
    ron::from_str(&ron)
}

/// A headless app with the real production wiring: `MinimalPlugins`, then
/// `AssetPlugin` (so the `Assets` collection and `AssetEvent` messages exist),
/// the theme RON asset registered, and `UiPlugin` (which registers the
/// re-derive system before the `ApplyTheme` set and the change-driven
/// `apply_theme`). `InputPlugin` is added because `UiPlugin`'s focus-nav bridge
/// reads the `ButtonInput<KeyCode>` resource / keyboard message buffers that
/// `InputPlugin` registers. (As of Bevy 0.19 the `InputDispatchPlugin` that owns
/// `InputFocus` ships in `DefaultPlugins`, not in `UiPlugin`; this headless app
/// uses `MinimalPlugins`, so it never gets that plugin — only the focus-nav
/// bridge + `DirectionalNavigationPlugin` that `UiPlugin` adds. See the
/// `interaction` tests / bevy-traps rule 1.)
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset::<GdtfThemeSpec>()
        // `Assets<Font>` is registered by `bevy_text` in the real app; the
        // font-handle-persistence test needs it to mint a distinctive handle.
        .init_asset::<Font>()
        .add_plugins(UiPlugin);
    app
}

/// Adds a `RonAsset<GdtfThemeSpec>` to the collection and returns its handle.
fn add_theme_asset(app: &mut App, spec: GdtfThemeSpec) -> Handle<RonAsset<GdtfThemeSpec>> {
    app.world_mut()
        .resource_mut::<Assets<RonAsset<GdtfThemeSpec>>>()
        .add(RonAsset(spec))
}

/// Overwrites the in-memory spec of an already-added theme asset (the hot edit
/// the file-watcher would make).
fn hot_edit(app: &mut App, handle: &Handle<RonAsset<GdtfThemeSpec>>, spec: GdtfThemeSpec) {
    let mut assets = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<GdtfThemeSpec>>>();
    // `Assets::get_mut` now hands back an `AssetMut` guard (Bevy 0.19) that
    // `DerefMut`s to the asset, so the binding must be `mut` to write through it.
    if let Some(mut asset) = assets.get_mut(handle) {
        asset.0 = spec;
    }
}

/// Injects an `AssetEvent::Modified` for the given handle id into the message
/// buffer (standing in for the file-watcher's reload signal).
fn inject_modified(app: &mut App, handle: &Handle<RonAsset<GdtfThemeSpec>>) {
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
}

/// On a matching `Modified` for the active theme handle, the re-derive system
/// overwrites `GdtfTheme` from the UPDATED in-memory spec AND the
/// change-driven `apply_theme` repaints spawned `Themed` entities the same
/// frame — AC1/AC2/AC3 + the GTW-144 repaint-on-change.
///
/// Pin-discriminating: dropping the re-derive (or the before-ApplyTheme
/// ordering) leaves `GdtfTheme` / the widgets on the OLD palette.
#[test]
fn modified_event_rederives_theme_and_repaints_widgets() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let handle = add_theme_asset(
        &mut app,
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
    );
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.world_mut()
        .insert_resource(ActiveThemeHandle::new(handle.clone()));

    let text = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Text), Text::new("x")))
        .id();
    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();

    // First update paints the OLD base look (the Added<Themed> path).
    app.update();

    // The asset is hot-edited to a NEW palette, then a Modified fires.
    let new_text = Color::srgb(0.10, 0.90, 0.20);
    let new_panel = Color::srgb(0.50, 0.10, 0.30);
    hot_edit(
        &mut app,
        &handle,
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?,
    );
    inject_modified(&mut app, &handle);

    app.update();

    let world = app.world();
    // (a) GdtfTheme re-derived to the NEW values.
    assert_eq!(
        world
            .get_resource::<GdtfTheme>()
            .map(|t| *t.text.text_color),
        Some(new_text),
        "GdtfTheme.text.text_color must be re-derived to the NEW palette",
    );
    // (b) the spawned Themed text + panel repainted to the NEW theme.
    assert_eq!(
        world.get::<bevy::text::TextColor>(text).map(|c| c.0),
        Some(new_text),
        "Themed(Text) must repaint to the NEW text color the same frame",
    );
    assert_eq!(
        world.get::<BackgroundColor>(panel).map(|c| c.0),
        Some(new_panel),
        "Themed(Panel) must repaint to the NEW panel bg the same frame",
    );

    Ok(())
}

/// A `Modified` event for a DIFFERENT asset id leaves `GdtfTheme` untouched —
/// the filter is on the ACTIVE handle id only (AC1: ignore other handles).
///
/// Pin-discriminating: dropping the id filter would re-derive on any theme
/// asset's event and this assert (theme unchanged) fails.
#[test]
fn modified_event_for_other_id_does_not_rederive() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let active = add_theme_asset(
        &mut app,
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
    );
    // A second, unrelated theme asset whose value differs from the active one.
    let other = add_theme_asset(
        &mut app,
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?,
    );
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.world_mut()
        .insert_resource(ActiveThemeHandle::new(active));

    app.update();
    let before = app.world().get_resource::<GdtfTheme>().cloned();

    // Fire a Modified for the OTHER (non-active) id only.
    inject_modified(&mut app, &other);
    app.update();

    assert_eq!(
        app.world().get_resource::<GdtfTheme>().cloned(),
        before,
        "a Modified for a non-active asset id must NOT re-derive GdtfTheme",
    );

    Ok(())
}

/// The re-derive re-resolves fonts through the `AssetServer` — `load` is
/// idempotent, so a sub-theme's resolved font handle after the re-derive equals
/// the handle the server hands back for that key (and is NOT the default handle,
/// proving fonts were threaded through resolution, not reset) — GTW-149.
///
/// Pin-discriminating: if the re-derive resolved with a default-font closure it
/// would carry the default handle and the `assert_ne!` fails; if it stopped
/// re-resolving fonts at all the key→handle equality fails.
#[test]
fn rederive_reresolves_fonts_through_the_asset_server() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let handle = add_theme_asset(
        &mut app,
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
    );
    // The handle the server yields for the spec's default_font key — `load` is
    // idempotent, so this is the SAME handle the re-derive will resolve to.
    let expected: Handle<Font> = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>("fonts/test.ttf");
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.world_mut()
        .insert_resource(ActiveThemeHandle::new(handle.clone()));

    app.update();

    hot_edit(
        &mut app,
        &handle,
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?,
    );
    inject_modified(&mut app, &handle);
    app.update();

    assert_eq!(
        app.world()
            .get_resource::<GdtfTheme>()
            .map(|t| t.button.font.clone()),
        Some(expected),
        "the re-derive must resolve the button font to the server's handle for its key",
    );
    assert_ne!(
        app.world()
            .get_resource::<GdtfTheme>()
            .map(|t| t.button.font.clone()),
        Some(Handle::<Font>::default()),
        "the re-resolved font must not be the default handle",
    );

    Ok(())
}

/// GTW-144 regression: with `apply_theme` change-driven, a hovered button
/// keeps its `HoverBg` across a steady-state frame instead of being clobbered
/// back to `PanelBg`.
///
/// Spawns a Themed panel button, hovers it (the swap a real pointer drives),
/// then runs a SECOND update with no theme change and no Interaction change.
/// The old every-frame `apply_theme` repainted `PanelBg` on that steady frame,
/// reverting the hover; the change-driven cadence must leave `HoverBg` intact.
///
/// Pin-discriminating: reverting to the every-frame cadence makes the second
/// update overwrite the fill back to `PanelBg` and this assert fails.
#[test]
fn change_driven_apply_theme_does_not_clobber_hover() -> Result<(), ron::error::SpannedError> {
    let mut app = app();

    let theme = spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
        .resolve(|_| Handle::<Font>::default());
    let hover_bg = *theme.button.hover;
    let panel_bg = *theme.button.color;
    app.world_mut().insert_resource(theme);

    let button = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Button), Button, Node::default()))
        .id();

    // Frame 1: apply_theme paints the base look (Added<Themed> + theme-changed).
    app.update();

    // Hover the button (the swap ui_focus_system would drive from a pointer).
    if let Some(mut interaction) = app.world_mut().get_mut::<Interaction>(button) {
        *interaction = Interaction::Hovered;
    }
    // Frame 2: theme_interaction swaps to HoverBg (Changed<Interaction>).
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(hover_bg),
        "precondition: the hover swap must land HoverBg",
    );

    // Frame 3: STEADY STATE — no theme change, no Interaction change.
    // Change-driven apply_theme must NOT run, so the hover is preserved.
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(hover_bg),
        "steady-state apply_theme must NOT clobber HoverBg back to PanelBg (GTW-144)",
    );
    assert_ne!(
        app.world().get::<BackgroundColor>(button).map(|c| c.0),
        Some(panel_bg),
        "the hovered button must not have reverted to the resting PanelBg",
    );

    Ok(())
}

/// GTW-144: a newly-Added `Themed` entity still gets its base look even on a
/// steady-state frame (no theme change) — the `any_themed_added` arm of the
/// change-driven cadence.
///
/// Pin-discriminating: dropping the `any_themed_added` run condition would
/// leave a widget spawned after the theme settled unpainted.
#[test]
fn newly_added_themed_entity_gets_base_look() -> Result<(), ron::error::SpannedError> {
    let mut app = app();
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );

    // Settle the theme: a first update marks GdtfTheme no-longer-changed.
    app.update();
    app.update();

    // Now spawn a Themed panel AFTER the theme settled (steady state).
    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(panel).map(|c| c.0),
        Some(Color::srgb(0.08, 0.08, 0.10)),
        "a Themed entity spawned on a steady-state frame must still get its base look",
    );

    Ok(())
}

/// GTW-144: a `GdtfTheme` change repaints ALL Themed entities (not just newly
/// added ones) — the `resource_changed` arm of the cadence (this is also the
/// retheme that AC3 leans on).
///
/// Pin-discriminating: dropping the `resource_changed` arm would leave an
/// already-settled widget on the old palette after a theme swap.
#[test]
fn theme_change_repaints_all_themed() -> Result<(), ron::error::SpannedError> {
    let mut app = app();
    app.world_mut().insert_resource(
        spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
            .resolve(|_| Handle::<Font>::default()),
    );

    let panel = app
        .world_mut()
        .spawn((Themed::new(ThemeRole::Panel), Node::default()))
        .id();

    // Settle: base look painted, then a steady frame.
    app.update();
    app.update();

    // Overwrite GdtfTheme with a NEW palette (no new Themed entities).
    let new_panel = Color::srgb(0.50, 0.10, 0.30);
    app.world_mut().insert_resource(
        spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?
            .resolve(|_| Handle::<Font>::default()),
    );
    app.update();

    assert_eq!(
        app.world().get::<BackgroundColor>(panel).map(|c| c.0),
        Some(new_panel),
        "a GdtfTheme change must repaint an already-settled Themed entity",
    );

    Ok(())
}
