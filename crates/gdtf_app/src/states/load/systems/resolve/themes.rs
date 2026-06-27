//! GTW-409: builds the theme-keyed [`ThemeCatalogRegistry`] from the loaded
//! `assets/content/themes/` folder, plus the LIVE hot-reload that rebuilds it on a
//! `*.theme.ron` edit. Mirrors the GTW-394 terrain resolve
//! ([`resolve_terrain`](super::terrain::resolve_terrain)) — except each member is keyed
//! by its DECLARED [`LevelTheme`], not the filename stem (a theme's catalog belongs to
//! the theme it declares).

use bevy::{
    asset::{AssetEvent, AssetServer, Assets, LoadedFolder, RecursiveDependencyLoadState},
    prelude::{Commands, MessageReader, Res, ResMut, info, warn},
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::level::{ThemeCatalogRegistry, ThemeSpec, ThemeTileCatalog};

use crate::states::load::resources::{ActiveThemesFolderHandle, LoadHandles};

/// GTW-409: builds the theme-keyed [`ThemeCatalogRegistry`] from the loaded
/// `assets/content/themes/` folder, mirroring the GTW-394 terrain resolve shape
/// (the theme mirror of [`resolve_terrain`](super::terrain::resolve_terrain)).
///
/// Called only while no [`ThemeCatalogRegistry`] resource exists yet (the caller's
/// own-absence guard), independently of the other load branches:
///
/// - Gates on the themes folder's [`RecursiveDependencyLoadState`]`::Loaded`. On
///   [`RecursiveDependencyLoadState::Failed`] it `warn!`s and inserts an EMPTY
///   [`ThemeCatalogRegistry`] so `Load` always exits with one present and never hangs on
///   a bad folder (the ADR-0003 error-path safety-net; the later consumption slice then
///   fails closed on a missing theme key rather than crashing).
/// - On success it reads the [`LoadedFolder`]'s member handles, types each as a
///   `RonAsset<ThemeSpec>`, reads its [`ThemeSpec`] out of the `Assets<RonAsset<ThemeSpec>>`
///   collection, builds a [`ThemeTileCatalog`] from it, and inserts it under the spec's
///   DECLARED [`LevelTheme`](gdtf_battle_sim::level::LevelTheme) (NOT the filename stem —
///   a theme's catalog is keyed by the theme it belongs to). If ANY member spec is not
///   yet in the collection (the one-frame loaded-but-not-yet-in-collection race), it
///   returns WITHOUT inserting and retries next frame — so a partial / empty registry is
///   never published while the folder is non-empty. The registry holds the catalogs BY
///   VALUE, so they survive the folder handle being dropped on `OnExit(Load)`.
/// - Else (still loading) it does nothing and is polled again next frame.
pub(super) fn resolve_themes(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    theme_specs: &Assets<RonAsset<ThemeSpec>>,
    handles: &LoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.themes);

    // Failure path: a bad/missing themes folder must not hang the app. Warn and insert
    // an EMPTY registry so Load always exits with one present (the later consumption
    // slice then fails closed on a missing theme key rather than crashing).
    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF Load: the `themes` folder failed to load; inserting an empty \
             ThemeCatalogRegistry (the editor/procgen will fail closed on a missing theme key)",
        );
        commands.insert_resource(ThemeCatalogRegistry::default());
        return;
    }

    // Success path: once every theme file in the folder is loaded, read the
    // LoadedFolder's member handles and build the theme-keyed registry.
    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(registry) = build_theme_registry(folders, theme_specs, &handles.themes) else {
            // Loaded-but-not-yet-in-collection (the folder, or a member spec) — retry
            // next frame (the system stays alive while the ThemeCatalogRegistry is absent).
            return;
        };

        // Insert the built registry — like the TerrainRegistry it persists past
        // OnExit(Load) (it is NOT removed in cleanup), because the editor/procgen reads it.
        commands.insert_resource(registry);
        // Insert the PERSISTENT folder handle alongside the registry — it survives
        // OnExit(Load) so the live hot-reload handler can re-enumerate the folder's member
        // handles to rebuild the registry on a `*.theme.ron` edit, and holding it keeps
        // every member theme asset loaded for the file-watcher.
        commands.insert_resource(ActiveThemesFolderHandle::new((*handles.themes).clone()));
    }
}

