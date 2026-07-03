//! `OnEnter(AppState::Load)`: start the real async asset loads.

use bevy::prelude::*;
use gdtf_assets::{HotRonHandle, RonAsset};
use gdtf_ui::theme::GdtfThemeSpec;

use crate::states::load::resources::{
    ArmorsFolderHandle, AttachmentsFolderHandle, FieldsFolderHandle, FontFolderHandle,
    GangsFolderHandle, InjuriesFolderHandle, LoadHandles, MeleeWeaponsFolderHandle,
    PrefabsFolderHandle, TerrainModelFolderHandle, WeaponsFolderHandle,
};

/// Path of the loose theme RON, relative to the asset source root.
const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// Path of the loose fonts folder, relative to the asset source root.
const FONTS_FOLDER_PATH: &str = "fonts";

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

/// Path of the loose attachments folder, relative to the asset source root (GTW-549 PHASE 1
/// — the per-attachment `assets/content/attachments/*.attachment.ron` items the
/// [`AttachmentRegistry`](gdtf_battle_sim::weapon::AttachmentRegistry) is built from). Its
/// OWN folder + the dedicated `attachment.ron` compound extension keep the `.ron` loader
/// dispatch unambiguous (attachments only) — the weapon/armor/gang precedent.
const ATTACHMENTS_DIR: &str = "content/attachments";

/// Path of the loose armor folder, relative to the asset source root (GTW-269 —
/// the per-armor `assets/content/armor/*.ron` files the registry is built from). Its OWN
/// folder so the `.ron` loader dispatch is unambiguous (armor only, no weapons).
const ARMOR_DIR: &str = "content/armor";

/// Path of the loose area-damage-fields folder, relative to the asset source root (GTW-545 —
/// the per-field-type `assets/content/fields/*.field.ron` catalog entries the
/// [`FieldDefRegistry`](gdtf_battle_sim::FieldDefRegistry) is built from). Its OWN folder + the
/// dedicated `field.ron` compound extension keep the `.ron` loader dispatch unambiguous (fields
/// only) — the armor precedent.
const FIELDS_DIR: &str = "content/fields";

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

/// Path of the maps (prefab) folder, relative to the asset source root (GTW-489 — child
/// T05c of the GTW-476 data-model refactor; the per-prefab
/// `assets/content/maps/<theme>/<size>/*.prefab.ron` fragments the `PrefabRegistry` is
/// built from). One recursive `load_folder` walks the whole `<theme>/<size>/` tree, and
/// the dedicated `prefab.ron` compound extension
/// keeps the `.ron` loader dispatch unambiguous. GTW-494 retired the legacy
/// `content/maps/*.prefab.ron` loader, so this is the ONLY prefab load in the Load flow.
const MAPS_DIR: &str = "content/maps";

/// Path of the loose per-theme terrain-model folder, relative to the asset source root
/// (GTW-487 — the GTW-484/485 UUID-keyed terrain + theme defs under the per-theme layout
/// `content/terrain/<theme>/<tile>.terrain_def.ron` +
/// `content/terrain/<theme>/<theme>.terrain_theme.ron`; GTW-562 moved the root under
/// `content/` so every authored terrain file lives in the canonical content tree).
/// One recursive `load_folder` walks every `<theme>/` subfolder, and the dedicated compound
/// extensions (`terrain_def.ron` / `terrain_theme.ron`) keep the `.ron` loader dispatch
/// unambiguous. GTW-494 retired the legacy flat-dir loaders, so this is the ONLY
/// terrain / theme load in the Load flow.
const TERRAIN_MODEL_DIR: &str = "content/terrain";

