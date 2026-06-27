//! GTW-418: builds the per-`(theme, size, spawn-role)` [`PrefabRegistry`] from the loaded
//! `assets/content/maps/` folder, plus the LIVE hot-reload that rebuilds it on a
//! `*.prefab.ron` edit.
//!
//! The prefab mirror of the GTW-415 gangs resolve
//! ([`resolve_gangs`](super::gangs::resolve_gangs)): the `Load` flow preloads the
//! NESTED `assets/content/maps/<theme>/<size>/*.prefab.ron` folder (recursive, so every
//! prefab `.ron` under every theme/size subfolder is covered), this resolves it into the
//! [`PrefabRegistry`](gdtf_battle_sim::level::PrefabRegistry) the GTW-424 space-packing
//! assembler enumerates by `(theme, size, role)`. A prefab authoring ZERO edge openings
//! is REJECTED fail-closed (C6 — the typed
//! [`PrefabLoadError::NoEdgeOpening`](gdtf_battle_sim::level::PrefabLoadError) is logged
//! and the prefab EXCLUDED, never a panic), so the registry only ever holds connectable
//! prefabs.

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::level::{Prefab, PrefabName, PrefabRegistry, PrefabSpec};

use crate::states::load::resources::{ActivePrefabsFolderHandle, LoadHandles};

/// GTW-418: builds the per-`(theme, size, spawn-role)` [`PrefabRegistry`] from the loaded
/// `assets/content/maps/` folder, mirroring the GTW-415 gangs resolve shape exactly (the
/// prefab mirror of [`resolve_gangs`](super::gangs::resolve_gangs)).
///
/// Called only while no [`PrefabRegistry`] resource exists yet (the caller's own-absence
/// guard), independently of every other load branch:
///
/// - Gates on the maps folder's
///   [`RecursiveDependencyLoadState`]`::Loaded` (recursive, so every prefab `.ron` under
///   the nested `<theme>/<size>/` subfolders is loaded — the injuries-folder pattern). On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`PrefabRegistry`] so `Load` always exits with one present and never hangs on a bad
///   folder (the ADR-0003 error-path safety-net; the assembler then has no fragments to
///   pack rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<PrefabSpec>`, reads its [`PrefabSpec`] out of the
///   `Assets<RonAsset<PrefabSpec>>` collection, keys it by the asset path's file STEM with
///   the dedicated `.prefab` infix stripped (so `entry_room.prefab.ron` keys `entry_room`
///   — the prefab NAME), VALIDATES the C6 edge-opening invariant via [`Prefab::new`], and
///   buckets each validated prefab into the registry under its OWN spec's
///   `(theme, size, spawn_role)`. A prefab that FAILS validation (zero edge openings) is
///   EXCLUDED + `warn!`-logged (fail-closed, no panic). If ANY member spec is not yet in
///   the collection (the one-frame loaded-but-not-yet-in-collection race), it returns
///   WITHOUT inserting and retries next frame — so a partial / empty registry is never
///   published while the folder is non-empty. The registry holds the prefabs BY VALUE, so
///   they survive the folder handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_prefabs(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.prefabs);

    // Failure path: a bad/missing maps folder must not hang the app. Warn and insert an
    // EMPTY registry so Load always exits with one present (the assembler then has no
    // fragments rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `maps` folder failed to load; inserting an empty PrefabRegistry \
             (the assembler will have no fragments to pack)",
        );
        commands.insert_resource(PrefabRegistry::default());
        return;
    }

    // Success path: once every prefab file in the folder is loaded, read the LoadedFolder's
    // member handles and build the bucketed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) =
            build_prefab_registry(asset_server, folders, prefab_specs, &handles.prefabs)
        else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) — retry
            // next frame (the system stays alive while the PrefabRegistry is absent).
            return;
        };

        // Insert the built registry — like the other registries it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the assembler reads it.
        commands.insert_resource(registry);
        // GTW-418: insert the PERSISTENT folder handle alongside the registry — it
        // survives OnExit(Load) so the live hot-reload handler can re-enumerate the
        // folder's member handles to rebuild the registry on a `*.prefab.ron` edit, and
        // holding it keeps every member prefab asset loaded for the file-watcher.
        commands.insert_resource(ActivePrefabsFolderHandle::new((*handles.prefabs).clone()));
    }
}

