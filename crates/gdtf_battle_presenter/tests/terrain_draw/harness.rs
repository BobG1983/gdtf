//! Shared `terrain_draw` fixture: the headless renderer app, async-load settling,
//! grid authoring, and the shared tile probes.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    ecs::error::warn,
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    TerrainFogMaterial, TerrainSprite, TileRoles, TopDownAtlases, TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    battle::BattleReady,
    cover::{CoverEntry, CoverHp, HeightBand},
    entity::TerrainCell,
    occupancy::{OccupancyInput, TerrainPlacement},
    occupancy_sync::{CoverDestroyed, SlabDestroyed},
    piece::{FootfallSound, TerrainGraphicKey},
    prelude::{CellLevel, Level, OccupancyGrid},
};
use gdtf_test_utils::advance_until_resource_exists;

/// Generous SAFETY-NET cap for the async atlas / tile-role loads polled by
/// [`settle_resources`]. It is a safety net against a genuine never-resolve hang, NOT a timing
/// budget: each gate resource is waited on by its inserted SIGNAL (not a fixed frame count),
/// which is what makes these draw tests deterministic under parallel `cargo` load (GTW-305).
pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

/// The workspace-root `assets/` directory (this crate's manifest → up two → assets),
/// the same root the running app uses so the shipped sheets + `tile_roles.ron` load.
pub(crate) fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`
/// (workspace `assets/`), the `TopDownRendererPlugin`, and the two sim message buffers
/// the draw reads. It does NOT add the sim's lifecycle systems — the test authors the
/// grids + `BattleInProgress` directly and writes `BattleReady` itself.
pub(crate) fn headless_renderer_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    // The draw + swap reactions read these buffers; the sim's BattleSimPlugin registers them
    // in the app, but this focused harness adds only the ones the suite needs. SlabDestroyed
    // is seeded here since GTW-623 C4/C5: the presenter no longer `add_message`s the sim-owned
    // buffer itself, and `swap_destroyed_slab` is `run_if`-gated on its presence — without this
    // seed the destruction-swap test's written message would be silently dropped.
    .add_message::<BattleReady>()
    .add_message::<CoverDestroyed>()
    .add_message::<SlabDestroyed>()
    .add_plugins(TopDownRendererPlugin);
    // Bevy 0.19 routes a FAILED system-param validation to the global error handler
    // (default panics); 0.18 silently SKIPPED. This no-renderer harness lacks the
    // render-provided resources some DefaultPlugins systems want (e.g. bevy_light's
    // update_gizmo_meshes -> Assets<GizmoAsset>), so `warn` restores the 0.18 skip
    // behavior instead of an intermittent headless panic.
    app.set_error_handler(warn);
    app
}

/// Drives `update()`s until `TileRoles` + `TopDownAtlases` are BOTH resident (the async
/// load chain has settled), polling each resource's inserted SIGNAL rather than a fixed frame
/// count (GTW-305). Both resolve over the same async `AssetServer` chain, so waiting for them
/// in sequence drives the app until the last is present. Panics (naming the missing resource)
/// via [`advance_until_resource_exists`] if either is still absent after the safety-net cap — a
/// genuine load failure, surfaced loudly rather than leaving the draw systems silently no-op.
pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<TileRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// Authors an `OccupancyGrid` with the given terrain placements (built through the real
/// `OccupancyInput` → `build_from_occupancy_input` path) and inserts it.
pub(crate) fn insert_occupancy(app: &mut App, terrain: Vec<TerrainPlacement>) {
    let input = OccupancyInput {
        terrain,
        occupants: Vec::new(),
    };
    // GTW-391: build_from_occupancy_input now takes a stair-cell set; pass empty
    // (no stair tiles in these presenter tests).
    let grid = OccupancyGrid::build_from_occupancy_input(
        &input,
        &bevy::platform::collections::HashSet::default(),
    );
    app.world_mut().insert_resource(grid);
}

/// Reads the resolved `TileRoles` resource as a clone, or `None` if it is absent (the
/// caller asserts it is `Some` — `settle_resources` already gated on its presence).
pub(crate) fn tile_roles(app: &App) -> Option<TileRoles> {
    app.world().get_resource::<TileRoles>().cloned()
}

/// A cover entry seeded for a Low prop (the AC2 cover cell), through the real ctor.
pub(crate) const fn low_cover_entry() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(30),
        HeightBand::Low,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

/// Reads the atlas index of the one `TerrainSprite` at `key`, if present (GTW-348 — the
/// tile renders through a `TerrainFogMaterial`, so the index is read off the material's
/// `atlas_index`, not a `Sprite`'s `TextureAtlas`).
pub(crate) fn sprite_index_at(app: &mut App, key: CellLevel) -> Option<usize> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let index = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .atlas_index;
    Some(index)
}

/// Reads the [`Entity`] id of the one `TerrainSprite` at `key`, if present — the C7
/// same-entity probe (the in-place swap must NOT despawn/respawn the tile, so the id read
/// before the destruction message must equal the id read after).
pub(crate) fn sprite_entity_at(app: &mut App, key: CellLevel) -> Option<bevy::ecs::entity::Entity> {
    let mut q = app
        .world_mut()
        .query::<(bevy::ecs::entity::Entity, &TerrainSprite)>();
    q.iter(app.world())
        .find(|(_, t)| t.at == key)
        .map(|(entity, _)| entity)
}

/// Spawns ONE sim-side terrain entity at `key` carrying its per-def
/// [`TerrainGraphicKey`] (and an OPTIONAL [`FootfallSound`]) — mirroring exactly what the
/// sim's `setup_battle` spawns onto every terrain entity (GTW-491). This is the seam the
/// GTW-493 presenter reads: the per-def graphic the draw resolves the cell's atlas index
/// from, ahead of the `TileRoles`-table default keyed only on `TerrainKind`.
pub(crate) fn spawn_terrain_entity(
    app: &mut App,
    key: CellLevel,
    graphic: &str,
    footfall: Option<&str>,
) {
    let mut entity = app.world_mut().spawn((
        TerrainCell::new(key),
        TerrainGraphicKey::new(graphic.to_owned()),
    ));
    if let Some(footfall) = footfall {
        entity.insert(FootfallSound::new(footfall.to_owned()));
    }
}

/// Counts the `TerrainSprite`s currently drawn on `level` (`at.z == level`).
pub(crate) fn terrain_sprite_count_on_level(app: &mut App, level: Level) -> usize {
    let z = i32::from(*level);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).filter(|t| t.at.z == z).count()
}
