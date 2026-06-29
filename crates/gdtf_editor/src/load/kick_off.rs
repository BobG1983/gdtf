//! `OnEnter(EditorState::Load)`: start the editor's async asset loads.
//!
//! A slim mirror of `gdtf_app`'s `kick_off_loads`, trimmed to the editor's needs: the
//! theme RON plus the weapon / armor / theme-catalog content folders. The editor does
//! NOT load the situation, combat/stat tuning, terrain, injuries, or gangs — those drive
//! the GAME's battle sim, which the editor does not run (the GTW-417 housing constraint:
//! the procgen assembly + debug visualizer stay in the main game, not the editor).

use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_ui::theme::GdtfThemeSpec;

use crate::load::handles::{
    EditorArmorFolderHandle, EditorLoadHandles, EditorTerrainModelFolderHandle, EditorThemeHandle,
    EditorThemesFolderHandle, EditorWeaponsFolderHandle,
};

/// Path of the loose theme RON, relative to the asset source root (the same shipped
/// theme the game loads, so the editor's themed regions match the game's look).
const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// Path of the loose weapons folder, relative to the asset source root — each
/// `*.weapon.ron` is a `RonAsset<WeaponSpec>` the resolve pass builds the
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) from.
const WEAPONS_DIR: &str = "content/weapons";

/// Path of the loose armor folder, relative to the asset source root — each
/// `*.armor.ron` is a `RonAsset<ArmorSpec>` the resolve pass builds the
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) from.
const ARMOR_DIR: &str = "content/armor";

/// Path of the loose themes folder, relative to the asset source root — each
/// `*.theme.ron` is a `RonAsset<ThemeSpec>` the resolve pass builds the
/// [`ThemeCatalogRegistry`](gdtf_battle_sim::level::ThemeCatalogRegistry) from (the
/// GTW-409 theme tile catalog the palette/canvas children consume).
const THEMES_DIR: &str = "content/themes";

/// Path of the loose NEW per-theme terrain-model folder, relative to the asset source root
/// (GTW-487) — its members are `*.terrain_def.ron` (`RonAsset<TerrainDef>`) +
/// `*.terrain_theme.ron` (`RonAsset<UuidThemeDef>`) the resolve pass builds the UUID-keyed
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) from. A NEW root DISTINCT
/// from the legacy `content/themes`, with dedicated extensions that do not collide with the
/// legacy `theme.ron` / `terrain.ron` loaders.
const TERRAIN_MODEL_DIR: &str = "terrain";

/// Kicks off the editor's theme-RON load and the weapon / armor / themes folder loads,
/// storing their typed handles in [`EditorLoadHandles`].
///
/// Takes `Option<Res<AssetServer>>` so a headless `MinimalPlugins` app with no
/// [`AssetServer`] no-ops rather than panics (`bevy-traps.md` #1) — though the editor
/// always runs under `DefaultPlugins`, the guard mirrors the game's kick-off and keeps a
/// state-machine harness valid. When the server is present the loads fire and
/// [`EditorLoadHandles`] is inserted for the resolve pass to poll.
pub(crate) fn kick_off_editor_loads(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };

    let theme =
        EditorThemeHandle::new(asset_server.load::<RonAsset<GdtfThemeSpec>>(THEME_RON_PATH));
    let weapons = EditorWeaponsFolderHandle::new(asset_server.load_folder(WEAPONS_DIR));
    let armor = EditorArmorFolderHandle::new(asset_server.load_folder(ARMOR_DIR));
    let themes = EditorThemesFolderHandle::new(asset_server.load_folder(THEMES_DIR));
    let terrain_model =
        EditorTerrainModelFolderHandle::new(asset_server.load_folder(TERRAIN_MODEL_DIR));

    commands.insert_resource(EditorLoadHandles {
        theme,
        weapons,
        armor,
        themes,
        terrain_model,
    });
}
