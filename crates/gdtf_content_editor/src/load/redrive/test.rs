//! GTW-533 C3 real-path tests for the EDITOR's live folder-content hot-reload.
//!
//! Each test drives the editor's ACTUAL registered redrive system (through `app.update()` or
//! `run_system_once`, not a copy) on a headless `MinimalPlugins` + `AssetPlugin` app: seed the
//! persistent [`EditorLoadHandles`] + a folder + a member asset, hot-edit the member's in-memory
//! payload, fire the SAME `AssetEvent::Modified` the real file-watcher emits, and assert the
//! resolved registry reflects the edit WITHOUT an app restart. Proves the previously-non-reloading
//! editor asset types now hot-reload through the shared `file_watcher` path (C3).

use std::sync::{Arc, Mutex};

use bevy::{
    MinimalPlugins,
    asset::{AssetEvent, AssetPlugin, AssetServer, Assets, Handle, LoadedFolder, UntypedHandle},
    ecs::system::RunSystemOnce,
    log::{
        tracing::{
            Event, Subscriber,
            field::{Field, Visit},
            subscriber::with_default,
        },
        tracing_subscriber::{Layer, layer::Context, prelude::*, registry::Registry},
    },
    prelude::*,
};
use gdtf_assets::{RonAsset, RonAssetAppExt};
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_ui::theme::GdtfThemeSpec;

use super::{redrive_terrain_defs_on_asset_event, redrive_weapons_on_asset_event};
use crate::load::handles::{
    EditorArmorFolderHandle, EditorLoadHandles, EditorTerrainModelFolderHandle, EditorThemeHandle,
    EditorTileRolesHandle, EditorWeaponsFolderHandle,
};

/// A scoped `tracing` layer recording each event's `message` field — the minimal capture needed
/// to prove the `info!` hot-reload line fired (mirrors the presenter/game redrive test capture;
/// no shared util is reachable from this crate).
struct CaptureLayer {
    /// The shared buffer captured messages append to.
    messages: Arc<Mutex<Vec<String>>>,
}

/// Pulls the `message` field's debug rendering out of a `tracing` event.
struct MessageVisitor {
    /// The captured message text, if a `message` field was visited.
    message: Option<String>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{value:?}"));
        }
    }
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor { message: None };
        event.record(&mut visitor);
        if let Some(message) = visitor.message
            && let Ok(mut buffer) = self.messages.lock()
        {
            buffer.push(message);
        }
    }
}

/// Run `body` with a scoped [`CaptureLayer`] active, returning every captured message in
/// emission order (scoped via `with_default`, so it never leaks into other tests).
fn capture_logs(body: impl FnOnce()) -> Vec<String> {
    let messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let layer = CaptureLayer {
        messages: Arc::clone(&messages),
    };
    let subscriber = Registry::default().with(layer);
    with_default(subscriber, body);
    messages
        .lock()
        .map(|buffer| buffer.clone())
        .unwrap_or_default()
}

/// A headless app with the real editor hot-reload wiring: `MinimalPlugins` + `AssetPlugin`
/// (registers the `Assets` collections + the `AssetEvent` message buffers), the editor's
/// dedicated-extension RON loaders, and the two folder redrives under test in `Update`.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_ron_asset_with_extensions::<WeaponSpec>(vec!["weapon.ron"])
        .init_ron_asset_with_extensions::<TerrainDef>(vec!["terrain_def.ron"])
        .add_systems(
            Update,
            (
                redrive_weapons_on_asset_event,
                redrive_terrain_defs_on_asset_event,
            ),
        );
    app
}

/// Add a `LoadedFolder` over the given member untyped handles, return its handle.
fn add_folder(app: &mut App, members: Vec<UntypedHandle>) -> Handle<LoadedFolder> {
    let folder = LoadedFolder { handles: members };
    app.world_mut()
        .resource_mut::<Assets<LoadedFolder>>()
        .add(folder)
}

