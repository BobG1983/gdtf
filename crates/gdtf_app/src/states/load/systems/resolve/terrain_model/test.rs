//! GTW-487 C2 hot-reload tests for the NEW UUID-keyed terrain-def + theme-def redrives,
//! mirroring the surviving gang redrive tests
//! ([`gangs::test`](super::super::gangs)): a `Modified` `AssetEvent` for a member of the
//! persistent per-theme folder rebuilds the matching registry from the folder handle and
//! emits the pin-discriminating `info!` line. (GTW-494 retired the legacy `resolve::terrain` /
//! `resolve::themes` loaders these once mirrored.)

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder},
    ecs::system::RunSystemOnce,
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry},
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

use super::{redrive_terrain_defs_on_asset_event, redrive_theme_defs_on_asset_event};
use crate::states::load::{
    resources::ActiveTerrainModelFolderHandle,
    systems::resolve::hot_reload_test_support::capture_logs,
};

/// A 128-bit constant → [`TerrainUuid`] (the def's own key).
const fn terrain_uuid(raw: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(raw))
}

/// A 128-bit constant → [`ThemeUuid`] (the def's own key).
const fn theme_uuid(raw: u128) -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(raw))
}

/// A Cover-kind [`TerrainDef`] keyed by `key`, carrying `display` as its label — built in
/// code (no inline RON needed since the def carries its own key).
fn cover_def(key: TerrainUuid, display: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(display.to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags: Vec::new(),
    }
}

/// A [`UuidThemeDef`] keyed by `key`, naming `floor` as its default-floor terrain.
fn theme_def(key: ThemeUuid, display: &str, floor: TerrainUuid) -> UuidThemeDef {
    UuidThemeDef {
        key,
        display_name: ThemeDisplayName::new(display.to_owned()),
        default_floor: floor,
        terrain: vec![floor],
    }
}

/// A headless app with the real new-model hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
/// (registers `Assets<RonAsset<TerrainDef>>` / `Assets<RonAsset<UuidThemeDef>>`,
/// `Assets<LoadedFolder>`, and the `AssetEvent` message buffers), the dedicated-extension RON
/// loaders, and BOTH redrive systems in `Update` — the legacy terrain test-app mirror.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset_with_extensions::<TerrainDef>(vec!["terrain_def.ron"])
        .init_ron_asset_with_extensions::<UuidThemeDef>(vec!["terrain_theme.ron"])
        .add_systems(
            Update,
            (
                redrive_terrain_defs_on_asset_event,
                redrive_theme_defs_on_asset_event,
            ),
        );
    app
}

/// Register a member terrain-def asset at `path` carrying `def`, returning its handle.
fn add_terrain_member(
    app: &mut App,
    path: &'static str,
    def: TerrainDef,
) -> Handle<RonAsset<TerrainDef>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<TerrainDef>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<TerrainDef>>>()
        .insert(handle.id(), RonAsset::new(def));
    assert!(inserted.is_ok(), "member terrain def insert must succeed");
    handle
}

/// Register a member theme-def asset at `path` carrying `def`, returning its handle.
fn add_theme_member(
    app: &mut App,
    path: &'static str,
    def: UuidThemeDef,
) -> Handle<RonAsset<UuidThemeDef>> {
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<UuidThemeDef>>(path);
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<UuidThemeDef>>>()
        .insert(handle.id(), RonAsset::new(def));
    assert!(inserted.is_ok(), "member theme def insert must succeed");
    handle
}

