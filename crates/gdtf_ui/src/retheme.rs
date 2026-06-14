//! The live-retheme seam: re-derive [`GdtfTheme`] when the theme asset changes.
//!
//! GTW-137 makes the data-driven theme **hot-reloadable** in memory. When the
//! loose `assets/theme/grimdark.ron` asset is modified — by the OS file-watcher
//! in dev (GTW-138), or by a test injecting the message — Bevy emits an
//! [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for the theme
//! [`RonAsset`]. [`redrive_theme_on_asset_event`] reacts to that message,
//! re-derives the resolved [`GdtfTheme`] from the **updated** in-memory spec, and
//! overwrites the [`GdtfTheme`] resource in place.
//!
//! ## Why this is all it does (the cadence lives elsewhere)
//!
//! This system does **not** re-paint widgets itself. Overwriting the
//! [`GdtfTheme`] resource through [`ResMut`] marks it *changed*, and
//! [`apply_theme`](crate::themed::apply_theme) — made change-driven by GTW-144 —
//! then repaints every [`Themed`](crate::themed::Themed) entity the same frame.
//! For that to happen in one frame, [`UiPlugin`](crate::UiPlugin) orders this
//! system **before** the [`UiSystems::ApplyTheme`](crate::themed::UiSystems::ApplyTheme)
//! set (bevy-traps rule 3).
//!
//! ## Engine-event driven, not file-watcher driven
//!
//! This is the deterministic, headless-testable LOGIC: it reacts to an
//! [`AssetEvent`](bevy::asset::AssetEvent) **however it arrives** — a real
//! file-watcher in dev or an injected message in a test. The OS file-watcher
//! wiring (the `AssetPlugin` watch source) is GTW-138 and is *not* part of this
//! module.

use bevy::{asset::AssetEvent, prelude::*};
use gdtf_assets::RonAsset;

use crate::theme::{ActiveThemeHandle, GdtfTheme, GdtfThemeSpec};

/// Re-derives [`GdtfTheme`] from the updated theme asset on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified).
///
/// Reads the [`MessageReader`] of
/// [`AssetEvent`](bevy::asset::AssetEvent)`<`[`RonAsset`]`<`[`GdtfThemeSpec`]`>>`
/// — asset events are **messages** in Bevy 0.18, so this is a `MessageReader`,
/// not an `EventReader` (bevy-traps rule 4) — and acts only on a
/// [`Modified`](bevy::asset::AssetEvent::Modified) event whose `id` matches the
/// [`ActiveThemeHandle`]; events for any other handle are ignored.
///
/// On a match it:
///
/// 1. reads the updated [`RonAsset`]`<`[`GdtfThemeSpec`]`>` out of the
///    `Assets` collection (the in-memory value the watcher just refreshed),
/// 2. clones the [`GdtfThemeSpec`] and resolves it the **same** way the `Load`
///    scene does ([`GdtfThemeSpec::resolve`]), **keeping** the font
///    [`Handle`](bevy::asset::Handle) carried by the current [`GdtfTheme`] (a
///    theme RON edit does not reload the font), and
/// 3. overwrites the [`GdtfTheme`] resource in place through [`ResMut`].
///
/// Overwriting via [`ResMut`] marks [`GdtfTheme`] changed, which is the signal the
/// change-driven [`apply_theme`](crate::themed::apply_theme) repaints on — so the
/// retheme is laid down across every [`Themed`](crate::themed::Themed) entity the
/// same frame, given the ordering [`UiPlugin`](crate::UiPlugin) installs.
///
/// Guarded so it never panics when the theme is absent (pre-`Load`): it takes the
/// theme as [`ResMut`] but is registered with `.run_if(resource_exists::<GdtfTheme>)`,
/// and it takes [`ActiveThemeHandle`] and the `Assets` collection as
/// [`Option`]al borrows, returning early if either is missing (bevy-traps rule 1).
pub fn redrive_theme_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<GdtfThemeSpec>>>,
    active: Option<Res<ActiveThemeHandle>>,
    theme_assets: Option<Res<Assets<RonAsset<GdtfThemeSpec>>>>,
    theme: Option<ResMut<GdtfTheme>>,
) {
    let (Some(active), Some(theme_assets), Some(mut theme)) = (active, theme_assets, theme) else {
        // Drain the reader so a pre-`Load` event does not linger and re-fire once
        // the resources arrive; there is nothing to re-derive yet.
        events.clear();
        return;
    };

    let active_id = active.id();

    // Act once per frame even if several Modified events arrive: a single
    // re-derive from the latest in-memory value covers them all. Only re-derive
    // when at least one Modified event targets the active theme handle. GTW-146
    // hot-reload instrumentation: log EVERY incoming theme-asset event so a live
    // edit can be traced — no line here after a save means no `AssetEvent` reached
    // us at all (e.g. an atomic-rename save the OS watcher silently drops).
    let mut modified = false;
    for event in events.read() {
        info!("theme hot-reload: received {event:?} (active id {active_id:?})");
        if matches!(event, AssetEvent::Modified { id } if *id == active_id) {
            modified = true;
        }
    }
    if !modified {
        return;
    }

    let Some(updated) = theme_assets.get(&**active) else {
        // The asset was modified but is not currently in the collection (e.g. a
        // transient reload state); leave the existing theme until it settles.
        warn!(
            "theme hot-reload: Modified received but the asset is not yet in the \
             collection; retrying next frame",
        );
        return;
    };

    // Re-derive the SAME way the Load scene does, keeping the already-loaded font
    // handle — a theme RON edit changes colors/scalars, not the font asset.
    let font = theme.font.clone();
    *theme = (**updated).clone().resolve(font);
    info!(
        "theme hot-reload: re-derived GdtfTheme (panel_bg {:?}, hover_bg {:?}, press_bg {:?})",
        *theme.panel_bg, *theme.hover_bg, *theme.press_bg,
    );
}