/// Insert an [`EditorLoadHandles`] whose weapons/terrain folders are `weapons` / `terrain`
/// and whose other handles are inert defaults (the redrives under test read only those two
/// folders). The whole-session-persistent handles the editor's redrives rebuild from.
fn insert_handles(app: &mut App, weapons: Handle<LoadedFolder>, terrain: Handle<LoadedFolder>) {
    app.world_mut().insert_resource(EditorLoadHandles {
        theme:         EditorThemeHandle::new(Handle::<RonAsset<GdtfThemeSpec>>::default()),
        weapons:       EditorWeaponsFolderHandle::new(weapons),
        armor:         EditorArmorFolderHandle::new(Handle::<LoadedFolder>::default()),
        terrain_model: EditorTerrainModelFolderHandle::new(terrain),
        tile_roles:    EditorTileRolesHandle::new(Handle::<RonAsset<TileRoles>>::default()),
    });
}

/// A 128-bit constant → [`TerrainUuid`] (the def's own key).
const fn terrain_uuid(raw: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(raw))
}

/// A Cover-kind [`TerrainDef`] keyed by `key`, carrying `display` as its label.
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
        on_death: None,
    }
}

/// A stub_pistol-shaped [`WeaponSpec`] with the given `damage`, parsed from inline RON so the
/// test does not hand-assemble the `FireMode` `Vec`. Returns `None` (assert-fail) on a parse
/// error rather than a denied `unwrap` (the game weapon-redrive test precedent).
fn weapon_spec(damage: i32) -> Option<WeaponSpec> {
    let ron = format!(
        "(base_spread: 0.10, accuracy: 1.0, kickback: 0.05, fatal_bias: 0.0, \
         damage: {damage}, punch: 2, shred: 1, damage_type: Kinetic, \
         magazine: (size: 12, reload_tu: 12), \
         fire_mode: [(kind: Single, cone_mult: 1.0, tu_percent: 0.30, shots: 1)], \
         stable: false, handedness: OneHanded)",
    );
    let parsed = ron::de::from_str::<WeaponSpec>(&ron);
    assert!(
        parsed.is_ok(),
        "weapon fixture must parse: {:?}",
        parsed.as_ref().err()
    );
    parsed.ok()
}

/// C3 (terrain def): a `Modified` for a member `*.terrain_def.ron` REBUILDS the editor's
/// `TerrainDefRegistry` — keyed by the def's OWN `TerrainUuid` — reflecting the edited def,
/// through the editor's real registered redrive system (no restart).
///
/// Pin-discriminating: dropping the rebuild leaves the OLD display name; mis-keying drops the
/// entry.
#[test]
fn modified_member_rebuilds_editor_terrain_def_registry() {
    let mut app = app();
    let key = terrain_uuid(0x0533_0ed1_0001);
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<TerrainDef>>("terrain/industrial_hive/crate.terrain_def.ron");
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<TerrainDef>>>()
        .insert(handle.id(), RonAsset::new(cover_def(key, "Old Crate")));
    assert!(inserted.is_ok(), "member terrain def insert must succeed");

    let terrain_folder = add_folder(&mut app, vec![handle.clone().untyped()]);
    let empty_weapons = add_folder(&mut app, Vec::new());
    insert_handles(&mut app, empty_weapons, terrain_folder);
    // A stale baseline registry (empty) the rebuild must overwrite.
    app.world_mut()
        .insert_resource(TerrainDefRegistry::default());
    app.update();

    // Hot-edit the member def to a DISTINCT display name, fire Modified, rebuild.
    if let Some(mut asset) = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<TerrainDef>>>()
        .get_mut(&handle)
    {
        **asset = cover_def(key, "Edited Crate");
    }
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
    app.update();

    let rebuilt = app
        .world()
        .get_resource::<TerrainDefRegistry>()
        .and_then(|r| r.def(&key).map(|d| (*d.display_name).clone()));
    assert_eq!(
        rebuilt,
        Some("Edited Crate".to_owned()),
        "the editor terrain-def hot-reload must rebuild the registry, keyed by the def UUID, \
         with the edited def — no restart",
    );
}