/// Build the theme-keyed [`ThemeCatalogRegistry`] from a loaded `themes/` [`LoadedFolder`],
/// or [`None`] if the folder (or any member spec) is not yet in its collection — the
/// theme mirror of `build_terrain_registry` (in the sibling `terrain` module).
///
/// Shared by [`resolve_themes`] (the one-time `Load`-state build) and
/// [`redrive_themes_on_asset_event`] (the live rebuild on a hot edit), so both build the
/// registry IDENTICALLY: read the folder's member handles, type each as a
/// `RonAsset<ThemeSpec>`, read its [`ThemeSpec`] out of the collection, build a
/// [`ThemeTileCatalog`], and key it by the spec's DECLARED
/// [`LevelTheme`](gdtf_battle_sim::level::LevelTheme). Returns [`None`] (do NOT publish a
/// partial registry) if the folder or any member spec is not yet in its collection — the
/// caller retries next frame.
fn build_theme_registry(
    folders: &Assets<LoadedFolder>,
    theme_specs: &Assets<RonAsset<ThemeSpec>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<ThemeCatalogRegistry> {
    let folder = folders.get(folder_handle)?;

    let mut registry = ThemeCatalogRegistry::default();
    for untyped in &folder.handles {
        // Type the untyped member handle as a RonAsset<ThemeSpec> and read its spec out
        // of the collection.
        let handle = untyped.clone().typed_debug_checked::<RonAsset<ThemeSpec>>();
        // One-frame loaded-but-not-yet-in-collection race: a member spec is not in the
        // collection yet. Bail (do NOT build a partial registry) so the caller re-polls.
        let spec = theme_specs.get(&handle)?;
        // Key by the spec's DECLARED LevelTheme — a theme's catalog belongs to the theme
        // it names, not its filename (the terrain mirror keys by stem; this keys by the
        // declared enum, the editor/procgen lookup key).
        let spec = (**spec).clone();
        registry.insert(spec.theme, ThemeTileCatalog::from_spec(spec));
    }
    Some(registry)
}

/// `Update`: rebuild the [`ThemeCatalogRegistry`] in place on a matching
/// [`AssetEvent::Modified`](bevy::asset::AssetEvent::Modified) for any member
/// `assets/content/themes/*.theme.ron` — the GTW-409 LIVE theme-catalog hot-reload, the
/// theme mirror of [`redrive_terrain_on_asset_event`](super::terrain::redrive_terrain_on_asset_event).
///
/// A folder load fans out into one `RonAsset<ThemeSpec>` asset PER file, and a hot edit
/// fires an [`AssetEvent`](bevy::asset::AssetEvent)`::Modified` for THAT member asset
/// (not the [`LoadedFolder`] handle), so this reacts to ANY
/// `AssetEvent<RonAsset<ThemeSpec>>::Modified` and rebuilds the whole registry from the
/// PERSISTENT [`ActiveThemesFolderHandle`]'s member handles via [`build_theme_registry`]
/// — the SAME builder the one-time resolve uses. Overwriting via [`ResMut`] marks the
/// registry changed, so the next editor/procgen read resolves against the edited specs
/// WITHOUT a rebuild.
///
/// Guarded so it never panics before the load chain has resolved (pre-`Load`): it takes
/// the folder handle / the `Assets` collections / the [`ThemeCatalogRegistry`] resource
/// as [`Option`]al borrows, draining the reader and returning early if any is missing
/// (`bevy-traps.md` #1).
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the optional folder handle /
/// `Assets` / [`ThemeCatalogRegistry`] borrows.
pub(in crate::states::load) fn redrive_themes_on_asset_event(
    mut events: MessageReader<AssetEvent<RonAsset<ThemeSpec>>>,
    folder_handle: Option<Res<ActiveThemesFolderHandle>>,
    folders: Option<Res<Assets<LoadedFolder>>>,
    theme_specs: Option<Res<Assets<RonAsset<ThemeSpec>>>>,
    registry: Option<ResMut<ThemeCatalogRegistry>>,
) {
    let (Some(folder_handle), Some(folders), Some(theme_specs), Some(mut registry)) =
        (folder_handle, folders, theme_specs, registry)
    else {
        // Drain the reader so a pre-resolve event does not linger and re-fire once the
        // resources arrive; there is nothing to rebuild yet.
        events.clear();
        return;
    };

    // Rebuild on ANY modified theme member — a single rebuild from the latest in-memory
    // specs covers however many member events arrived this frame.
    let modified = events
        .read()
        .any(|event| matches!(event, AssetEvent::Modified { .. }));
    if !modified {
        return;
    }

    let Some(rebuilt) = build_theme_registry(&folders, &theme_specs, &folder_handle) else {
        // A member spec is mid-reload (not yet back in the collection) — leave the
        // existing registry until it settles; the next event rebuilds.
        return;
    };
    *registry = rebuilt;
    info!(
        "theme hot-reload: rebuilt ThemeCatalogRegistry from `assets/content/themes/` ({} themes)",
        registry.len(),
    );
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
    use gdtf_battle_sim::level::{LevelTheme, ThemeCatalogRegistry, ThemeSpec};

    use super::redrive_themes_on_asset_event;
    use crate::states::load::{
        resources::ActiveThemesFolderHandle,
        systems::resolve::hot_reload_test_support::capture_logs,
    };

    /// An IndustrialHive-shaped `ThemeSpec` whose default-floor tile carries the given
    /// `move_cost` — parsed from inline RON so the test does not hand-assemble the nested
    /// payload. Returns `None` (assert-fail) on a parse error rather than a denied
    /// `unwrap`. The `move_cost` value is an arbitrary DISCRIMINATOR, NOT a shipped pin.
    fn hive_spec(move_cost: u8) -> Option<ThemeSpec> {
        let ron = format!(
            "(theme: IndustrialHive, default_floor: \"deck\", tiles: {{ \"deck\": (display_name: \
             \"Deck\", atlas_index: 6, kind: Floor(move_cost: {move_cost})) }})",
        );
        let parsed = ron::de::from_str::<ThemeSpec>(&ron);
        assert!(
            parsed.is_ok(),
            "theme fixture must parse: {:?}",
            parsed.as_ref().err(),
        );
        parsed.ok()
    }

    /// A headless app with the real hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
    /// (registers `Assets<RonAsset<ThemeSpec>>`, `Assets<LoadedFolder>`, and the
    /// `AssetEvent` message buffers), the `ThemeSpec` RON loader, and the redrive system
    /// in `Update` — the terrain test-app mirror.
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .init_ron_asset_with_extensions::<ThemeSpec>(vec!["theme.ron"])
            .add_systems(Update, redrive_themes_on_asset_event);
        app
    }

    /// Register a member theme asset at `path` carrying `spec`, returning its handle.
    fn add_member(
        app: &mut App,
        path: &'static str,
        spec: ThemeSpec,
    ) -> Handle<RonAsset<ThemeSpec>> {
        let handle = app
            .world()
            .resource::<AssetServer>()
            .load::<RonAsset<ThemeSpec>>(path);
        let inserted = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<ThemeSpec>>>()
            .insert(handle.id(), RonAsset::new(spec));
        assert!(inserted.is_ok(), "member spec insert must succeed");
        handle
    }

    /// Build a `LoadedFolder` over the given member handles, add it, return its handle.
    fn add_folder(app: &mut App, members: &[Handle<RonAsset<ThemeSpec>>]) -> Handle<LoadedFolder> {
        let folder = LoadedFolder {
            handles: members.iter().map(|h| h.clone().untyped()).collect(),
        };
        app.world_mut()
            .resource_mut::<Assets<LoadedFolder>>()
            .add(folder)
    }

    /// GTW-409: a `Modified` for a member `*.theme.ron` REBUILDS the
    /// `ThemeCatalogRegistry` from the folder's members — keyed by the spec's DECLARED
    /// `LevelTheme` — reflecting the edited spec.
    ///
    /// Pin-discriminating: dropping the rebuild leaves the OLD `move_cost`; mis-keying
    /// drops the `IndustrialHive` entry. The `move_cost` fixture values (4 -> 7) are
    /// arbitrary discriminators — NOT shipped tuning pins.
    #[test]
    fn modified_member_rebuilds_theme_registry() {
        let mut app = app();
        let Some(original) = hive_spec(4) else {
            return;
        };
        let member = add_member(
            &mut app,
            "content/themes/industrial_hive.theme.ron",
            original,
        );
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveThemesFolderHandle::new(folder));
        // A stale baseline registry (empty) the rebuild must overwrite.
        app.world_mut()
            .insert_resource(ThemeCatalogRegistry::default());
        app.update();

        // Hot-edit the member spec to a DISTINCT move_cost, fire Modified, rebuild.
        let Some(edited) = hive_spec(7) else { return };
        if let Some(mut asset) = app
            .world_mut()
            .resource_mut::<Assets<RonAsset<ThemeSpec>>>()
            .get_mut(&member)
        {
            **asset = edited;
        }
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });
        app.update();

        let rebuilt_move_cost = app
            .world()
            .get_resource::<ThemeCatalogRegistry>()
            .and_then(|r| r.catalog(LevelTheme::IndustrialHive).cloned())
            .and_then(|c| c.default_floor().cloned())
            .and_then(|tile| match tile.kind {
                gdtf_battle_sim::level::CatalogTileKind::Floor { move_cost } => Some(*move_cost),
                _ => None,
            });
        assert_eq!(
            rebuilt_move_cost,
            Some(7),
            "the hot-reload must rebuild the registry, keyed by declared theme, with the \
             edited move_cost",
        );
    }

    /// GTW-409: a hot-reload of a theme member fires the Part C `info!` line naming what
    /// reloaded. Run via `run_system_once` on the calling thread so the thread-local
    /// `tracing` capture sees the emission.
    ///
    /// Pin-discriminating: removing the `info!` leaves the capture empty.
    #[test]
    fn theme_hot_reload_logs_an_info_line() {
        let mut app = app();
        let Some(spec) = hive_spec(4) else { return };
        let member = add_member(&mut app, "content/themes/industrial_hive.theme.ron", spec);
        let folder = add_folder(&mut app, std::slice::from_ref(&member));
        app.world_mut()
            .insert_resource(ActiveThemesFolderHandle::new(folder));
        app.world_mut()
            .insert_resource(ThemeCatalogRegistry::default());
        app.world_mut()
            .write_message(AssetEvent::Modified { id: member.id() });

        let captured = capture_logs(|| {
            let result = app
                .world_mut()
                .run_system_once(redrive_themes_on_asset_event);
            assert!(result.is_ok(), "the redrive system must run cleanly");
        });

        assert!(
            captured
                .iter()
                .any(|line| line.contains("theme hot-reload")
                    && line.contains("ThemeCatalogRegistry")),
            "the theme hot-reload must emit an info! line naming what reloaded; captured: \
             {captured:?}",
        );
    }
}
