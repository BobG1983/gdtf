//! `OnEnter(AppState::Load)`: start the real async asset loads.

use bevy::prelude::*;
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    situation::Situation,
    tuning::{CombatTuning, GangerStatTuning},
};
use gdtf_ui::theme::GdtfThemeSpec;

use crate::states::load::resources::{
    ArmorsFolderHandle, FontFolderHandle, GangsFolderHandle, InjuriesFolderHandle, LoadHandles,
    PrefabsFolderHandle, SituationHandle, StatTuningHandle, TerrainFolderHandle, ThemeHandle,
    ThemesFolderHandle, TuningHandle, WeaponsFolderHandle,
};

/// Path of the loose theme RON, relative to the asset source root.
const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// Path of the loose fonts folder, relative to the asset source root.
const FONTS_FOLDER_PATH: &str = "fonts";

/// Path of the loose authored-situation RON, relative to the asset source root
/// (GTW-205 / E10.3 — the canonical authored battlefield the Generation slice reads).
const SITUATION_RON_PATH: &str = "situations/skirmish.ron";

/// Path of the loose combat-tuning RON, relative to the asset source root
/// (GTW-206 / E10.4 — the shipped balance coefficients the sim marches with;
/// `core_tuning/combat.tuning.ron`).
const TUNING_RON_PATH: &str = "core_tuning/combat.tuning.ron";

/// Path of the loose ganger stat-tuning RON, relative to the asset source root
/// (GTW-384 — the attribute → computed-stat derivation weights, a SEPARATE file from
/// `core_tuning/combat.tuning.ron`).
const STAT_TUNING_RON_PATH: &str = "core_tuning/stat.tuning.ron";

/// Path of the loose weapons folder, relative to the asset source root (GTW-257 —
/// the per-weapon `assets/content/weapons/*.ron` files the registry is built from). Its OWN
/// folder so the `.ron` loader dispatch is unambiguous (weapons only, no armour).
const WEAPONS_DIR: &str = "content/weapons";

/// Path of the loose armor folder, relative to the asset source root (GTW-269 —
/// the per-armor `assets/content/armor/*.ron` files the registry is built from). Its OWN
/// folder so the `.ron` loader dispatch is unambiguous (armor only, no weapons).
const ARMOR_DIR: &str = "content/armor";

/// Path of the loose terrain folder, relative to the asset source root (GTW-394 —
/// the per-terrain-piece `assets/content/terrain/*.terrain.ron` files the registry is
/// built from). Its OWN folder so the `.ron` loader dispatch is unambiguous (terrain only).
const TERRAIN_DIR: &str = "content/terrain";

/// Path of the loose themes folder, relative to the asset source root (GTW-409 —
/// the per-theme `assets/content/themes/*.theme.ron` named-tile catalogs the
/// `ThemeCatalogRegistry` is built from). A NEW content category parallel to
/// terrain/weapons/armor/injuries (USER DIRECTION 2026-06-26: content under
/// `assets/content/<x>/`); its OWN folder + the dedicated `theme.ron` compound
/// extension keep the `.ron` loader dispatch unambiguous (themes only).
const THEMES_DIR: &str = "content/themes";

/// Path of the loose injuries folder, relative to the asset source root (GTW-437 —
/// the per-injury `assets/content/injuries/**/*.injury.ron` files + the per-part
/// `content/injuries/weighting/*.weighting.ron` files the registry + tables are built
/// from). One recursive folder carrying both asset types; the dedicated compound
/// extensions (`injury.ron` / `weighting.ron`) keep the `.ron` loader dispatch unambiguous.
const INJURIES_DIR: &str = "content/injuries";

/// Path of the loose gangs folder, relative to the asset source root (GTW-415 —
/// the per-gang `assets/content/gangs/*.gang.ron` rosters the `GangRegistry` is built
/// from). Its OWN folder + the dedicated `gang.ron` compound extension keep the `.ron`
/// loader dispatch unambiguous (gangs only) — the weapons/armor/terrain precedent.
const GANGS_DIR: &str = "content/gangs";

/// Path of the loose maps (prefab) folder, relative to the asset source root (GTW-418 —
/// the per-prefab `assets/content/maps/<theme>/<size>/*.prefab.ron` fragments the
/// `PrefabRegistry` is built from). NESTED by theme + size (the user-locked path); a
/// recursive `load_folder` walks the whole tree, and the dedicated `prefab.ron` compound
/// extension keeps the `.ron` loader dispatch unambiguous (prefabs only) — the
/// weapons/armor/terrain/gangs precedent.
const MAPS_DIR: &str = "content/maps";

