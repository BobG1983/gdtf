//! The theme's hot-RON chain pieces: the shared spec→theme map hook and the
//! chain config the generic redrive runs with.

use bevy::{asset::AssetServer, prelude::*, text::Font};
use gdtf_assets::HotRonChain;

use crate::theme::{GdtfTheme, GdtfThemeSpec};

/// The path of the loose theme RON, relative to the asset source root — the
/// canonical `core_tuning/ui_theme.tuning.ron` every theme host loads.
const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// The ONE spec→theme map hook: resolve a [`GdtfThemeSpec`] into a [`GdtfTheme`],
/// loading each font key through the [`AssetServer`].
///
/// This is the [`HotRonMapFn`](gdtf_assets::HotRonMapFn) of the theme's hot-RON
/// chain (GTW-564) AND the shared resolve every bespoke theme host calls — the
/// game's `Load` scene and the editor's Load pass resolve the spec the SAME way,
/// so a hot redrive yields exactly what a restart would. `load` is idempotent
/// (an already-loaded key returns its existing handle), and a hot edit that
/// switches a sub-theme's font key re-loads the new font.
#[must_use]
pub fn resolve_theme_spec(spec: &GdtfThemeSpec, asset_server: &AssetServer) -> GdtfTheme {
    spec.clone()
        .resolve(|key| asset_server.load::<Font>(key.to_owned()))
}

/// The theme's [`HotRonChain`] config: the canonical theme path plus the
/// font-resolving [`resolve_theme_spec`] map hook (no `Failed` fallback — the
/// game's Load-side resolve owns the theme's failure path, which ALSO records
/// `LoadFailed` and pairs with the fonts-folder gate, so it stays bespoke;
/// see the GTW-564 C7 record).
///
/// Inserted by [`UiPlugin`](crate::UiPlugin) for the game, and by the editor's
/// `register_load` (the second in-app asset host) — both then register the SAME
/// generic [`redrive_hot_ron_resource`](gdtf_assets::redrive_hot_ron_resource)
/// `::<GdtfThemeSpec, GdtfTheme>` against the generic
/// [`HotRonHandle`](gdtf_assets::HotRonHandle)`<GdtfThemeSpec>` their resolves
/// store, so the live-retheme drain body exists exactly once.
#[must_use]
pub fn theme_hot_ron_chain() -> HotRonChain<GdtfThemeSpec, GdtfTheme> {
    HotRonChain::new(THEME_RON_PATH, resolve_theme_spec, None)
}