/// Build a `LoadedFolder` over the given member untyped handles, add it, return its handle.
fn add_folder(app: &mut App, members: Vec<bevy::asset::UntypedHandle>) -> Handle<LoadedFolder> {
    let folder = LoadedFolder { handles: members };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

/// C2 (terrain def): a `Modified` for a member `*.terrain_def.ron` REBUILDS the
/// `TerrainDefRegistry` from the folder's members — keyed by the def's OWN `TerrainUuid` —
/// reflecting the edited def. Pin-discriminating: dropping the rebuild leaves the OLD
/// display name; mis-keying drops the entry.
#[test]
fn modified_member_rebuilds_terrain_def_registry() {
    let mut app = app();
    let key = terrain_uuid(0x0184_0a3e_0701);
    let member = add_terrain_member(
        &mut app,
        "terrain/industrial_hive/crate.terrain_def.ron",
        cover_def(key, "Old Crate"),
    );
    let folder = add_folder(&mut app, vec![member.clone().untyped()]);
    app.world_mut()
        .insert_resource(ActiveTerrainModelFolderHandle::new(folder));
    // A stale baseline registry (empty) the rebuild must overwrite.
    app.world_mut()
        .insert_resource(TerrainDefRegistry::default());
    app.update();

    // Hot-edit the member def to a DISTINCT display name, fire Modified, rebuild.
    if let Some(mut asset) = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<TerrainDef>>>()
        .get_mut(&member)
    {
        **asset = cover_def(key, "Edited Crate");
    }
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let rebuilt = app
        .world()
        .get_resource::<TerrainDefRegistry>()
        .and_then(|r| r.def(&key).map(|d| (*d.display_name).clone()));
    assert_eq!(
        rebuilt,
        Some("Edited Crate".to_owned()),
        "the hot-reload must rebuild the registry, keyed by the def's UUID, with the edited def",
    );
}

/// C2 (terrain def): a hot-reload of a terrain-def member fires the `info!` line naming what
/// reloaded. Run via `run_system_once` on the calling thread so the thread-local `tracing`
/// capture sees the emission. Pin-discriminating: removing the `info!` leaves the capture
/// empty.
#[test]
fn terrain_def_hot_reload_logs_an_info_line() {
    let mut app = app();
    let key = terrain_uuid(0x0184_0a3e_0702);
    let member = add_terrain_member(
        &mut app,
        "terrain/industrial_hive/crate.terrain_def.ron",
        cover_def(key, "Crate"),
    );
    let folder = add_folder(&mut app, vec![member.clone().untyped()]);
    app.world_mut()
        .insert_resource(ActiveTerrainModelFolderHandle::new(folder));
    app.world_mut()
        .insert_resource(TerrainDefRegistry::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_terrain_defs_on_asset_event);
        assert!(
            result.is_ok(),
            "the terrain-def redrive system must run cleanly"
        );
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("terrain-def hot-reload")
                && line.contains("TerrainDefRegistry")),
        "the terrain-def hot-reload must emit an info! line naming what reloaded; captured: \
         {captured:?}",
    );
}

/// C2 (theme def): a `Modified` for a member `*.terrain_theme.ron` REBUILDS the
/// `UuidThemeRegistry` from the folder's members — keyed by the def's OWN `ThemeUuid` —
/// reflecting the edited def. Pin-discriminating: dropping the rebuild leaves the OLD
/// default-floor; mis-keying drops the entry.
#[test]
fn modified_member_rebuilds_theme_def_registry() {
    let mut app = app();
    let key = theme_uuid(0x0184_0a3e_07a1);
    let original_floor = terrain_uuid(0x0184_0a3e_07b1);
    let member = add_theme_member(
        &mut app,
        "terrain/industrial_hive/industrial_hive.terrain_theme.ron",
        theme_def(key, "Industrial Hive", original_floor),
    );
    let folder = add_folder(&mut app, vec![member.clone().untyped()]);
    app.world_mut()
        .insert_resource(ActiveTerrainModelFolderHandle::new(folder));
    app.world_mut()
        .insert_resource(UuidThemeRegistry::default());
    app.update();

    // Hot-edit the theme def to a DISTINCT default-floor UUID, fire Modified, rebuild.
    let edited_floor = terrain_uuid(0x0184_0a3e_07b2);
    if let Some(mut asset) = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<UuidThemeDef>>>()
        .get_mut(&member)
    {
        **asset = theme_def(key, "Industrial Hive", edited_floor);
    }
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });
    app.update();

    let rebuilt = app
        .world()
        .get_resource::<UuidThemeRegistry>()
        .and_then(|r| r.default_floor(&key));
    assert_eq!(
        rebuilt,
        Some(edited_floor),
        "the hot-reload must rebuild the registry, keyed by the def's UUID, with the edited \
         default-floor",
    );
}

/// C2 (theme def): a hot-reload of a theme-def member fires the `info!` line naming what
/// reloaded. Pin-discriminating: removing the `info!` leaves the capture empty.
#[test]
fn theme_def_hot_reload_logs_an_info_line() {
    let mut app = app();
    let key = theme_uuid(0x0184_0a3e_07a2);
    let floor = terrain_uuid(0x0184_0a3e_07b3);
    let member = add_theme_member(
        &mut app,
        "terrain/industrial_hive/industrial_hive.terrain_theme.ron",
        theme_def(key, "Industrial Hive", floor),
    );
    let folder = add_folder(&mut app, vec![member.clone().untyped()]);
    app.world_mut()
        .insert_resource(ActiveTerrainModelFolderHandle::new(folder));
    app.world_mut()
        .insert_resource(UuidThemeRegistry::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: member.id() });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_theme_defs_on_asset_event);
        assert!(
            result.is_ok(),
            "the theme-def redrive system must run cleanly"
        );
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("theme-def hot-reload")
                && line.contains("UuidThemeRegistry")),
        "the theme-def hot-reload must emit an info! line naming what reloaded; captured: \
         {captured:?}",
    );
}
