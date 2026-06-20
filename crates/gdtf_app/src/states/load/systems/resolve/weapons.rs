//! GTW-257: builds the name-keyed [`WeaponRegistry`] from the loaded `assets/weapons/`
//! folder.

use bevy::{
    asset::{LoadedFolder, RecursiveDependencyLoadState},
    prelude::{AssetServer, Assets, Commands, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::weapon::{WeaponName, WeaponRegistry, WeaponSpec};

use crate::states::load::resources::LoadHandles;

/// GTW-257: builds the name-keyed [`WeaponRegistry`] from the loaded
/// `assets/weapons/` folder, mirroring the fonts folder-load gating + the situation
/// resolve shape.
///
/// Called only while no [`WeaponRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the theme / tuning branches:
///
/// - Gates on the weapons folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every weapon `.ron`
///   IN the folder is loaded — the fonts-folder pattern). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`WeaponRegistry`] so `Load` always exits with one present and never hangs on a
///   bad folder (the ADR-0003 error-path safety-net; a battle then fails closed with
///   [`WeaponNotFound`](gdtf_battle_sim::situation::BattleSetupError) rather than
///   crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<WeaponSpec>`, reads its [`WeaponSpec`] out of the
///   `Assets<RonAsset<WeaponSpec>>` collection, keys it by the asset path's file
///   STEM with the dedicated `.weapon` infix stripped (so `stub_pistol.weapon.ron` keys
///   `stub_pistol` — the weapon KEY), and inserts every `(WeaponName, WeaponSpec)` into
///   the registry. If ANY member spec is not yet in the collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries
///   next frame — so a partial / empty registry is never published while the folder
///   is non-empty. The registry holds the specs BY VALUE, so they survive the folder
///   handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_weapons(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    weapon_specs: &Assets<RonAsset<WeaponSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.weapons);

    // Failure path: a bad/missing weapons folder must not hang the app. Warn and
    // insert an EMPTY registry so Load always exits with one present (a battle then
    // fails closed with WeaponNotFound rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `weapons` folder failed to load; inserting an empty WeaponRegistry \
             (battles will fail closed on a missing weapon key)",
        );
        commands.insert_resource(WeaponRegistry::default());
        return;
    }

    // Success path: once every weapon file in the folder is loaded, read the
    // LoadedFolder's member handles and build the name-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(folder) = folders.get(&*handles.weapons) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while the WeaponRegistry is still absent).
            return;
        };

        let mut registry = WeaponRegistry::default();
        for untyped in &folder.handles {
            // Type the untyped member handle as a RonAsset<WeaponSpec> and read its
            // spec out of the collection.
            let handle = untyped
                .clone()
                .typed_debug_checked::<RonAsset<WeaponSpec>>();
            let Some(spec) = weapon_specs.get(&handle) else {
                // One-frame loaded-but-not-yet-in-collection race: a member spec is not
                // in the collection yet. RETURN (do NOT insert a partial registry) so we
                // re-poll next frame — the WeaponRegistry stays absent, keeping this
                // branch alive until every member resolves.
                return;
            };
            // Key by the asset path's file STEM with the dedicated `.weapon` infix
            // stripped: `stub_pistol.weapon.ron`'s `file_stem()` is `stub_pistol.weapon`,
            // whose weapon KEY is `stub_pistol`. A handle with no resolvable path / stem is
            // skipped defensively (it would carry no usable key).
            let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
                path.path()
                    .file_stem()
                    .map(|stem| weapon_key_from_stem(&stem.to_string_lossy()))
            }) else {
                continue;
            };
            registry.insert(WeaponName::new(stem), (**spec).clone());
        }

        // Insert the built registry — like GdtfTheme / CombatTuning it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the battle reads it.
        commands.insert_resource(registry);
    }
}

/// The weapon KEY for a loaded weapon file's stem — the stem with the dedicated
/// `.weapon` infix stripped (GTW-257).
///
/// A weapon file is `<key>.weapon.ron`; Bevy's `file_stem()` yields `<key>.weapon`,
/// so the KEY (the [`WeaponName`] a [`GangerSpawn`](gdtf_battle_sim::situation::GangerSpawn)
/// references) is that stem minus a trailing `.weapon`. A stem without the infix is
/// returned unchanged (defensive — keeps a mis-named file's key its plain stem).
fn weapon_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".weapon").unwrap_or(stem).to_owned()
}
