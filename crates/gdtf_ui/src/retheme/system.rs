//! The [`redrive_theme_on_asset_event`] live-retheme system.

use bevy::{
    asset::{AssetEvent, AssetServer},
    prelude::*,
};
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
///    scene does ([`GdtfThemeSpec::resolve`] with an
///    `|key| asset_server.load::<Font>(key)` resolver), so a hot-edit that
///    switches a sub-theme's font (the override / `default_font` keys) re-loads
///    the new font — `load` is idempotent and returns the already-loaded handle
///    for an unchanged key, and
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
    asset_server: Res<AssetServer>,
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

    // Re-derive the SAME way the Load scene does, resolving fonts through the
    // asset server — `load` is idempotent (already-loaded keys return their
    // existing handle), and a hot-edit that switches a font key re-loads it.
    *theme = (**updated)
        .clone()
        .resolve(|key| asset_server.load::<Font>(key.to_owned()));
    info!(
        "theme hot-reload: re-derived GdtfTheme (panel {:?}, button.hover {:?}, button.pressed {:?})",
        *theme.panel.color, *theme.button.hover, *theme.button.pressed,
    );
}