#[cfg(test)]
mod tests {
    use bevy::{
        MinimalPlugins,
        asset::{AssetApp, AssetEvent, AssetPlugin, Assets, Handle},
        input::InputPlugin,
        text::Font,
        ui::{BackgroundColor, Interaction, Node, widget::Button},
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};

    use super::*;
    use crate::{
        UiPlugin,
        theme::{ActiveThemeHandle, GdtfTheme, GdtfThemeSpec},
        themed::{ThemeRole, Themed},
    };

    /// Builds a [`GdtfThemeSpec`] from caller-chosen text + panel colors (hover /
    /// press track the panel), mirroring the shipped `grimdark.ron` shape.
    /// Returns the `ron` error so a malformed literal surfaces via `?` rather than
    /// a denied `unwrap`/`panic`.
    fn spec(
        text: [f32; 3],
        panel: [f32; 3],
        hover: [f32; 3],
    ) -> Result<GdtfThemeSpec, ron::error::SpannedError> {
        let ron = format!(
            "(\
             text: ({tr}, {tg}, {tb}, 1.0), \
             panel_bg: ({pr}, {pg}, {pb}, 1.0), \
             border_color: (0.20, 0.20, 0.24, 1.0), \
             border_width_px: 1.0, \
             corner_radius_px: 2.0, \
             margin_left_px: 8.0, margin_right_px: 8.0, \
             margin_top_px: 6.0, margin_bottom_px: 6.0, \
             font_size_pt: 18.0, \
             font_key: \"fonts/test.ttf\", \
             hover_bg: ({hr}, {hg}, {hb}, 1.0), \
             press_bg: ({pr}, {pg}, {pb}, 1.0), \
             disabled_bg: (0.16, 0.16, 0.18, 0.55))",
            tr = text[0],
            tg = text[1],
            tb = text[2],
            pr = panel[0],
            pg = panel[1],
            pb = panel[2],
            hr = hover[0],
            hg = hover[1],
            hb = hover[2],
        );
        ron::from_str(&ron)
    }