/// Build the bucketed [`PrefabRegistry`] from a loaded `maps/` [`LoadedFolder`], or
/// [`None`] if the folder (or any member spec) is not yet in its collection — the prefab
/// mirror of `build_gang_registry` (in the sibling `gangs` module).
///
/// Shared by [`resolve_prefabs`] (the one-time `Load`-state build) and the GTW-418
/// [`redrive_prefabs_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<PrefabSpec>`, read its [`PrefabSpec`] out of the collection, key it by the
/// asset path's file STEM with the dedicated `.prefab` infix stripped, VALIDATE the C6
/// edge-opening invariant ([`Prefab::new`]), and bucket each VALID prefab by its own
/// `(theme, size, spawn_role)`. A prefab failing validation (zero edge openings) is
/// EXCLUDED + `warn!`-logged (fail-closed, no panic — C6). Returns [`None`] (do NOT
/// publish a partial registry) if the folder or any member spec is not yet in its
/// collection — the caller retries next frame.
fn build_prefab_registry(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    prefab_specs: &Assets<RonAsset<PrefabSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<PrefabRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = PrefabRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<PrefabSpec> and read its spec out
        // of the collection.
        let handle = untyped
            .clone()
            .typed_debug_checked::<RonAsset<PrefabSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: a member spec is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let spec = prefab_specs.get(&handle)?;
        // Key by the asset path's file STEM with the dedicated `.prefab` infix stripped:
        // `entry_room.prefab.ron`'s `file_stem()` is `entry_room.prefab`, whose prefab
        // NAME is `entry_room`. A handle with no resolvable path / stem is skipped
        // defensively (it would carry no usable name).
        let Some(stem) = asset_server.get_path(untyped.id()).and_then(|path| {
            path.path()
                .file_stem()
                .map(|stem| prefab_name_from_stem(&stem.to_string_lossy()))
        }) else {
            continue;
        };
        let name = PrefabName::new(stem);
        // C6: validate the edge-opening invariant. A zero-opening prefab is EXCLUDED +
        // warn-logged (fail-closed, no panic) — the registry only holds connectable
        // prefabs.
        match Prefab::new(name, (**spec).clone()) {
            Ok(prefab) => registry.insert(prefab),
            Err(err) => warn!("GDTF Load: skipping invalid prefab — {err}"),
        }
    }
    Some(registry)
}

/// `Update`: rebuild the [`PrefabRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/maps/**/*.prefab.ron` — the GTW-418 LIVE prefab hot-reload, the prefab
/// mirror of [`redrive_gangs_on_asset_event`](super::gangs::redrive_gangs_on_asset_event).
///
/// A folder load fans out into one `RonAsset<PrefabSpec>` asset PER file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset
/// (not the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<PrefabSpec>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActivePrefabsFolderHandle`]'s member handles via [`build_prefab_registry`]
/// — the SAME builder the one-time resolve uses (so a live edit re-runs the C6 validation
/// identically). Overwriting via [`ResMut`] marks the registry changed.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`PrefabRegistry`] resource as
/// [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional
/// [`AssetServer`] / folder handle / `Assets` / [`PrefabRegistry`] borrows.
pub(in crate::states::load) fn redrive_prefabs_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<PrefabSpec>>>,
    asset_server: Option<Res<AssetServer>>,
    folder_handle: Option<Res<ActivePrefabsFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    prefab_specs: Option<Res<Assets<RonAsset<PrefabSpec>>>>,
    registry: Option<ResMut<PrefabRegistry>>,
) {
    let (
        Some(asset_server),
        Some(folder_handle),
        Some(folders),
        Some(prefab_specs),
        Some(mut registry),
    ) = (asset_server, folder_handle, folders, prefab_specs, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified prefab member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) =
        build_prefab_registry(&asset_server, &folders, &prefab_specs, &folder_handle)
    else {
        // A member spec is mid-reload (not yet back in the collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "prefab hot-reload: rebuilt PrefabRegistry from `assets/content/maps/` ({} prefabs)",
        registry.len(),
    );
}

