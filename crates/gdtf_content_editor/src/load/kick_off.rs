//! `OnEnter(EditorState::Load)`: start the editor's async asset loads.
//!
//! A slim mirror of `gdtf_app`'s `kick_off_loads`, trimmed to the editor's needs: the
//! theme RON, the weapon / armor content folders, the NEW UUID-keyed per-theme `content/terrain/`
//! folder (GTW-487), and the presenter's tile-role RON (GTW-495 — the per-def graphic
//! resolution table). The editor does NOT load the situation, combat/stat tuning,
//! injuries, or gangs — those drive the GAME's battle sim, which the editor does not run.

use bevy::{asset::AssetServer, prelude::*};
use gdtf_assets::RonAsset;
use gdtf_battle_presenter::TileRoles;
use gdtf_ui::theme::GdtfThemeSpec;

use crate::load::handles::{
    EditorArmorFolderHandle, EditorLoadHandles, EditorTerrainModelFolderHandle, EditorThemeHandle,
    EditorTileRolesHandle, EditorWeaponsFolderHandle,
};

/// Path of the loose theme RON, relative to the asset source root (the same shipped
/// theme the game loads, so the editor's themed regions match the game's look).
const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// Path of the loose RANGED-weapons folder, relative to the asset source root — each
/// `ranged/*.weapon.ron` is a `RonAsset<WeaponSpec>` the resolve pass builds the
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) from. GTW-505 split the
/// weapons tree into `ranged/` + `melee/`; the editor's `WeaponRegistry` build loads ONLY
/// the `ranged/` leaf (the editor has no melee registry yet), so the recursive
/// `load_folder` never hits the sibling `melee/` `.melee_weapon.ron` members it does not
/// register a loader for.
const WEAPONS_DIR: &str = "content/weapons/ranged";

/// Path of the loose armor folder, relative to the asset source root — each
/// `*.armor.ron` is a `RonAsset<ArmorSpec>` the resolve pass builds the
/// [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry) from.
const ARMOR_DIR: &str = "content/armor";

/// Path of the loose NEW per-theme terrain-model folder, relative to the asset source root
/// (GTW-487) — its members are `*.terrain_def.ron` (`RonAsset<TerrainDef>`) +
/// `*.terrain_theme.ron` (`RonAsset<UuidThemeDef>`) the resolve pass builds the UUID-keyed
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) from — the editor's SOLE
/// terrain/theme source after GTW-495 (the legacy theme catalog is retired; the root moved
/// under `content/` in GTW-562).
const TERRAIN_MODEL_DIR: &str = "content/terrain";

/// Path of the loose tile-role RON, relative to the asset source root (GTW-495) — the SAME
/// `sprites/tile_roles.spritedef.ron` the presenter loads. The editor resolves a terrain
/// def's `presenter_kind.graphic_name` to a terrain atlas index through the resolved
/// [`TileRoles`](gdtf_battle_presenter::TileRoles), so its sprites match the battlescape's.
const TILE_ROLES_RON_PATH: &str = "sprites/tile_roles.spritedef.ron";

/// Kicks off the editor's theme-RON load, the weapon / armor / terrain-model folder loads,
/// and the tile-role RON load, storing their typed handles in [`EditorLoadHandles`].
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
    let terrain_model =
        EditorTerrainModelFolderHandle::new(asset_server.load_folder(TERRAIN_MODEL_DIR));
    let tile_roles =
        EditorTileRolesHandle::new(asset_server.load::<RonAsset<TileRoles>>(TILE_ROLES_RON_PATH));

    commands.insert_resource(EditorLoadHandles {
        theme,
        weapons,
        armor,
        terrain_model,
        tile_roles,
    });
}
