//! GTW-394: builds the name-keyed [`TerrainRegistry`] from the loaded `assets/terrain/`
//! folder, plus the LIVE hot-reload that rebuilds it on a `*.terrain.ron` edit.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::terrain::piece::{TerrainName, TerrainRegistry, TerrainSpec};

use crate::states::load::resources::{ActiveTerrainFolderHandle, LoadHandles};

/// GTW-394: builds the name-keyed [`TerrainRegistry`] from the loaded
/// `assets/terrain/` folder, mirroring the GTW-269 armor resolve shape exactly
/// (the terrain mirror of [`resolve_armor`](super::armor::resolve_armor)).
///
/// Called only while no [`TerrainRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the other load branches:
///
/// - Gates on the terrain folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every terrain `.ron`
///   IN the folder is loaded — the weapons/armor-folder pattern). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`TerrainRegistry`] so `Load` always exits with one present and never hangs on
///   a bad folder (the ADR-0003 error-path safety-net; the later consumption slice
///   then fails closed on a missing terrain key rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<TerrainSpec>`, reads its [`TerrainSpec`] out of the
///   `Assets<RonAsset<TerrainSpec>>` collection, keys it by the asset path's file
///   STEM with the dedicated `.terrain` infix stripped (so
///   `deck_floor.terrain.ron` keys `deck_floor` — the terrain KEY), and inserts
///   every `(TerrainName, TerrainSpec)` into the registry. If ANY member spec is
///   not yet in the collection (the one-frame loaded-but-not-yet-in-collection
///   race), it returns WITHOUT inserting and retries next frame — so a partial /
///   empty registry is never published while the folder is non-empty. The registry
///   holds the specs BY VALUE, so they survive the folder handle being dropped on
///   `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_terrain(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    terrain_specs: &Assets<RonAsset<TerrainSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.terrain);

    // Failure path: a bad/missing terrain folder must not hang the app. Warn and
    // insert an EMPTY registry so Load always exits with one present (the later
    // consumption slice then fails closed on a missing terrain key rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `terrain` folder failed to load; inserting an empty TerrainRegistry \
             (battles will fail closed on a missing terrain key)",
        );
        commands.insert_resource(TerrainRegistry::default());
        return;
    }

    // Success path: once every terrain file in the folder is loaded, read the
    // LoadedFolder's member handles and build the name-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_terrain_registry(asset_server, folders, terrain_specs, &handles.terrain)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) — retry
            // next frame (the system stays alive while the TerrainRegistry is absent).
            return;
        };

        // Insert the built registry — like the WeaponRegistry/ArmorRegistry it persists
        // past OnExit(Load) (it is NOT removed in cleanup), because the battle reads it.
        commands.insert_resource(registry);
        // GTW-394: insert the PERSISTENT folder handle alongside the registry — it
        // survives OnExit(Load) so the live hot-reload handler can re-enumerate the
        // folder's member handles to rebuild the registry on a `*.terrain.ron` edit,
        // and holding it keeps every member terrain asset loaded for the file-watcher.
        commands.insert_resource(ActiveTerrainFolderHandle::new((*handles.terrain).clone()));
    }
}