/// Kicks off the theme-RON load and the fonts-folder preload, storing their typed
/// handles.
///
/// Loads `core_tuning/ui_theme.tuning.ron` as a `RonAsset<GdtfThemeSpec>` (through
/// the GTW-136 loader) and preloads the entire `fonts` folder via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder) (GTW-149 —
/// loads ALL fonts up front so any font a sub-theme selects, override or default,
/// is resident), AND loads `situations/skirmish.ron` as a `RonAsset<Situation>`
/// (GTW-205 / E10.3 — through the same generic loader) AND
/// `core_tuning/combat.tuning.ron` as a `RonAsset<CombatTuning>` (GTW-206 / E10.4
/// — through the same generic loader)
/// AND preloads the entire `content/weapons` folder via `load_folder` (GTW-257 — every
/// `assets/content/weapons/*.ron`, each a `RonAsset<WeaponSpec>`, so the poll/resolve
/// system can build the name-keyed
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry))
/// AND preloads the entire `content/armor` folder via `load_folder` (GTW-269 — every
/// `assets/content/armor/*.ron`, each a `RonAsset<ArmorSpec>`, so the poll/resolve
/// system can build the name-keyed [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry))
/// AND preloads the entire `content/terrain` folder via `load_folder` (GTW-394 — every
/// `assets/content/terrain/*.terrain.ron`, each a `RonAsset<TerrainSpec>`, so the
/// poll/resolve system can build the name-keyed
/// [`TerrainRegistry`](gdtf_battle_sim::terrain::piece::TerrainRegistry))
/// AND preloads the entire `content/themes` folder via `load_folder` (GTW-409 — every
/// `assets/content/themes/*.theme.ron`, each a `RonAsset<ThemeSpec>`, so the
/// poll/resolve system can build the theme-keyed
/// [`ThemeCatalogRegistry`](gdtf_battle_sim::level::ThemeCatalogRegistry)),
/// then inserts the Load-scoped [`LoadHandles`] resource the poll/resolve system
/// reads.
///
/// It takes `Option<Res<AssetServer>>`: a `MinimalPlugins` headless app has **no**
/// [`AssetServer`], so the system must no-op rather than panic when it is absent
/// (bevy-traps rule 1). When the `AssetServer` is present (the running app and the
/// real-asset harness) the loads fire and `LoadHandles` is inserted.
pub(in crate::states::load) fn kick_off_loads(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
) {
    let Some(asset_server) = asset_server else {
        return;
    };

    let theme = ThemeHandle::new(asset_server.load::<RonAsset<GdtfThemeSpec>>(THEME_RON_PATH));
    let fonts = FontFolderHandle::new(asset_server.load_folder(FONTS_FOLDER_PATH));
    let situation =
        SituationHandle::new(asset_server.load::<RonAsset<Situation>>(SITUATION_RON_PATH));
    let tuning = TuningHandle::new(asset_server.load::<RonAsset<CombatTuning>>(TUNING_RON_PATH));
    let stat_tuning = StatTuningHandle::new(
        asset_server.load::<RonAsset<GangerStatTuning>>(STAT_TUNING_RON_PATH),
    );
    let weapons = WeaponsFolderHandle::new(asset_server.load_folder(WEAPONS_DIR));
    let armor = ArmorsFolderHandle::new(asset_server.load_folder(ARMOR_DIR));
    let terrain = TerrainFolderHandle::new(asset_server.load_folder(TERRAIN_DIR));
    let themes = ThemesFolderHandle::new(asset_server.load_folder(THEMES_DIR));
    let injuries = InjuriesFolderHandle::new(asset_server.load_folder(INJURIES_DIR));
    let gangs = GangsFolderHandle::new(asset_server.load_folder(GANGS_DIR));
    let prefabs = PrefabsFolderHandle::new(asset_server.load_folder(MAPS_DIR));

    commands.insert_resource(LoadHandles {
        theme,
        fonts,
        situation,
        tuning,
        stat_tuning,
        weapons,
        armor,
        terrain,
        themes,
        injuries,
        gangs,
        prefabs,
    });
}