/// The prefab NAME for a loaded prefab file's stem — the stem with the dedicated
/// `.prefab` infix stripped (GTW-418).
///
/// A prefab file is `<name>.prefab.ron`; Bevy's `file_stem()` yields `<name>.prefab`, so
/// the NAME is that stem minus a trailing `.prefab`. A stem without the infix is returned
/// unchanged (defensive — keeps a mis-named file's name its plain stem).
fn prefab_name_from_stem(stem: &str) -> String {
    stem.strip_suffix(".prefab").unwrap_or(stem).to_owned()
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
    use gdtf_battle_sim::level::{
        GridHeight, GridLevels, GridSize, GridWidth, LevelTheme, PrefabKey, PrefabRegistry,
        PrefabSpec, SpawnRole,
    };

    use super::redrive_prefabs_on_asset_event;
    use crate::states::load::{
        resources::ActivePrefabsFolderHandle,
        systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// A 3x3x1 footprint (a valid small prefab size), or `None` (assert-fail) on a bad
    /// span.
    fn small_size() -> Option<GridSize> {
        GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
    }

    /// A `Fill`-role `IndustrialHive` prefab with the given number of edge openings, parsed
    /// from inline RON so the test does not hand-assemble the value graph. Returns `None`
    /// (assert-fail) on a parse error rather than a denied `unwrap`. `openings` lets the
    /// test exercise the C6 validation both ways (>= 1 passes, 0 is rejected at build).
    fn prefab_spec(openings: usize) -> Option<PrefabSpec> {
        let opening_list = (0..openings)
            .map(|i| format!("(at: (cell: (x: 2, y: {i}), level: 0))"))
            .collect::<Vec<_>>()
            .join(", ");
        let ron = format!(
            "(theme: IndustrialHive, size: (width: 3, height: 3, levels: 1), \
             spawn_role: Fill, default_floor: \"deck\", \
             edge_openings: [{opening_list}])",
        );
        let parsed = ron::de::from_str::<PrefabSpec>(&ron);
        assert!(
            parsed.is_ok(),
            "prefab fixture must parse: {:?}",
            parsed.as_ref().err(),
        );
        parsed.ok()
    }

    /// A headless app with the real hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
    /// (registers `Assets<RonAsset<PrefabSpec>>`, `Assets<LoadedFolder>`, and the
    /// `AssetEvent` message buffers), the `PrefabSpec` RON loader, and the redrive system
    /// in `Update` — the gangs test-app mirror.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<PrefabSpec>(vec!["prefab.ron"])
            .add_systems(Update, redrive_prefabs_on_asset_event);
        app
    }

    /// Register a member prefab asset at `path` carrying `spec`, returning its handle.
    fn add_member(
        app: &mut App,
        path: &'static str,
        spec: PrefabSpec,
    ) -> Handle<RonAsset<PrefabSpec>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<PrefabSpec>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<PrefabSpec>>>()
            .insert(handle.id(), RonAsset::new(spec));
        assert!(inserted.is_ok(), "member spec insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(app: &mut App, members: &[Handle<RonAsset<PrefabSpec>>]) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// GTW-418: a `Modified` for a member `*.prefab.ron` REBUILDS the `PrefabRegistry`
    /// from the folder's members — keyed by `(theme, size, role)` — reflecting the edited
    /// spec, and EXCLUDES a prefab that loses its only edge opening (the C6 fail-closed
    /// rebuild).
    ///
    /// Discriminating: dropping the rebuild leaves the registry at its baseline; dropping
    /// the C6 validation would keep the zero-opening edit IN the registry.
    #[test]
    fn modified_member_rebuilds_prefab_registry_fail_closed() {
        let mut app = app();
        let Some(original) = prefab_spec(1) else {
            return;
        };
        let member = add_member(&mut app, "content/maps/hive/3x3/entry.prefab.ron", original);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActivePrefabsFolderHandle::new(folder));
        app.world_mut().insert_resource(PrefabRegistry::default());
        // Fire an initial Modified so the redrive builds the baseline registry from the
        // valid spec (the redrive only rebuilds on a Modified event, not on a bare update).
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let Some(size) = small_size() else { return };
        let key = PrefabKey::new(LevelTheme::IndustrialHive, size, SpawnRole::Fill);
        // Baseline: one valid prefab is registered under its (theme, size, role) key.
        assert_eq!(
            app.world()
                .get_resource::<PrefabRegistry>()
                .map(|r| r.prefabs_for(&key).len()),
            Some(1),
            "the valid prefab must be bucketed by (theme, size, role)",
        );

        // Hot-edit the member to ZERO edge openings (C6-invalid), fire Modified, rebuild.
        let Some(edited) = prefab_spec(0) else { return };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<PrefabSpec>>>()
            .get_mut(&member)
        {
            **asset = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        // C6: the now-invalid prefab is EXCLUDED from the rebuilt registry (fail-closed).
        assert_eq!(
            app.world()
                .get_resource::<PrefabRegistry>()
                .map(PrefabRegistry::is_empty),
            Some(true),
            "the hot-reload must exclude the prefab that lost its only edge opening (C6)",
        );
    }

    /// GTW-418: a hot-reload of a prefab member fires the `info!` line naming what
    /// reloaded. Run via `run_system_once` on the calling thread so the thread-local
    /// `tracing` capture sees the emission.
    ///
    /// Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn prefab_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(spec) = prefab_spec(1) else { return };
        let member = add_member(&mut app, "content/maps/hive/3x3/entry.prefab.ron", spec);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActivePrefabsFolderHandle::new(folder));
        app.world_mut().insert_resource(PrefabRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_prefabs_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("prefab hot-reload") && line.contains("PrefabRegistry")),
            "the prefab hot-reload must emit an info! line naming what reloaded; captured: \
             {captured:?}",
        );
    }
}
