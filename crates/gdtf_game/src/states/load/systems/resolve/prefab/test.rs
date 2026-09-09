use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetServer, Assets, Handle, LoadedFolder},
    prelude::*,
};
use cobalt_ron_assets::{RonAsset, RonAssetAppExt};
use cobalt_test_utils::unwatched_asset_plugin;
use gdtf_battle_sim::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, PrefabSpec,
        SpawnRole, TerrainPlacementEntry, ThemeUuid,
    },
    metric::{Cell, CellLevel, Level},
    terrain::{def::TerrainUuid, facing::TerrainFacing},
};

use super::redrive_prefabs_on_asset_event;
use crate::states::load::resources::ActivePrefabsFolderHandle;

fn theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_09a1))
}

fn small_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

fn prefab_spec(theme: ThemeUuid, size: GridSize, placements: usize) -> PrefabSpec {
    let entries = (0..placements)
        .map(|i| {
            let offset = u128::try_from(i).unwrap_or(0);
            TerrainPlacementEntry::new(
                TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(
                    0x0184_0a3e_09b0 + offset,
                )),
                CellLevel::new(Cell::new(1, i32::try_from(i).unwrap_or(0)), Level::new(0)),
                TerrainFacing::default(),
            )
        })
        .collect();
    PrefabSpec::new(theme, size, SpawnRole::Fill, entries)
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(unwatched_asset_plugin())
        .init_ron_asset_with_extensions::<PrefabSpec>(vec!["prefab.ron"])
        .add_systems(Update, redrive_prefabs_on_asset_event);
    app
}

fn add_member(app: &mut App, path: &'static str, spec: PrefabSpec) -> Handle<RonAsset<PrefabSpec>> {
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

fn add_folder(app: &mut App, members: &[Handle<RonAsset<PrefabSpec>>]) -> Handle<LoadedFolder> {
    let folder = LoadedFolder {
        handles: members.iter().map(|h| h.clone().untyped()).collect(),
    };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

#[test]
fn modified_member_rebuilds_prefab_registry() {
    let mut app = app();
    let theme = theme_uuid();
    let Some(size) = small_size() else { return };
    let member = add_member(
        &mut app,
        "content/maps/hive/3x3/entry.prefab.ron",
        prefab_spec(theme, size, 1),
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActivePrefabsFolderHandle::new(folder));
    app.world_mut().insert_resource(PrefabRegistry::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let key = PrefabKey::new(theme, size, SpawnRole::Fill);
    assert_eq!(
        app.world()
            .get_resource::<PrefabRegistry>()
            .map(|r| r.prefabs_for(&key).len()),
        Some(1),
        "the hot-reload must rebuild the registry, bucketing the prefab by (theme, size, role)",
    );
}

#[test]
fn zero_placement_prefab_is_included_no_opening_exclusion() {
    let mut app = app();
    let theme = theme_uuid();
    let Some(size) = small_size() else { return };
    let member = add_member(
        &mut app,
        "content/maps/hive/3x3/sealed.prefab.ron",
        prefab_spec(theme, size, 0),
    );
    let folder = add_folder(&mut app, std::slice::from_ref(&member));
    app.world_mut()
        .insert_resource(ActivePrefabsFolderHandle::new(folder));
    app.world_mut().insert_resource(PrefabRegistry::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let key = PrefabKey::new(theme, size, SpawnRole::Fill);
    assert_eq!(
        app.world()
            .get_resource::<PrefabRegistry>()
            .map(|r| r.prefabs_for(&key).len()),
        Some(1),
        "an openingless (zero-placement) prefab must be INCLUDED — the path runs NO C6 \
         edge-opening exclusion",
    );
}