/// Kicks off the theme-RON load and the fonts-folder preload, storing their typed
/// handles.
///
/// Loads `core_tuning/ui_theme.tuning.ron` as a `RonAsset<GdtfThemeSpec>` (through
/// the GTW-136 loader) and preloads the entire `fonts` folder via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder) (GTW-149 —
/// loads ALL fonts up front so any font a sub-theme selects, override or default,
/// is resident), AND preloads the `content/weapons/ranged` leaf folder via `load_folder` (GTW-257; GTW-505
/// split the tree into `ranged/` + `melee/` — every
/// `assets/content/weapons/ranged/*.weapon.ron`, each a `RonAsset<WeaponSpec>`, so the poll/resolve
/// system can build the name-keyed
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry))
/// AND preloads the entire `content/armor` folder via `load_folder` (GTW-269 — every
/// `assets/content/armor/*.ron`, each a `RonAsset<ArmorSpec>`, so the poll/resolve
/// system can build the name-keyed [`ArmorRegistry`](gdtf_battle_sim::armor::ArmorRegistry))
/// AND preloads the per-theme `content/terrain` folder via `load_folder` (GTW-487 — every
/// `content/terrain/<theme>/*.terrain_def.ron` + `*.terrain_theme.ron`, the UUID-keyed
/// [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) +
/// [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) the sim + procgen +
/// presenter consume) AND preloads the `content/maps` folder via `load_folder` (GTW-489 — every
/// `content/maps/<theme>/<size>/*.prefab.ron`, the UUID-keyed
/// [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) the procgen pipeline packs),
/// then inserts the Load-scoped [`LoadHandles`] resource the poll/resolve system
/// reads.
///
/// GTW-564: the four single-file RON chains (`content/situations/skirmish.ron`,
/// `core_tuning/combat.tuning.ron`, `core_tuning/stat.tuning.ron`,
/// `core_tuning/procgen.tuning.ron`) no longer kick off here — each is one
/// generic hot-RON ext call in the Load plugin (its `Startup` kick-off stores
/// the persistent [`HotRonHandle`], its gated resolve publishes the resource
/// the Load gate still requires, and its redrive hot-reloads it). The THEME
/// kick-off stays bespoke because its resolve pairs with the fonts folder (the
/// C7 record); it now mints the generic [`HotRonHandle`] directly.
///
/// GTW-494 (child T08 of GTW-476): the OLD FLAT-DIR terrain / theme / map folder loads were
/// RETIRED — the per-theme `content/terrain` model + the `content/maps` prefab loads above
/// are the ONLY terrain / theme / prefab loads in the Load flow.
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

    let theme = HotRonHandle::new(asset_server.load::<RonAsset<GdtfThemeSpec>>(THEME_RON_PATH));
    let fonts = FontFolderHandle::new(asset_server.load_folder(FONTS_FOLDER_PATH));
    let weapons = WeaponsFolderHandle::new(asset_server.load_folder(WEAPONS_DIR));
    // GTW-505: the sibling melee-weapons folder loads through its OWN folder handle so the
    // `MeleeWeaponRegistry` builds from the `.melee_weapon.ron` members only.
    let melee_weapons = MeleeWeaponsFolderHandle::new(asset_server.load_folder(MELEE_WEAPONS_DIR));
    // GTW-549 PHASE 1: the data-driven attachment items load through their OWN folder handle
    // so the AttachmentRegistry builds from the `.attachment.ron` members only.
    let attachments = AttachmentsFolderHandle::new(asset_server.load_folder(ATTACHMENTS_DIR));
    let armor = ArmorsFolderHandle::new(asset_server.load_folder(ARMOR_DIR));
    // GTW-545: the area-damage-fields catalog folder loads through its OWN folder handle so
    // the FieldDefRegistry builds from the `.field.ron` members only.
    let fields = FieldsFolderHandle::new(asset_server.load_folder(FIELDS_DIR));
    let injuries = InjuriesFolderHandle::new(asset_server.load_folder(INJURIES_DIR));
    let gangs = GangsFolderHandle::new(asset_server.load_folder(GANGS_DIR));
    // GTW-489 / GTW-494: the UUID-keyed prefab fragments live under the `maps/` root. One
    // recursive `load_folder` fans every `*.prefab.ron` member to the dedicated-extension
    // `RonAsset<PrefabSpec>` loader. This is the ONLY prefab load (the legacy flat-dir
    // `content/maps/*.prefab.ron` loader was retired).
    let prefabs = PrefabsFolderHandle::new(asset_server.load_folder(MAPS_DIR));
    let terrain_model = TerrainModelFolderHandle::new(asset_server.load_folder(TERRAIN_MODEL_DIR));

    commands.insert_resource(LoadHandles {
        theme,
        fonts,
        weapons,
        melee_weapons,
        attachments,
        armor,
        fields,
        injuries,
        gangs,
        prefabs,
        terrain_model,
    });
}