/// Build the name-keyed [`TerrainRegistry`] from a loaded `terrain/` [`LoadedFolder`],
/// or [`None`] if the folder (or any member spec) is not yet in its collection — the
/// terrain mirror of `build_armor_registry` (in the sibling `armor` module).
///
/// Shared by [`resolve_terrain`] (the one-time `Load`-state build) and the GTW-394
/// [`redrive_terrain_on_asset_event`] (the live rebuild on a hot edit), so both build
/// the registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<TerrainSpec>`, read its [`TerrainSpec`] out of the collection, and key
/// it by the asset path's file STEM with the dedicated `.terrain` infix stripped.
/// Returns [`None`] (do NOT publish a partial registry) if the folder or any member
/// spec is not yet in its collection — the caller retries next frame.
fn build_terrain_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    terrain_specs: &Assets<RonAsset<TerrainSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<TerrainRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = TerrainRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<TerrainSpec> and read its spec
        // out of the collection.
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<TerrainSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: a member spec is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let spec = terrain_specs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.terrain` infix stripped:
        // `deck_floor.terrain.ron`'s `file_stem()` is `deck_floor.terrain`, whose terrain
        // KEY is `deck_floor`. A handle with no resolvable path / stem is skipped
        // defensively (it would carry no usable key).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| terrain_key_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        registry.insert(TerrainName::new(stem), (**spec).clone());
    }
    Some(registry)
}

/// `Update`: rebuild the [`TerrainRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/terrain/*.terrain.ron` — the GTW-394 LIVE terrain hot-reload, the terrain
/// mirror of `redrive_armor_on_asset_event`.
///
/// A folder load fans out into one `RonAsset<TerrainSpec>` asset PER file, and a hot
/// edit fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member
/// asset (not the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<TerrainSpec>>::Modified` and rebuilds the whole registry from
/// the PERSISTENT [`ActiveTerrainFolderHandle`]'s member handles via
/// [`build_terrain_registry`] — the SAME builder the one-time resolve uses.
/// Overwriting via [`ResMut`] marks the registry changed, so the next battle setup
/// resolves against the edited specs WITHOUT a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`TerrainRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional
/// [`AssetServer`] / folder handle / `Assets` / [`TerrainRegistry`] borrows.
pub(in crate::states::load) fn redrive_terrain_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<TerrainSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActiveTerrainFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    terrain_specs: Option<Res<Assets<RonAsset<TerrainSpec>>>>,
    registry: Option<ResMut<TerrainRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(terrain_specs),
        Some(mut registry),
    ) = (
        asset_server,
        folder_handle,
        folders,
        terrain_specs,
        registry,
    )
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified terrain member — a single rebuild from the latest
    // in-memory specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_terrain_registry(&asset_server, &folders, &terrain_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "terrain hot-reload: rebuilt TerrainRegistry from `assets/terrain/` ({} pieces)",
        registry.len(),
    );
}

/// The terrain KEY for a loaded terrain file's stem — the stem with the dedicated
/// `.terrain` infix stripped (GTW-394).
///
/// A terrain file is `<key>.terrain.ron`; Bevy's `file_stem()` yields `<key>.terrain`,
/// so the KEY (the [`TerrainName`] a generated cell references) is that stem minus a
/// trailing `.terrain`. A stem without the infix is returned unchanged (defensive —
/// keeps a mis-named file's key its plain stem).
fn terrain_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".terrain").unwrap_or(stem).to_owned()
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
    use gdtf_battle_sim::terrain::piece::{
        FootfallSound, TerrainGraphicKey, TerrainKindSpec, TerrainName, TerrainRegistry,
        TerrainSpec,
    };

    use super::redrive_terrain_on_asset_event;
    use crate::states::load::{
        resources::ActiveTerrainFolderHandle,
        systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A deck-floor-shaped `TerrainSpec` with the given `move_cost` — parsed from
    /// inline RON so the test does not hand-assemble the nested payload. Returns
    /// `None` (assert-fail) on a parse error rather than a denied `unwrap`.
    fn floor_spec(move_cost: u8) -> Option<TerrainSpec> {
        let ron = format!(
            "(graphic: \"floor\", footfall: \"footfall_metal\", kind: Floor((move_cost: {move_cost})))",
        );
        let parsed = ron::de::from_str::<TerrainSpec>(&ron);
        assert!(
            parsed.is_ok(),
            "terrain fixture must parse: {:?}",
            parsed.as_ref().err()
        );
        parsed.ok()
    }

    /// A headless app with the real hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
    /// (registers `Assets<RonAsset<TerrainSpec>>`, `Assets<LoadedFolder>`, and the
    /// `AssetEvent` message buffers), the `TerrainSpec` RON loader, and the redrive
    /// system in `Update`.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<TerrainSpec>(vec!["terrain.ron"])
            .add_systems(Update, redrive_terrain_on_asset_event);
        app
    }

    /// Register a member terrain asset at `path` (so `AssetServer::get_path` resolves
    /// its stem) carrying `spec`, and return its typed handle. `load(path)` registers
    /// the path→id map synchronously; `insert` then provides the in-memory spec (the
    /// async load finds no file under the test CWD, so it never clobbers this).
    fn add_member(
        app: &mut App,
        path: &'static str,
        spec: TerrainSpec,
    ) -> Handle<RonAsset<TerrainSpec>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<TerrainSpec>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<TerrainSpec>>>()
            .insert(handle.id(), RonAsset::new(spec));
        assert!(inserted.is_ok(), "member spec insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(
        app: &mut App,
        members: &[Handle<RonAsset<TerrainSpec>>],
    ) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// B2 (terrain): a `Modified` for a member `*.terrain.ron` REBUILDS the
    /// `TerrainRegistry` from the folder's members — keyed by file stem — reflecting
    /// the edited spec.
    ///
    /// Pin-discriminating: dropping the rebuild leaves the OLD `move_cost`; mis-keying
    /// drops the `deck_floor` entry. The `move_cost` fixture values (4 → 7) are arbitrary
    /// discriminators — NOT shipped tuning pins.
    #[test]
    fn modified_member_rebuilds_terrain_registry() {
        let mut app = app();
        let Some(original) = floor_spec(4) else {
            return;
        };
        let member = add_member(&mut app, "terrain/deck_floor.terrain.ron", original);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveTerrainFolderHandle::new(folder));
        // A stale baseline registry (empty) the rebuild must overwrite.
        app.world_mut().insert_resource(TerrainRegistry::default());
        app.update();

        // Hot-edit the member spec to a DISTINCT move_cost, fire Modified, rebuild.
        let Some(edited) = floor_spec(7) else { return };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<TerrainSpec>>>()
            .get_mut(&member)
        {
            **asset = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let key = TerrainName::new("deck_floor".to_owned());
        let rebuilt_move_cost = app
            .world()
            .get_resource::<TerrainRegistry>()
            .and_then(|r| r.spec(&key))
            .and_then(|s| {
                if let TerrainKindSpec::Floor(f) = &s.kind {
                    Some(*f.move_cost)
                } else {
                    None
                }
            });
        assert_eq!(
            rebuilt_move_cost,
            Some(7),
            "the hot-reload must rebuild the registry, keyed by stem, with the edited move_cost",
        );
    }

    /// B2 (terrain): a hot-reload of a terrain member fires the Part C `info!` line
    /// naming what reloaded. Run via `run_system_once` on the calling thread so the
    /// thread-local `tracing` capture sees the emission.
    ///
    /// Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn terrain_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(spec) = floor_spec(4) else { return };
        let member = add_member(&mut app, "terrain/deck_floor.terrain.ron", spec);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveTerrainFolderHandle::new(folder));
        app.world_mut().insert_resource(TerrainRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_terrain_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("terrain hot-reload") && line.contains("TerrainRegistry")),
            "the terrain hot-reload must emit an info! line naming what reloaded; captured: {captured:?}",
        );
    }

    // Suppress unused-import warnings for types referenced only via let-binding
    // in the discriminating assertions — these are intentional test fixtures.
    const _: fn() = || {
        let _: FootfallSound;
        let _: TerrainGraphicKey;
    };
}
