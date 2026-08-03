//! Hot-reload chain from theme RON assets.

use bevy::{asset::AssetServer, prelude::*, text::Font};
use gdtf_assets::HotRonChain;

use crate::theme::{GdtfTheme, GdtfThemeSpec};

const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// Resolve a theme spec into a runtime theme using the asset server for fonts.
#[must_use]
pub fn resolve_theme_spec(spec: &GdtfThemeSpec, asset_server: &AssetServer) -> GdtfTheme {
    spec.clone()
        .resolve(|key| asset_server.load::<Font>(key.to_owned()))
}

/// Hot RON chain for the default UI theme path.
#[must_use]
pub fn theme_hot_ron_chain() -> HotRonChain<GdtfThemeSpec, GdtfTheme> {
    HotRonChain::new(THEME_RON_PATH, resolve_theme_spec, None)
}