/// C3 (weapons): a `Modified` for a member `*.weapon.ron` REBUILDS the editor's
/// `WeaponRegistry` — keyed by file stem — reflecting the edited spec, through the editor's
/// real registered redrive system (no restart).
///
/// Pin-discriminating: dropping the rebuild leaves the OLD damage; mis-keying drops the entry.
#[test]
fn modified_member_rebuilds_editor_weapon_registry() {
    let mut app = app();
    let Some(original) = weapon_spec(12) else {
        return;
    };
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<WeaponSpec>>("content/weapons/ranged/stub_pistol.weapon.ron");
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<WeaponSpec>>>()
        .insert(handle.id(), RonAsset::new(original));
    assert!(inserted.is_ok(), "member weapon spec insert must succeed");

    let weapons_folder = add_folder(&mut app, vec![handle.clone().untyped()]);
    let empty_terrain = add_folder(&mut app, Vec::new());
    insert_handles(&mut app, weapons_folder, empty_terrain);
    app.world_mut().insert_resource(WeaponRegistry::default());
    app.update();

    // Hot-edit the member spec to a DISTINCT damage, fire Modified, rebuild.
    let Some(edited) = weapon_spec(6) else { return };
    if let Some(mut asset) = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<WeaponSpec>>>()
        .get_mut(&handle)
    {
        **asset = edited;
    }
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });
    app.update();

    let key = WeaponName::new("stub_pistol".to_owned());
    let damage = app
        .world()
        .get_resource::<WeaponRegistry>()
        .and_then(|r| r.spec(&key).map(|s| *s.damage));
    assert_eq!(
        damage,
        Some(6),
        "the editor weapon hot-reload must rebuild the registry, keyed by stem, with the edited \
         damage — no restart",
    );
}

/// C3: the editor terrain-def hot-reload fires its `info!` line naming what reloaded. Run via
/// `run_system_once` on the calling thread so the thread-local `tracing` capture sees the
/// emission (the schedule executor may run on a worker thread the capture would miss).
///
/// Pin-discriminating: removing the `info!` leaves the capture empty.
#[test]
fn editor_terrain_def_hot_reload_logs_an_info_line() {
    let mut app = app();
    let key = terrain_uuid(0x0533_0ed1_0002);
    let handle = app
        .world()
        .resource::<AssetServer>()
        .load::<RonAsset<TerrainDef>>("terrain/industrial_hive/crate.terrain_def.ron");
    let inserted = app
        .world_mut()
        .resource_mut::<Assets<RonAsset<TerrainDef>>>()
        .insert(handle.id(), RonAsset::new(cover_def(key, "Crate")));
    assert!(inserted.is_ok(), "member terrain def insert must succeed");

    let terrain_folder = add_folder(&mut app, vec![handle.clone().untyped()]);
    let empty_weapons = add_folder(&mut app, Vec::new());
    insert_handles(&mut app, empty_weapons, terrain_folder);
    app.world_mut()
        .insert_resource(TerrainDefRegistry::default());
    app.world_mut()
        .write_message(AssetEvent::Modified { id: handle.id() });

    let captured = capture_logs(|| {
        let result = app
            .world_mut()
            .run_system_once(redrive_terrain_defs_on_asset_event);
        assert!(
            result.is_ok(),
            "the editor terrain-def redrive system must run cleanly"
        );
    });

    assert!(
        captured
            .iter()
            .any(|line| line.contains("editor terrain-def hot-reload")
                && line.contains("TerrainDefRegistry")),
        "the editor terrain-def hot-reload must emit an info! line naming what reloaded; \
         captured: {captured:?}",
    );
}
