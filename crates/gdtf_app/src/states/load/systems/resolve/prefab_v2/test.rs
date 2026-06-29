//! GTW-489 hot-reload + C2 tests for the NEW UUID-keyed v2 prefab redrive, mirroring the
//! legacy prefab redrive tests ([`prefabs::test`](super::super::prefabs)): a `Modified`
//! `AssetEvent` for a member of the persistent maps folder rebuilds the
//! [`PrefabRegistry2`] from the folder handle and emits the pin-discriminating `info!` line.
//!
//! Crucially these tests exercise the C2 contract — an openingless (ZERO-placement)
//! [`PrefabSpecV2`] is INCLUDED in the registry (the v2 schema has no edge-opening
//! validation, so the old C6 `NoEdgeOpening` exclusion is NOT applied on this path).
//!
//! No magnitude assertions — the UUID / grid fixtures are mechanism, not balance.

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_battle_sim::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, PrefabKey2, PrefabRegistry2, PrefabSpecV2,
        SpawnRole, TerrainPlacementEntry, ThemeUuid,
    },
    metric::{Cell, CellLevel, Level},
    terrain::def::TerrainUuid,
};

use super::redrive_prefabs_v2_on_asset_event;
use crate::states::load::{
    resources::ActivePrefabsV2FolderHandle, systems::resolve::hot_reload_test_support::capture_logs,
};

/// A stable [`ThemeUuid`] fixture (a fixed UUID) — mechanism, not balance.
fn theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_09a1))
}

/// A 3x3x1 footprint (a valid small prefab size), or `None` (assert-fail) on a bad span.
fn small_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

/// A `Fill`-role v2 prefab spec with `placements` entries (`placements == 0` is the
/// openingless case the C2 contract requires the loader to INCLUDE). Built in code — the spec
/// carries its own theme UUID, so no inline RON is needed.
fn prefab_v2_spec(theme: ThemeUuid, size: GridSize, placements: usize) -> PrefabSpecV2 {
    let entries = (0..placements)
        .map(|i| {
            let offset = u128::try_from(i).unwrap_or(0);
            TerrainPlacementEntry::new(
                TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(
                    0x0184_0a3e_09b0 + offset,
                )),
                CellLevel::new(Cell::new(1, i32::try_from(i).unwrap_or(0)), Level::new(0)),
            )
        })
        .collect();
    PrefabSpecV2::new(theme, size, SpawnRole::Fill, entries)
}

/// A headless app with the real v2 hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
/// (registers `Assets<RonAsset<PrefabSpecV2>>`, `Assets<LoadedFolder>`, and the `AssetEvent`
/// message buffers), the dedicated-extension `PrefabSpecV2` RON loader, and the redrive system
/// in `Update` — the legacy prefab test-app mirror.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset_with_extensions::<PrefabSpecV2>(vec!["prefab_v2.ron"])
        .add_systems(Update, redrive_prefabs_v2_on_asset_event);
    app
}

/// Register a member v2 prefab asset at `path` carrying `spec`, returning its handle.
fn add_member(
    app: &mut App,
    path: &'static str,
    spec: PrefabSpecV2,
) -> Handle<RonAsset<PrefabSpecV2>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<PrefabSpecV2>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<PrefabSpecV2>>>()
        .insert(handle.id(), RonAsset::new(spec));
    assert!(inserted.is_ok(), "member v2 spec insert must succeed");
    handle
}

/// Build a `LoadedFolder` over the given member handles, add it, return its handle.
fn add_folder(app: &mut App, members: &[Handle<RonAsset<PrefabSpecV2>>]) -> Handle<LoadedFolder> {
    let folder = LoadedFolder {
        handles: members.iter().map(|h| h.clone().untyped()).collect(),
    };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

/// GTW-489 C3 — a `Modified` for a member `*.prefab_v2.ron` REBUILDS the [`PrefabRegistry2`]
/// from the folder's members — keyed by `(theme, size, role)` — reflecting the edited spec.
///
/// Pin-discriminating: dropping the rebuild leaves the registry at its (empty) baseline;
/// mis-keying drops the entry.
#[test]
fn modified_member_rebuilds_prefab_v2_registry() {
    let mut app = app();
    let theme = theme_uuid();
    let Some(size) = small_size() else { return };
    let member = add_member(
        &mut app,
        "content/maps/hive/3x3/entry.prefab_v2.ron",
        prefab_v2_spec(theme, size, 1),
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActivePrefabsV2FolderHandle::new(folder));
    // A stale baseline registry (empty) the rebuild must overwrite.
    app.world_mut().insert_resource(PrefabRegistry2::default());
    // Fire an initial Modified so the redrive builds the baseline registry (the redrive only
    // rebuilds on a Modified event, not on a bare update).
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let key = PrefabKey2::new(theme, size, SpawnRole::Fill);
    assert_eq!(
        app.world()
            .get_resource::<PrefabRegistry2>()
            .map(|r| r.prefabs_for(&key).len()),
        Some(1),
        "the hot-reload must rebuild the registry, bucketing the v2 prefab by (theme, size, \
         role)",
    );
}

/// GTW-489 C2 — a v2 prefab with ZERO placements (an openingless fragment) is INCLUDED in the
/// rebuilt [`PrefabRegistry2`] (the v2 schema has no edge-opening validation, so the old C6
/// `NoEdgeOpening` exclusion is NOT applied on this path).
///
/// Pin-discriminating: were the legacy C6 gate applied here, the zero-placement prefab would
/// be EXCLUDED and the bucket empty — this test asserts it is PRESENT.
#[test]
fn zero_placement_prefab_is_included_no_c6_exclusion() {
    let mut app = app();
    let theme = theme_uuid();
    let Some(size) = small_size() else { return };
    // Openingless: zero placements (the v2 schema has no edge_openings to author at all).
    let member = add_member(
        &mut app,
        "content/maps/hive/3x3/sealed.prefab_v2.ron",
        prefab_v2_spec(theme, size, 0),
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActivePrefabsV2FolderHandle::new(folder));
    app.world_mut().insert_resource(PrefabRegistry2::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let key = PrefabKey2::new(theme, size, SpawnRole::Fill);
    assert_eq!(
        app.world()
            .get_resource::<PrefabRegistry2>()
            .map(|r| r.prefabs_for(&key).len()),
        Some(1),
        "an openingless (zero-placement) v2 prefab must be INCLUDED — the v2 path runs NO C6 \
         edge-opening exclusion",
    );
}

/// GTW-489 C3 — a hot-reload of a v2 prefab member fires the `info!` line naming what
/// reloaded. Run via `run_system_once` on the calling thread so the thread-local `tracing`
/// capture sees the emission. Pin-discriminating: removing the `info!` leaves the capture
/// empty.
#[test]
fn prefab_v2_hot_reload_logs_an_info_line() {
    let mut app = app();
    let theme = theme_uuid();
    let Some(size) = small_size() else { return };
    let member = add_member(
        &mut app,
        "content/maps/hive/3x3/entry.prefab_v2.ron",
        prefab_v2_spec(theme, size, 1),
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActivePrefabsV2FolderHandle::new(folder));
    app.world_mut().insert_resource(PrefabRegistry2::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_prefabs_v2_on_asset_event);
        assert!(result.is_ok(), "the v2 redrive system must run cleanly");
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("prefab-v2 hot-reload") && line.contains("PrefabRegistry2")),
        "the v2 prefab hot-reload must emit an info! line naming what reloaded; captured: \
         {captured:?}",
    );
}
