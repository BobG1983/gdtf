//! GTW-269: builds the name-keyed [`ArmorRegistry`] from the loaded `assets/armor/`
//! folder.

use bevy::{
    asset::{LoadedFolder, RecursiveDependencyLoadState},
    prelude::{AssetServer, Assets, Commands, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

use crate::scenes::load::resources::LoadHandles;

/// GTW-269: builds the name-keyed [`ArmorRegistry`] from the loaded
/// `assets/armor/` folder, mirroring the GTW-257 weapons resolve shape exactly
/// (the armor mirror of [`resolve_weapons`](super::weapons::resolve_weapons)).
///
/// Called only while no [`ArmorRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the theme / tuning / weapons / situation
/// branches:
///
/// - Gates on the armor folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every armor `.ron`
///   IN the folder is loaded — the fonts/weapons-folder pattern). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`ArmorRegistry`] so `Load` always exits with one present and never hangs on a
///   bad folder (the ADR-0003 error-path safety-net; the later consumption slice
///   then fails closed on a missing armor key rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<ArmorSpec>`, reads its [`ArmorSpec`] out of the
///   `Assets<RonAsset<ArmorSpec>>` collection, keys it by the asset path's file
///   STEM with the dedicated `.armor` infix stripped (so `flak.armor.ron` keys
///   `flak` — the armor KEY), and inserts every `(ArmorName, ArmorSpec)` into
///   the registry. If ANY member spec is not yet in the collection (the one-frame
///   loaded-but-not-yet-in-collection race), it returns WITHOUT inserting and retries
///   next frame — so a partial / empty registry is never published while the folder
///   is non-empty. The registry holds the specs BY VALUE, so they survive the folder
///   handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_armor(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    armor_specs: &Assets<RonAsset<ArmorSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.armor);

    // Failure path: a bad/missing armor folder must not hang the app. Warn and
    // insert an EMPTY registry so Load always exits with one present (the later
    // consumption slice then fails closed on a missing armor key rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `armor` folder failed to load; inserting an empty ArmorRegistry \
             (battles will fail closed on a missing armor key)",
        );
        commands.insert_resource(ArmorRegistry::default());
        return;
    }

    // Success path: once every armor file in the folder is loaded, read the
    // LoadedFolder's member handles and build the name-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(folder) = folders.get(&*handles.armor) else {
            // Loaded-but-not-yet-in-collection — retry next frame (the system stays
            // alive while the ArmorRegistry is still absent).
            return;
        };

        let mut registry = ArmorRegistry::default();
        for untyped in &folder.handles {
            // Type the untyped member handle as a RonAsset<ArmorSpec> and read its
            // spec out of the collection.
            let handle = untyped.clone().typed_debug_checked::<RonAsset<ArmorSpec>>();
            let Some(spec) = armor_specs.get(&handle) else {
                // One-frame loaded-but-not-yet-in-collection race: a member spec is not
                // in the collection yet. RETURN (do NOT insert a partial registry) so we
                // re-poll next frame — the ArmorRegistry stays absent, keeping this
                // branch alive until every member resolves.
                return;
            };
            // Key by the asset path's file STEM with the dedicated `.armor` infix
            // stripped: `flak.armor.ron`'s `file_stem()` is `flak.armor`, whose armor
            // KEY is `flak`. A handle with no resolvable path / stem is skipped
            // defensively (it would carry no usable key).
            let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
                path.path()
                    .file_stem()
                    .map(|stem| armor_key_from_stem(&stem.to_string_lossy()))
            }) else {
                continue;
            };
            registry.insert(ArmorName::new(stem), **spec);
        }

        // Insert the built registry — like the WeaponRegistry it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the battle reads it
        // (slice C).
        commands.insert_resource(registry);
    }
}

/// The armor KEY for a loaded armor file's stem — the stem with the dedicated
/// `.armor` infix stripped (GTW-269).
///
/// An armor file is `<key>.armor.ron`; Bevy's `file_stem()` yields `<key>.armor`,
/// so the KEY (the [`ArmorName`] a ganger references) is that stem minus a trailing
/// `.armor`. A stem without the infix is returned unchanged (defensive — keeps a
/// mis-named file's key its plain stem).
fn armor_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".armor").unwrap_or(stem).to_owned()
}
