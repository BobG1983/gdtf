//! GTW-269: builds the name-keyed [`ArmorRegistry`] from the loaded `assets/armor/`
//! folder, plus the GTW-374 LIVE hot-reload that rebuilds it on a `*.armor.ron` edit.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

use crate::states::load::resources::{ActiveArmorFolderHandle, LoadHandles};

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
///   STEM with the dedicated `.armor` infix stripped (so `flak_vest.armor.ron` keys
///   `flak_vest` — the armor KEY), and inserts every `(ArmorName, ArmorSpec)` into
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
        let Some(registry) =
            build_armor_registry(asset_server, folders, armor_specs, &handles.armor)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) — retry
            // next frame (the system stays alive while the ArmorRegistry is absent).
            return;
        };

        // Insert the built registry — like the WeaponRegistry it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the battle reads it.
        commands.insert_resource(registry);
        // GTW-374: insert the PERSISTENT folder handle alongside the registry — it
        // survives OnExit(Load) so the live hot-reload handler can re-enumerate the
        // folder's member handles to rebuild the registry on a `*.armor.ron` edit, and
        // holding it keeps every member armor asset loaded for the file-watcher.
        commands.insert_resource(ActiveArmorFolderHandle::new((*handles.armor).clone()));
    }
}

/// Build the name-keyed [`ArmorRegistry`] from a loaded `armor/` [`LoadedFolder`], or
/// [`None`] if the folder (or any member spec) is not yet in its collection — the armor
/// mirror of `build_weapon_registry` (in the sibling `weapons` module).
///
/// Shared by [`resolve_armor`] (the one-time `Load`-state build) and the GTW-374
/// [`redrive_armor_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<ArmorSpec>`, read its [`ArmorSpec`] out of the collection, and key it by
/// the asset path's file STEM with the dedicated `.armor` infix stripped. Returns
/// [`None`] (do NOT publish a partial registry) if the folder or any member spec is not
/// yet in its collection — the caller retries next frame.
fn build_armor_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    armor_specs: &Assets<RonAsset<ArmorSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<ArmorRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = ArmorRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<ArmorSpec> and read its spec out
        // of the collection.
        let handle = untyped.clone().typed_debug_checked::<RonAsset<ArmorSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: a member spec is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let spec = armor_specs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.armor` infix stripped:
        // `flak_vest.armor.ron`'s `file_stem()` is `flak_vest.armor`, whose armor KEY is
        // `flak_vest`. A handle with no resolvable path / stem is skipped defensively (it
        // would carry no usable key).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| armor_key_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        registry.insert(ArmorName::new(stem), **spec);
    }
    Some(registry)
}

/// `Update`: rebuild the [`ArmorRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/armor/*.armor.ron` — the GTW-374 LIVE armor hot-reload, the armor mirror of
/// `redrive_weapons_on_asset_event` (in the sibling `weapons` module).
///
/// A folder load fans out into one `RonAsset<ArmorSpec>` asset PER file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset
/// (not the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<ArmorSpec>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActiveArmorFolderHandle`]'s member handles via [`build_armor_registry`]
/// — the SAME builder the one-time resolve uses. Overwriting via [`ResMut`] marks the
/// registry changed, so the next battle setup resolves against the edited specs WITHOUT
/// a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`ArmorRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional
/// [`AssetServer`] / folder handle / `Assets` / [`ArmorRegistry`] borrows.
pub(in crate::states::load) fn redrive_armor_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<ArmorSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveArmorFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    armor_specs: Option<Res<Assets<RonAsset<ArmorSpec>>>>,
    registry: Option<ResMut<ArmorRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(armor_specs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, armor_specs, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified armor member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) = build_armor_registry(&asset_server, &folders, &armor_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "armor hot-reload: rebuilt ArmorRegistry from `assets/armor/` ({} armor suits)",
        registry.len(),
    );
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

#[cfg(test)]
mod test {
    use bevy::{
        MinimalPlugins,
        asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
        ecs::system::RunSystemOnce,
        prelude::*,
    };
    use gdtf_assets::{RonAsset, RonAssetAppExt};
    use gdtf_battle_sim::armor::{ArmorName, ArmorRegistry, ArmorSpec};

    use super::redrive_armor_on_asset_event;
    use crate::states::load::{
        resources::ActiveArmorFolderHandle, systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A flak_vest-shaped `ArmorSpec` whose torso `protection` is `protection` — parsed
    /// from inline RON. Returns `None` (assert-fail) on a parse error rather than a
    /// denied `unwrap`.
    fn armor_spec(torso_protection: i32) -> Option<ArmorSpec> {
        let ron = format!(
            "(head:      (floor: 2, protection: 3, integrity: 50, hardness: 1, armor_type: Flak), \
              torso:     (floor: 2, protection: {torso_protection}, integrity: 60, hardness: 1, armor_type: Flak), \
              left_arm:  (floor: 1, protection: 2, integrity: 45, hardness: 1, armor_type: Flak), \
              right_arm: (floor: 1, protection: 2, integrity: 45, hardness: 1, armor_type: Flak), \
              left_leg:  (floor: 1, protection: 2, integrity: 50, hardness: 1, armor_type: Flak), \
              right_leg: (floor: 1, protection: 2, integrity: 50, hardness: 1, armor_type: Flak))",
        );
        let parsed = ron::de::from_str::<ArmorSpec>(&ron);
        assert!(
            parsed.is_ok(),
            "armor fixture must parse: {:?}",
            parsed.as_ref().err()
        );
        parsed.ok()
    }

    /// A headless app with the real hot-reload wiring (the armor mirror of the weapons
    /// test app).
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<ArmorSpec>(vec!["armor.ron"])
            .add_systems(Update, redrive_armor_on_asset_event);
        app
    }

    /// Register a member armor asset at `path` carrying `spec`, returning its handle —
    /// the armor mirror of the weapons test's `add_member`.
    fn add_member(
        app: &mut App,
        path: &'static str,
        spec: ArmorSpec,
    ) -> Handle<RonAsset<ArmorSpec>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<ArmorSpec>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<ArmorSpec>>>()
            .insert(handle.id(), RonAsset(spec));
        assert!(inserted.is_ok(), "member spec insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(app: &mut App, members: &[Handle<RonAsset<ArmorSpec>>]) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// B3: a `Modified` for a member `*.armor.ron` REBUILDS the `ArmorRegistry` from the
    /// folder's members — keyed by file stem — reflecting the edited spec.
    ///
    /// Pin-discriminating: dropping the rebuild leaves the OLD torso protection.
    #[test]
    fn modified_member_rebuilds_armor_registry() {
        let mut app = app();
        let Some(original) = armor_spec(8) else {
            return;
        };
        let member = add_member(&mut app, "armor/flak_vest.armor.ron", original);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveArmorFolderHandle::new(folder));
        app.world_mut().insert_resource(ArmorRegistry::default());
        app.update();

        // Hot-edit the member to a DISTINCT torso protection, fire Modified, rebuild.
        let Some(edited) = armor_spec(4) else { return };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<ArmorSpec>>>()
            .get_mut(&member)
        {
            asset.0 = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let key = ArmorName::new("flak_vest".to_owned());
        let protection = app
            .world()
            .get_resource::<ArmorRegistry>()
            .and_then(|r| r.spec(&key).map(|s| *s.torso.protection));
        assert_eq!(
            protection,
            Some(4),
            "the hot-reload must rebuild the registry, keyed by stem, with the edited protection",
        );
    }

    /// B3: a hot-reload of an armor member fires the Part C `info!` line naming what
    /// reloaded. Run via `run_system_once` on the calling thread so the thread-local
    /// `tracing` capture sees the emission (the schedule executor may run on a worker
    /// thread the capture would miss).
    ///
    /// Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn armor_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(spec) = armor_spec(8) else { return };
        let member = add_member(&mut app, "armor/flak_vest.armor.ron", spec);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveArmorFolderHandle::new(folder));
        app.world_mut().insert_resource(ArmorRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_armor_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("armor hot-reload") && line.contains("ArmorRegistry")),
            "the armor hot-reload must emit an info! line naming what reloaded; captured: {captured:?}",
        );
    }
}
