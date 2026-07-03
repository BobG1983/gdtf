//! `OnEnter(AppState::Load)`: start the real async asset loads.

use bevy::prelude::*;
use gdtf_assets::{HotRonHandle, RonAsset};
use gdtf_ui::theme::GdtfThemeSpec;

use crate::states::load::resources::{
    AttachmentsFolderHandle, FontFolderHandle, InjuriesFolderHandle, LoadHandles,
    PrefabsFolderHandle,
};

/// Path of the loose theme RON, relative to the asset source root.
const THEME_RON_PATH: &str = "core_tuning/ui_theme.tuning.ron";

/// Path of the loose fonts folder, relative to the asset source root.
const FONTS_FOLDER_PATH: &str = "fonts";

/// Path of the loose attachments folder, relative to the asset source root (GTW-549 PHASE 1
/// — the per-attachment `assets/content/attachments/*.attachment.ron` items the
/// [`AttachmentRegistry`](gdtf_battle_sim::weapon::AttachmentRegistry) is built from). Its
/// OWN folder + the dedicated `attachment.ron` compound extension keep the `.ron` loader
/// dispatch unambiguous (attachments only) — the weapon/armor/gang precedent.
const ATTACHMENTS_DIR: &str = "content/attachments";

/// Path of the loose injuries folder, relative to the asset source root (GTW-437 —
/// the per-injury `assets/content/injuries/**/*.injury.ron` files + the per-part
/// `content/injuries/weighting/*.weighting.ron` files the registry + tables are built
/// from). One recursive folder carrying both asset types; the dedicated compound
/// extensions (`injury.ron` / `weighting.ron`) keep the `.ron` loader dispatch unambiguous.
const INJURIES_DIR: &str = "content/injuries";

/// Path of the maps (prefab) folder, relative to the asset source root (GTW-489 — child
/// T05c of the GTW-476 data-model refactor; the per-prefab
/// `assets/content/maps/<theme>/<size>/*.prefab.ron` fragments the `PrefabRegistry` is
/// built from). One recursive `load_folder` walks the whole `<theme>/<size>/` tree, and
/// the dedicated `prefab.ron` compound extension
/// keeps the `.ron` loader dispatch unambiguous. GTW-494 retired the legacy
/// `content/maps/*.prefab.ron` loader, so this is the ONLY prefab load in the Load flow.
const MAPS_DIR: &str = "content/maps";

/// Kicks off the theme-RON load and the BESPOKE folder preloads, storing their
/// typed handles.
///
/// Loads `core_tuning/ui_theme.tuning.ron` as a `RonAsset<GdtfThemeSpec>` (through
/// the GTW-136 loader) and preloads the entire `fonts` folder via
/// [`AssetServer::load_folder`](bevy::asset::AssetServer::load_folder) (GTW-149 —
/// loads ALL fonts up front so any font a sub-theme selects, override or default,
/// is resident), AND preloads the bespoke content folders — `content/attachments`
/// (GTW-549), `content/injuries` (GTW-437, one folder → two resources), and
/// `content/maps` (GTW-489, the UUID-keyed
/// [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) multimap) — then
/// inserts the Load-scoped [`LoadHandles`] resource the poll/resolve system reads.
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
/// GTW-570: the seven FOLDER content families (ranged/melee weapons, armor,
/// fields, gangs, terrain + theme defs) no longer kick off here either — each is
/// one `register_content_family` ext call in the Load plugin (its `Startup`
/// kick-off stores the persistent generic
/// [`ContentFolderHandle`](gdtf_assets::ContentFolderHandle), its gated resolve
/// publishes the registry the Load gate still requires, and its redrive
/// hot-reloads it). Only the theme + fonts and the bespoke folders (attachments,
/// injuries, prefabs — the declared GTW-570 exclusions) remain in this kick-off.
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
    // GTW-549 PHASE 1: the data-driven attachment items load through their OWN folder handle
    // so the AttachmentRegistry builds from the `.attachment.ron` members only.
    let attachments = AttachmentsFolderHandle::new(asset_server.load_folder(ATTACHMENTS_DIR));
    let injuries = InjuriesFolderHandle::new(asset_server.load_folder(INJURIES_DIR));
    // GTW-489 / GTW-494: the UUID-keyed prefab fragments live under the `maps/` root. One
    // recursive `load_folder` fans every `*.prefab.ron` member to the dedicated-extension
    // `RonAsset<PrefabSpec>` loader. This is the ONLY prefab load (the legacy flat-dir
    // `content/maps/*.prefab.ron` loader was retired).
    let prefabs = PrefabsFolderHandle::new(asset_server.load_folder(MAPS_DIR));

    commands.insert_resource(LoadHandles {
        theme,
        fonts,
        attachments,
        injuries,
        prefabs,
    });
}
