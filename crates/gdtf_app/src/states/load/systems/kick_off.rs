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
    MeleeWeaponsFolderHandle, PrefabsV2FolderHandle, SituationHandle, StatTuningHandle,
    TerrainModelFolderHandle, ThemeHandle, TuningHandle, WeaponsFolderHandle,
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

/// Path of the loose RANGED-weapons folder, relative to the asset source root (GTW-257;
/// GTW-505 split it into `ranged/`) — the per-weapon
/// `assets/content/weapons/ranged/*.weapon.ron` files the [`WeaponRegistry`] is built from.
/// Its OWN leaf folder so a recursive `load_folder` walks ONLY `.weapon.ron` members (the
/// sibling `melee/` folder is loaded separately), keeping the registry build clean.
const WEAPONS_DIR: &str = "content/weapons/ranged";

/// Path of the loose MELEE-weapons folder, relative to the asset source root (GTW-505) —
/// the per-weapon `assets/content/weapons/melee/*.melee_weapon.ron` files the
/// [`MeleeWeaponRegistry`](gdtf_battle_sim::weapon::MeleeWeaponRegistry) is built from. Its
/// OWN leaf folder (sibling of `ranged/`) so a recursive `load_folder` walks ONLY
/// `.melee_weapon.ron` members, the ranged-folder precedent.
const MELEE_WEAPONS_DIR: &str = "content/weapons/melee";

/// Path of the loose armor folder, relative to the asset source root (GTW-269 —
/// the per-armor `assets/content/armor/*.ron` files the registry is built from). Its OWN
/// folder so the `.ron` loader dispatch is unambiguous (armor only, no weapons).
const ARMOR_DIR: &str = "content/armor";

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

/// Path of the loose v2 maps (prefab) folder, relative to the asset source root (GTW-489 —
/// child T05c of the GTW-476 data-model refactor; the per-prefab
/// `assets/maps/<theme>/<size>/*.prefab_v2.ron` fragments the `PrefabRegistry2` is built
/// from). One recursive `load_folder` walks the whole `<theme>/<size>/` tree, and the
/// dedicated `prefab_v2.ron` compound extension keeps the `.ron` loader dispatch
/// unambiguous (v2 prefabs only). GTW-494 retired the legacy `content/maps/*.prefab.ron`
/// loader, so this is the ONLY prefab load in the Load flow.
const MAPS_V2_DIR: &str = "maps";

/// Path of the loose per-theme terrain-model folder, relative to the asset source root
/// (GTW-487 — the GTW-484/485 UUID-keyed terrain + theme defs under the per-theme layout
/// `terrain/<theme>/<tile>.terrain_def.ron` + `terrain/<theme>/<theme>.terrain_theme.ron`).
/// One recursive `load_folder` walks every `<theme>/` subfolder, and the dedicated compound
/// extensions (`terrain_def.ron` / `terrain_theme.ron`) keep the `.ron` loader dispatch
/// unambiguous. GTW-494 retired the legacy flat-dir `content/terrain` / `content/themes`
/// loaders, so this is the ONLY terrain / theme load in the Load flow.
const TERRAIN_MODEL_DIR: &str = "terrain";

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
/// AND preloads the `content/weapons/ranged` leaf folder via `load_folder` (GTW-257; GTW-505
/// split the tree into `ranged/` + `melee/` — every
/// `assets/content/weapons/ranged/*.weapon.ron`, each a `RonAsset<WeaponSpec>`, so the poll/resolve
/// system can build the name-keyed
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry))
/// AND preloads the entire `content/armor` folder via `load_folder` (GTW-269 — every
/// `assets/content/armor/*.ron`, each a `RonAsset<ArmorSpec>`, so the poll/resolve
/// system can build the name-keyed [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry))
/// AND preloads the per-theme `terrain` folder via `load_folder` (GTW-487 — every
/// `terrain/<theme>/*.terrain_def.ron` + `*.terrain_theme.ron`, the UUID-keyed
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) the sim + procgen +
/// presenter consume) AND preloads the `maps` folder via `load_folder` (GTW-489 — every
/// `maps/<theme>/<size>/*.prefab_v2.ron`, the UUID-keyed
/// [`PrefabRegistry2`](gdtf_battle_sim::level::PrefabRegistry2) the procgen pipeline packs),
/// then inserts the Load-scoped [`LoadHandles`] resource the poll/resolve system
/// reads.
///
/// GTW-494 (child T08 of GTW-476): the OLD flat-dir `content/terrain` / `content/themes` /
/// `content/maps` folder loads were RETIRED — the per-theme `terrain` model + the `maps` v2
/// prefab loads above are the ONLY terrain / theme / prefab loads in the Load flow.
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
    // GTW-505: the sibling melee-weapons folder loads through its OWN folder handle so the
    // `MeleeWeaponRegistry` builds from the `.melee_weapon.ron` members only.
    let melee_weapons = MeleeWeaponsFolderHandle::new(asset_server.load_folder(MELEE_WEAPONS_DIR));
    let armor = ArmorsFolderHandle::new(asset_server.load_folder(ARMOR_DIR));
    let injuries = InjuriesFolderHandle::new(asset_server.load_folder(INJURIES_DIR));
    let gangs = GangsFolderHandle::new(asset_server.load_folder(GANGS_DIR));
    // GTW-489 / GTW-494: the UUID-keyed v2 prefab fragments live under the `maps/` root. One
    // recursive `load_folder` fans every `*.prefab_v2.ron` member to the dedicated-extension
    // `RonAsset<PrefabSpecV2>` loader. This is the ONLY prefab load (the legacy flat-dir
    // `content/maps/*.prefab.ron` loader was retired).
    let prefabs_v2 = PrefabsV2FolderHandle::new(asset_server.load_folder(MAPS_V2_DIR));
    let terrain_model = TerrainModelFolderHandle::new(asset_server.load_folder(TERRAIN_MODEL_DIR));

    commands.insert_resource(LoadHandles {
        theme,
        fonts,
        situation,
        tuning,
        stat_tuning,
        weapons,
        melee_weapons,
        armor,
        injuries,
        gangs,
        prefabs_v2,
        terrain_model,
    });
}