    /// A headless app with the real production wiring: `MinimalPlugins`, then
    /// `AssetPlugin` (so the `Assets` collection and `AssetEvent` messages exist),
    /// the theme RON asset registered, and `UiPlugin` (which registers the
    /// re-derive system before the `ApplyTheme` set and the change-driven
    /// `apply_theme`). `InputPlugin` is added because `UiPlugin`'s focus-nav layer
    /// pulls in `InputDispatchPlugin`, whose dispatch systems need the input
    /// message buffers (see the `interaction` tests / bevy-traps rule 1).
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
        if let Some(asset) = assets.get_mut(handle) {
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
    fn modified_event_rederives_theme_and_repaints_widgets() -> Result<(), ron::error::SpannedError>
    {
        let mut app = app();

        let handle = add_theme_asset(
            &mut app,
            spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
        );
        app.world_mut().insert_resource(
            spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?
                .resolve(Handle::<Font>::default()),
        );
        app.world_mut()
            .insert_resource(ActiveThemeHandle(handle.clone()));

        let text = app
            .world_mut()
            .spawn((Themed(ThemeRole::Text), Text::new("x")))
            .id();
        let panel = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()))
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
            world.get_resource::<GdtfTheme>().map(|t| *t.text),
            Some(new_text),
            "GdtfTheme.text must be re-derived to the NEW palette",
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
                .resolve(Handle::<Font>::default()),
        );
        app.world_mut().insert_resource(ActiveThemeHandle(active));

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

    /// The re-derive keeps the existing resolved font handle (a theme RON edit
    /// changes colors/scalars, not the font asset) — AC2.
    ///
    /// Pin-discriminating: if the re-derive resolved with a default font handle
    /// it would clobber the real one and this assert fails.
    #[test]
    fn rederive_keeps_the_existing_font_handle() -> Result<(), ron::error::SpannedError> {
        let mut app = app();

        let handle = add_theme_asset(
            &mut app,
            spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?,
        );
        // A distinctive, non-default font handle to prove it survives the re-derive.
        let font: Handle<Font> = app
            .world_mut()
            .resource_mut::<Assets<Font>>()
            .reserve_handle();
        app.world_mut().insert_resource(
            spec([0.84, 0.80, 0.73], [0.08, 0.08, 0.10], [0.20, 0.20, 0.24])?.resolve(font.clone()),
        );
        app.world_mut()
            .insert_resource(ActiveThemeHandle(handle.clone()));

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
                .map(|t| t.font.clone()),
            Some(font),
            "the re-derive must keep the existing resolved font handle, not reset it",
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
            .resolve(Handle::<Font>::default());
        let hover_bg = *theme.hover_bg;
        let panel_bg = *theme.panel_bg;
        app.world_mut().insert_resource(theme);

        let button = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Button, Node::default()))
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
                .resolve(Handle::<Font>::default()),
        );

        // Settle the theme: a first update marks GdtfTheme no-longer-changed.
        app.update();
        app.update();

        // Now spawn a Themed panel AFTER the theme settled (steady state).
        let panel = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()))
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
                .resolve(Handle::<Font>::default()),
        );

        let panel = app
            .world_mut()
            .spawn((Themed(ThemeRole::Panel), Node::default()))
            .id();

        // Settle: base look painted, then a steady frame.
        app.update();
        app.update();

        // Overwrite GdtfTheme with a NEW palette (no new Themed entities).
        let new_panel = Color::srgb(0.50, 0.10, 0.30);
        app.world_mut().insert_resource(
            spec([0.10, 0.90, 0.20], [0.50, 0.10, 0.30], [0.70, 0.10, 0.40])?
                .resolve(Handle::<Font>::default()),
        );
        app.update();

        assert_eq!(
            app.world().get::<BackgroundColor>(panel).map(|c| c.0),
            Some(new_panel),
            "a GdtfTheme change must repaint an already-settled Themed entity",
        );

        Ok(())
    }
}
