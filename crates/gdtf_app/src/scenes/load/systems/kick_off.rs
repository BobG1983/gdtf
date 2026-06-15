//! `OnEnter(AppState::Load)`: start the real async asset loads.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use gdtf_battle_sim::situation::Situation;
use gdtf_ui::theme::GdtfThemeSpec;

use crate::scenes::load::resources::{FontFolderHandle, LoadHandles, SituationHandle, ThemeHandle};

/// Path of the loose theme RON, relative to the asset source root.
const THEME_RON_PATH: &str = "theme/grimdark.ron";

/// Path of the loose fonts folder, relative to the asset source root.
const FONTS_FOLDER_PATH: &str = "fonts";

/// Path of the loose authored-situation RON, relative to the asset source root
/// (GTW-205 / E10.3 — the canonical authored battlefield the Generation slice reads).
const SITUATION_RON_PATH: &str = "situations/skirmish.ron";

/// Kicks off the theme-RON load and the fonts-folder preload, storing their typed
/// handles.
///
/// Loads `theme/grimdark.ron` as a `RonAsset<GdtfThemeSpec>` (through the GTW-136
/// loader) and preloads the entire `fonts` folder via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder) (GTW-149 —
/// loads ALL fonts up front so any font a sub-theme selects, override or default,
/// is resident), AND loads `situations/skirmish.ron` as a `RonAsset<Situation>`
/// (GTW-205 / E10.3 — through the same generic loader), then inserts the
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
    let fonts = FontFolderHandle(asset_server.load_folder(FONTS_FOLDER_PATH));
    let situation = SituationHandle(asset_server.load::<RonAsset<Situation>>(SITUATION_RON_PATH));

    commands.insert_resource(LoadHandles {
        theme,
        fonts,
        situation,
    });
}
