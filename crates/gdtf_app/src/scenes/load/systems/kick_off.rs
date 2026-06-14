//! `OnEnter(AppState::Load)`: start the real async asset loads.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use gdtf_ui::theme::GdtfThemeSpec;

use crate::scenes::load::resources::{FontHandle, LoadHandles, ThemeHandle};

/// Path of the loose theme RON, relative to the asset source root.
const THEME_RON_PATH: &str = "theme/grimdark.ron";

/// Path of the loose theme font, relative to the asset source root.
const FONT_PATH: &str = "fonts/Alegreya-Variable.ttf";

/// Kicks off the theme-RON and font loads and stores their typed handles.
///
/// Loads `theme/grimdark.ron` as a `RonAsset<GdtfThemeSpec>` (through the GTW-136
/// loader) and `fonts/Alegreya-Variable.ttf` as a [`Font`], then inserts the
/// Load-scoped [`LoadHandles`] resource the poll/resolve system reads.
///
/// It takes `Option<Res<AssetServer>>`: a `MinimalPlugins` headless app has **no**
/// [`AssetServer`], so the system must no-op rather than panic when it is absent
/// (bevy-traps rule 1). When the `AssetServer` is present (the running app and the
/// real-asset harness) the loads fire and `LoadHandles` is inserted.
pub(in crate::scenes::load) fn kick_off_loads(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };

    let theme = ThemeHandle(asset_server.load::<RonAsset<GdtfThemeSpec>>(THEME_RON_PATH));
    let font = FontHandle(asset_server.load::<Font>(FONT_PATH));

    commands.insert_resource(LoadHandles { theme, font });
}
