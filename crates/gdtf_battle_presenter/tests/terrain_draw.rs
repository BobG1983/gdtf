//! GTW-218 (GTW-48 S4): headless draw-LOGIC tests for the static-battlefield terrain
//! draw — AC2 (one-shot draw of role-correct, cell-positioned, CELL_PX-sized sprites),
//! AC3 (a `CoverDestroyed` swaps the cover cell to the rubble tile), and (GTW-519) the
//! multi-level draw core: raising `ActiveLevel` redraws the whole drawn band `[0..=active]`
//! (C1/C7), an open upper-storey cell peeks through to the storey below (C2), a tile on
//! storey `k+1` sorts strictly in front of the same `(x,y)` on storey `k` (per-storey Z, C3),
//! and a cover / slab destroyed on a DRAWN lower storey still swaps its tile (C6).
//!
//! These prove the DRAW LOGIC headless; "the right tiles appear on screen" is AC6's
//! in-engine QA evidence. The harness is a `DefaultPlugins`/`no_renderer` app (a live
//! `AssetServer` rooted at the workspace `assets/` so `TopDownAtlases` + the
//! `tile_roles.ron`-resolved `TileRoles` are resident) plus `TopDownRendererPlugin` and
//! the two sim message buffers the draw reads (`BattleReady` / `CoverDestroyed`). The
//! three grids + `BattleInProgress` are authored, and `BattleReady` is written, DIRECTLY
//! via `app.world_mut()` in the test body — the accepted headless idiom (`bevy-traps.md`
//! #7 carve-out (a)). No function here takes `&mut World`/`&World`.

use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::{AssetPlugin, Assets},
    ecs::{error::warn, message::Messages},
    math::Vec2,
    platform::collections::HashSet,
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, CELL_PX, TerrainFogMaterial, TerrainSprite, TileRoles, TopDownAtlases,
    TopDownRendererPlugin, ViewMode, cell_to_world,
};
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, BattleInProgress, BattleReady, Cell, CellLevel, CombatTuning,
    CoverDestroyed, CoverEntry, CoverHp, CoverLedger, EmplacementState, FootfallSound, HeightBand,
    Level, OccupancyGrid, OccupancyInput, SlabDestroyed, SlabState, SquadVisibility, SurfaceGrid,
    TerrainCell, TerrainGraphicKey, TerrainKind, TerrainPlacement,
};
use gdtf_test_utils::advance_until_resource_exists;

/// Generous SAFETY-NET cap for the async atlas / tile-role loads polled by
/// [`settle_resources`]. It is a safety net against a genuine never-resolve hang, NOT a timing
/// budget: each gate resource is waited on by its inserted SIGNAL (not a fixed frame count),
/// which is what makes these draw tests deterministic under parallel `cargo` load (GTW-305).
const LOAD_SAFETY_NET: u32 = 10_000;

/// The workspace-root `assets/` directory (this crate's manifest → up two → assets),
/// the same root the running app uses so the shipped sheets + `tile_roles.ron` load.
fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

/// Builds a headless `DefaultPlugins`/`no_renderer` app with a live `AssetServer`
/// (workspace `assets/`), the `TopDownRendererPlugin`, and the two sim message buffers
/// the draw reads. It does NOT add the sim's lifecycle systems — the test authors the
/// grids + `BattleInProgress` directly and writes `BattleReady` itself.
fn headless_renderer_app() -> App {
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
    // The draw reads these two buffers; the sim's BattleSimPlugin registers them in the
    // app, but this focused harness adds only the two the draw needs.
    .add_message::<BattleReady>()
    .add_message::<CoverDestroyed>()
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
fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<TileRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
}

/// Authors an `OccupancyGrid` with the given terrain placements (built through the real
/// `OccupancyInput` → `build_from_occupancy_input` path) and inserts it.
fn insert_occupancy(app: &mut App, terrain: Vec<TerrainPlacement>) {
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
fn tile_roles(app: &App) -> Option<TileRoles> {
    app.world().get_resource::<TileRoles>().cloned()
}

/// A cover entry seeded for a Low prop (the AC2 cover cell), through the real ctor.
const fn low_cover_entry() -> CoverEntry {
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
fn sprite_index_at(app: &mut App, key: CellLevel) -> Option<usize> {
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
fn sprite_entity_at(app: &mut App, key: CellLevel) -> Option<bevy::ecs::entity::Entity> {
    let mut q = app
        .world_mut()
        .query::<(bevy::ecs::entity::Entity, &TerrainSprite)>();
    q.iter(app.world())
        .find(|(_, t)| t.at == key)
        .map(|(entity, _)| entity)
}

/// Reads the (material `custom_size`, entity translation) of the one `TerrainSprite` at
/// `key` (GTW-348 — `custom_size` lives on the `TerrainFogMaterial`, the `Transform` stays
/// on the entity).
fn sprite_geometry_at(app: &mut App, key: CellLevel) -> Option<(Option<Vec2>, bevy::math::Vec3)> {
    let mut q = app.world_mut().query::<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &Transform,
    )>();
    let (handle, translation) = q
        .iter(app.world())
        .find(|(t, ..)| t.at == key)
        .map(|(_, mat, transform)| (mat.id(), transform.translation))?;
    let custom_size = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .custom_size;
    Some((custom_size, translation))
}

/// Counts the `TerrainSprite` entities currently in the world.
fn terrain_sprite_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).count()
}

/// AC2 — on `BattleReady`, the static draw spawns one role-correct, CELL_PX-sized,
/// cell-positioned terrain sprite per non-empty cell on the active level.
///
/// Authors exactly one `Wall (8,7,0)`, one `Cover (9,8,0)`, and one `Present` slab
/// `(2,2,0)` on the active level (level 0), then writes `BattleReady` and updates once.
/// For each, asserts (a) the sprite's atlas index equals the `TileRoles` role index READ
/// FROM THE RESOURCE (structural, never a literal), (b) `custom_size ==
/// Some(Vec2::splat(CELL_PX))`, and (c) `Transform.translation == cell_to_world(cell,
/// L0)`.
#[test]
fn battle_ready_draws_role_correct_sized_positioned_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let wall_cell = Cell::new(8, 7);
    let cover_cell = Cell::new(9, 8);
    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);
    let cover_key = CellLevel::new(cover_cell, l0);
    let slab_key = CellLevel::new(slab_cell, l0);

    // Author the three grids + the in-progress witness directly (the headless idiom).
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(wall_key, TerrainKind::Wall),
            TerrainPlacement::new(cover_key, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_key, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // Fire the one-shot.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // Read the resolved role indices FROM the resource — never a hardcoded literal.
    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // (a) role-correct atlas indices, structural against the resource.
    assert_eq!(
        sprite_index_at(&mut app, wall_key),
        Some(*roles.wall),
        "the wall cell's sprite index must equal the resource's wall role index",
    );
    assert_eq!(
        sprite_index_at(&mut app, cover_key),
        Some(*roles.cover),
        "the cover cell's sprite index must equal the resource's cover role index",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab_key),
        Some(*roles.slab),
        "the slab cell's sprite index must equal the resource's slab role index",
    );

    // (b) + (c) sizing + positioning for the wall cell (representative).
    let geometry = sprite_geometry_at(&mut app, wall_key);
    assert!(geometry.is_some(), "the wall sprite must be present");
    let Some((size, translation)) = geometry else {
        return;
    };
    assert_eq!(
        size,
        Some(Vec2::splat(CELL_PX)),
        "every terrain sprite must be custom_size Some(Vec2::splat(CELL_PX))",
    );
    assert_eq!(
        translation,
        cell_to_world(wall_cell, l0),
        "the wall sprite must be positioned at cell_to_world(cell, L0)",
    );

    // A non-empty cell is at least floor: the active level draws far more than three
    // sprites (the whole 60x60 floor field), so the count is large — proves the scan
    // ran across the grid, not just the three authored cells.
    assert!(
        terrain_sprite_count(&mut app) > 3,
        "the active level draws a floor field, not only the three authored facts",
    );
}

/// AC3 — a `CoverDestroyed { at }` on the active level swaps that cover cell's sprite to
/// the `rubble` tile (read from `TileRoles`), and the other terrain sprites are untouched.
#[test]
fn cover_destroyed_swaps_the_cover_cell_to_rubble() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let wall_cell = Cell::new(8, 7);
    let cover_cell = Cell::new(9, 8);
    let l0 = Level::new(0);
    let wall_key = CellLevel::new(wall_cell, l0);
    let cover_key = CellLevel::new(cover_cell, l0);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(wall_key, TerrainKind::Wall),
            TerrainPlacement::new(cover_key, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_key, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident");
    let Some(roles) = roles else { return };
    let wall_index_before = sprite_index_at(&mut app, wall_key);

    // Smash the cover cell.
    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(cover_key));
    app.update();

    assert_eq!(
        sprite_index_at(&mut app, cover_key),
        Some(*roles.rubble),
        "the destroyed cover cell's sprite must now be the rubble tile index",
    );
    assert_eq!(
        sprite_index_at(&mut app, wall_key),
        wall_index_before,
        "the other (wall) terrain sprite must be untouched by the cover destruction",
    );
}

/// GTW-367 C6/C7 (POSITIVE) — a `SlabDestroyed { at }` on the active level, fed through the
/// REAL presenter plugin's `swap_destroyed_slab` reaction, swaps that slab cell's sprite to
/// the `slab_destroyed` tile (read from `TileRoles`) IN PLACE — the SAME `Entity` persists
/// (no despawn/respawn), the destroyed index is DISTINCT from the intact slab index, and the
/// neighbouring slab cell is untouched.
///
/// Drives the production `TopDownRendererPlugin` (no hand-mutated sprite in the arrange):
/// authors two `Present` slabs, fires `BattleReady` + `update()`s so the real draw spawns the
/// tiles, then writes a real `SlabDestroyed` to its `Messages` buffer and `update()`s again so
/// `swap_destroyed_slab` runs (the settle-before-read rule) before the assert reads the
/// resulting material `atlas_index` + entity id.
#[test]
fn slab_destroyed_swaps_the_slab_cell_to_destroyed_in_place() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(4, 5);
    let other_slab_cell = Cell::new(6, 7);
    let l0 = Level::new(0);
    let slab_key = CellLevel::new(slab_cell, l0);
    let other_slab_key = CellLevel::new(other_slab_cell, l0);

    // Author two Present slabs (no walls/cover) + the in-progress witness. The destroyed
    // slab and an untouched neighbour, so the test proves the swap is targeted.
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key, SlabState::Present);
    surface.set_slab(other_slab_key, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // Fire the one-shot draw so the real plugin spawns the terrain tiles.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the destroyed treatment is a REAL visible swap (distinct atlas index).
    assert_ne!(
        *roles.slab_destroyed, *roles.slab,
        "the slab_destroyed tile index must differ from the intact slab index (a real swap)",
    );

    // The slab cell starts on the intact slab tile, and we capture its Entity id for the C7
    // same-entity check.
    assert_eq!(
        sprite_index_at(&mut app, slab_key),
        Some(*roles.slab),
        "the slab cell's sprite must start on the intact slab tile index",
    );
    let entity_before = sprite_entity_at(&mut app, slab_key);
    assert!(
        entity_before.is_some(),
        "the slab cell's terrain sprite must exist before the destruction",
    );
    let other_index_before = sprite_index_at(&mut app, other_slab_key);

    // Smash the slab — write a REAL message to the buffer the plugin registered, then settle.
    app.world_mut()
        .resource_mut::<Messages<SlabDestroyed>>()
        .write(SlabDestroyed::new(slab_key));
    app.update();

    // POSITIVE: the slab cell now renders the destroyed-slab tile index (the intended content
    // actually renders — not merely "something changed").
    assert_eq!(
        sprite_index_at(&mut app, slab_key),
        Some(*roles.slab_destroyed),
        "the destroyed slab cell's sprite must now be the slab_destroyed tile index",
    );
    // C7: the SAME entity persists (in-place mutation, no despawn/respawn).
    assert_eq!(
        sprite_entity_at(&mut app, slab_key),
        entity_before,
        "the destroyed slab cell must be the SAME Entity after the swap (no despawn/respawn)",
    );
    // The neighbouring slab cell is untouched.
    assert_eq!(
        sprite_index_at(&mut app, other_slab_key),
        other_index_before,
        "the other (intact) slab sprite must be untouched by the slab destruction",
    );
}

/// GTW-519 C1/C7 — raising `ActiveLevel` redraws the WHOLE drawn band `[0..=active]`
/// (multi-level, bottom-up), culling everything strictly above `active`; nothing above the
/// band is drawn.
///
/// Authors a `Present` slab on level 0 AND one on level 1 (mirroring skirmish's `(2,2,0)` +
/// `(2,2,1)`). At `ActiveLevel` 0 only the level-0 slab draws and the level-1 slab is CULLED
/// (strictly above active); after raising `ActiveLevel` to 1, BOTH slabs draw (level 0 is a
/// DRAWN lower storey now, not despawned — the pre-GTW-519 single-active-level behaviour is
/// replaced by the UFO:EU band). Every drawn sprite lies WITHIN the band (`z <= active`).
#[test]
fn raising_active_level_redraws_the_whole_drawn_band() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let slab0 = CellLevel::new(slab_cell, l0);
    let slab1 = CellLevel::new(slab_cell, l1);

    // Empty occupancy: the ground plane (storey 0) draws its full floor field; the upper
    // storey draws ONLY its authored slab (peek-through, C2). The band-scoping is proven by
    // the slab sprites' PRESENCE per level and by every drawn sprite lying within the band.
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab0, SlabState::Present);
    surface.set_slab(slab1, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident");
    let Some(roles) = roles else { return };

    // At active level 0: the level-0 slab draws; the level-1 slab is CULLED (above active).
    assert_eq!(
        sprite_index_at(&mut app, slab0),
        Some(*roles.slab),
        "the level-0 slab sprite must be present at active level 0",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab1),
        None,
        "the level-1 slab sprite (strictly ABOVE active) must be CULLED at active level 0",
    );
    // Every drawn sprite is within the band [0..=0]: nothing above active.
    assert!(
        all_sprites_within_band(&mut app, l0),
        "every terrain sprite must be within the drawn band [0..=0] before the change",
    );

    // Raise the active level to 1.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.update();

    // BOTH slabs now draw: level 1 is the active storey, level 0 is a DRAWN lower storey
    // (NOT despawned — the multi-level band redraw, C1/C7).
    assert_eq!(
        sprite_index_at(&mut app, slab1),
        Some(*roles.slab),
        "after raising to level 1, the level-1 (active) slab sprite must be present",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab0),
        Some(*roles.slab),
        "after raising to level 1, the level-0 slab sprite must STILL be present (a drawn \
         lower storey, not despawned)",
    );
    // Every drawn sprite is within the band [0..=1]: nothing above active.
    assert!(
        all_sprites_within_band(&mut app, l1),
        "after raising to level 1, every terrain sprite must be within the drawn band [0..=1]",
    );
}

/// Whether every `TerrainSprite` in the world lies WITHIN the drawn band `[0..=active]`
/// (`at.z <= active`) — the GTW-519 multi-level cull check (nothing strictly above active).
fn all_sprites_within_band(app: &mut App, active: Level) -> bool {
    let ceiling = i32::from(*active);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).all(|t| t.at.z <= ceiling)
}

/// Spawns ONE sim-side terrain entity at `key` carrying its per-def
/// [`TerrainGraphicKey`] (and an OPTIONAL [`FootfallSound`]) — mirroring exactly what the
/// sim's `setup_battle` spawns onto every terrain entity (GTW-491). This is the seam the
/// GTW-493 presenter reads: the per-def graphic the draw resolves the cell's atlas index
/// from, ahead of the `TileRoles`-table default keyed only on `TerrainKind`.
fn spawn_terrain_entity(app: &mut App, key: CellLevel, graphic: &str, footfall: Option<&str>) {
    let mut entity = app.world_mut().spawn((
        TerrainCell::new(key),
        TerrainGraphicKey::new(graphic.to_owned()),
    ));
    if let Some(footfall) = footfall {
        entity.insert(FootfallSound::new(footfall.to_owned()));
    }
}

/// GTW-493 C1 (PIN-DISCRIMINATING, the in-engine evidence) — two cells of the SAME
/// `TerrainKind` (`Cover`) but DIFFERENT `presenter_kind.graphic_name` resolve to DISTINCT
/// atlas indices, driven through the REAL `draw_static_battlefield` system.
///
/// Both cells are authored `TerrainKind::Cover` in the occupancy grid, so the OLD
/// `TileRoles`-table-only resolution (keyed solely on `TerrainKind`) would draw them
/// IDENTICALLY at `roles.cover`. The sim-spawned per-def `TerrainGraphicKey` ("cover" vs
/// "rubble") is what makes them DIFFER: cell A resolves to `roles.cover`, cell B to
/// `roles.rubble`. Reverting the presenter to role-table-only resolution makes both
/// `roles.cover` — identical — and this test FAILS.
///
/// Occlusion-aware (a visible+laid-out node can still draw nothing): the assertion reads
/// the resolved material `atlas_index` actually carried by the spawned sprite at each cell
/// (`sprite_index_at`), and settles a frame (the one-shot draw) before reading.
#[test]
fn per_def_graphic_distinguishes_same_kind_cells() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let cover_a = CellLevel::new(Cell::new(8, 7), l0);
    let cover_b = CellLevel::new(Cell::new(9, 8), l0);

    // Both cells are the SAME TerrainKind::Cover in the occupancy grid — so the role-table
    // default keyed on TerrainKind would draw both at roles.cover.
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(cover_a, TerrainKind::Cover),
            TerrainPlacement::new(cover_b, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_a, low_cover_entry());
    cover_ledger.insert(cover_b, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: SAME kind, DISTINCT graphic_names.
    spawn_terrain_entity(&mut app, cover_a, "cover", None);
    spawn_terrain_entity(&mut app, cover_b, "rubble", None);

    // Fire the one-shot draw and settle.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the two graphic roles are DISTINCT indices in the shipped table (so a
    // "different" assertion is meaningful rather than vacuously true).
    assert_ne!(
        *roles.cover, *roles.rubble,
        "the `cover` and `rubble` role indices must differ (else the pin is vacuous)",
    );

    let index_a = sprite_index_at(&mut app, cover_a);
    let index_b = sprite_index_at(&mut app, cover_b);

    // POSITIVE: each cell resolves to ITS OWN def graphic, not the shared TerrainKind default.
    assert_eq!(
        index_a,
        Some(*roles.cover),
        "cell A (graphic_name \"cover\") must resolve to the `cover` atlas index",
    );
    assert_eq!(
        index_b,
        Some(*roles.rubble),
        "cell B (graphic_name \"rubble\") must resolve to the `rubble` atlas index, NOT the \
         shared TerrainKind::Cover default",
    );
    // The DISCRIMINATING clause: two same-TerrainKind cells draw DIFFERENT sprites — exactly
    // what role-table-only resolution (keyed on TerrainKind) could not do.
    assert_ne!(
        index_a, index_b,
        "two cells of the SAME TerrainKind but DIFFERENT graphic_name must draw DIFFERENT \
         sprites (role-table-only resolution would make them identical and fail here)",
    );
}

/// GTW-469 C3 (PIN-DISCRIMINATING, the in-engine evidence) — an NS-wall cell and an EW-wall
/// cell, BOTH `TerrainKind::Wall` but DIFFERENT `presenter_kind.graphic_name` (`"wall"` vs
/// `"wall_ew"`), resolve to DISTINCT atlas indices, driven through the REAL
/// `draw_static_battlefield` system.
///
/// Both cells are authored `TerrainKind::Wall` in the occupancy grid, so the
/// `TileRoles`-table-only resolution (keyed solely on `TerrainKind`) would draw them
/// IDENTICALLY at `roles.wall` — orientation is presentation-only, so the sim semantics ARE
/// identical (C5). The sim-spawned per-def `TerrainGraphicKey` (`"wall"` vs `"wall_ew"`) is what
/// makes the SPRITES differ: the NS cell resolves to `roles.wall`, the EW cell to `roles.wall_ew`
/// (the row-2 rotated tile). Reverting the presenter to role-table-only resolution makes both
/// `roles.wall` — identical — and this test FAILS.
///
/// Occlusion-aware (a visible+laid-out node can still draw nothing): the assertion reads the
/// resolved material `atlas_index` actually carried by the spawned sprite at each cell
/// (`sprite_index_at`), and settles a frame (the one-shot draw) before reading.
#[test]
fn ns_and_ew_wall_resolve_to_distinct_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let wall_ns = CellLevel::new(Cell::new(8, 7), l0);
    let wall_ew = CellLevel::new(Cell::new(9, 8), l0);

    // Both cells are the SAME TerrainKind::Wall in the occupancy grid — so the role-table
    // default keyed on TerrainKind would draw both at roles.wall (identical LOS/move blocking).
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(wall_ns, TerrainKind::Wall),
            TerrainPlacement::new(wall_ew, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: SAME TerrainKind::Wall, DISTINCT graphic_names —
    // the NS def's "wall" vs the GTW-469 EW def's "wall_ew".
    spawn_terrain_entity(&mut app, wall_ns, "wall", None);
    spawn_terrain_entity(&mut app, wall_ew, "wall_ew", None);

    // Fire the one-shot draw and settle.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the two wall-orientation roles are DISTINCT indices in the shipped table
    // (so a "different" assertion is meaningful rather than vacuously true).
    assert_ne!(
        *roles.wall, *roles.wall_ew,
        "the `wall` (NS) and `wall_ew` (EW) role indices must differ (else the pin is vacuous)",
    );

    let index_ns = sprite_index_at(&mut app, wall_ns);
    let index_ew = sprite_index_at(&mut app, wall_ew);

    // POSITIVE: each cell resolves to ITS OWN orientation graphic, not the shared Wall default.
    assert_eq!(
        index_ns,
        Some(*roles.wall),
        "the NS-wall cell (graphic_name \"wall\") must resolve to the `wall` atlas index",
    );
    assert_eq!(
        index_ew,
        Some(*roles.wall_ew),
        "the EW-wall cell (graphic_name \"wall_ew\") must resolve to the `wall_ew` atlas index, \
         NOT the shared TerrainKind::Wall default",
    );
    // The DISCRIMINATING clause: two same-TerrainKind::Wall cells draw DIFFERENT (perpendicular)
    // sprites — exactly what role-table-only resolution (keyed on TerrainKind) could not do.
    assert_ne!(
        index_ns, index_ew,
        "an NS-wall cell and an EW-wall cell (SAME TerrainKind::Wall, DIFFERENT graphic_name) \
         must draw DIFFERENT sprites (role-table-only resolution would make them identical)",
    );
}

/// GTW-470 C3 (PIN-DISCRIMINATING, the in-engine evidence) — the 6 orientation/direction
/// door + stair tiles each resolve to a DISTINCT atlas index, both from EACH OTHER and from
/// the existing terrain tiles (`wall`/`cover`/`slab`/`floor`/`rubble`), driven through the
/// REAL `draw_static_battlefield` system.
///
/// The 2 doors are authored `TerrainKind::Wall` cells and the 4 stairs are authored slab
/// cells (matching their `sim_kind` — doors are `Wall`, stairs are `Slab`), so the
/// `TileRoles`-table-only resolution (keyed solely on `TerrainKind`) would collapse all 2
/// doors onto `roles.wall` and all 4 stairs onto `roles.slab`. The sim-spawned per-def
/// `TerrainGraphicKey` (`"door_ns"`/`"door_ew"`/`"stair_ns_up"`/`"stair_ns_down"`/
/// `"stair_ew_up"`/`"stair_ew_down"`) is what makes the SPRITES differ: each resolves to its
/// own orientation/direction index. ANY two of the six collapsing to the same sprite FAILS;
/// any one collapsing onto an existing tile (wall/cover/slab/floor/rubble) FAILS.
///
/// Occlusion-aware (a visible+laid-out node can still draw nothing): the assertion reads the
/// resolved material `atlas_index` actually carried by the spawned sprite at each cell
/// (`sprite_index_at`), and settles a frame (the one-shot draw) before reading.
#[test]
fn door_and_stair_orientation_tiles_resolve_to_distinct_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    // The six new cells: 2 doors (Wall) + 4 stairs (Slab). Distinct cells so each draws its own
    // terrain sprite.
    let door_ns = CellLevel::new(Cell::new(3, 3), l0);
    let door_ew = CellLevel::new(Cell::new(4, 3), l0);
    let stair_ns_up = CellLevel::new(Cell::new(5, 3), l0);
    let stair_ns_down = CellLevel::new(Cell::new(6, 3), l0);
    let stair_ew_up = CellLevel::new(Cell::new(7, 3), l0);
    let stair_ew_down = CellLevel::new(Cell::new(8, 3), l0);

    // Doors are TerrainKind::Wall in the occupancy grid; stairs are Present slabs (matching the
    // sim_kind), so the TerrainKind-keyed role default would collapse the doors onto roles.wall
    // and the stairs onto roles.slab.
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(door_ns, TerrainKind::Wall),
            TerrainPlacement::new(door_ew, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(stair_ns_up, SlabState::Present);
    surface.set_slab(stair_ns_down, SlabState::Present);
    surface.set_slab(stair_ew_up, SlabState::Present);
    surface.set_slab(stair_ew_down, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: each cell names its own orientation/direction graphic.
    spawn_terrain_entity(&mut app, door_ns, "door_ns", None);
    spawn_terrain_entity(&mut app, door_ew, "door_ew", None);
    spawn_terrain_entity(&mut app, stair_ns_up, "stair_ns_up", None);
    spawn_terrain_entity(&mut app, stair_ns_down, "stair_ns_down", None);
    spawn_terrain_entity(&mut app, stair_ew_up, "stair_ew_up", None);
    spawn_terrain_entity(&mut app, stair_ew_down, "stair_ew_down", None);

    // Fire the one-shot draw and settle.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Each cell resolves to ITS OWN per-def orientation/direction graphic.
    assert_eq!(
        sprite_index_at(&mut app, door_ns),
        Some(*roles.door_ns),
        "the NS-door cell must resolve to the `door_ns` atlas index",
    );
    assert_eq!(
        sprite_index_at(&mut app, door_ew),
        Some(*roles.door_ew),
        "the EW-door cell must resolve to the `door_ew` atlas index",
    );
    assert_eq!(
        sprite_index_at(&mut app, stair_ns_up),
        Some(*roles.stair_ns_up),
        "the NS-up-stair cell must resolve to the `stair_ns_up` atlas index",
    );
    assert_eq!(
        sprite_index_at(&mut app, stair_ns_down),
        Some(*roles.stair_ns_down),
        "the NS-down-stair cell must resolve to the `stair_ns_down` atlas index",
    );
    assert_eq!(
        sprite_index_at(&mut app, stair_ew_up),
        Some(*roles.stair_ew_up),
        "the EW-up-stair cell must resolve to the `stair_ew_up` atlas index",
    );
    assert_eq!(
        sprite_index_at(&mut app, stair_ew_down),
        Some(*roles.stair_ew_down),
        "the EW-down-stair cell must resolve to the `stair_ew_down` atlas index",
    );

    // The DISCRIMINATING clause: all 6 new indices are mutually DISTINCT, AND distinct from the
    // existing terrain tiles (wall / cover / slab / floor / rubble). Any collision FAILS.
    assert_orientation_indices_all_distinct(&roles);
}

/// The DISCRIMINATING half of [`door_and_stair_orientation_tiles_resolve_to_distinct_sprites`]
/// (extracted so the test body stays under the `too_many_lines` lint): every GTW-470 door/stair
/// orientation index is DISTINCT from each other AND from every existing terrain tile index — any
/// collision means an orientation/direction would be indistinguishable, or reuses an existing tile.
fn assert_orientation_indices_all_distinct(roles: &TileRoles) {
    let new_indices = [
        ("door_ns", *roles.door_ns),
        ("door_ew", *roles.door_ew),
        ("stair_ns_up", *roles.stair_ns_up),
        ("stair_ns_down", *roles.stair_ns_down),
        ("stair_ew_up", *roles.stair_ew_up),
        ("stair_ew_down", *roles.stair_ew_down),
    ];
    let existing = [
        ("wall", *roles.wall),
        ("wall_ew", *roles.wall_ew),
        ("cover", *roles.cover),
        ("slab", *roles.slab),
        ("floor", *roles.floor),
        ("rubble", *roles.rubble),
        ("door", *roles.door),
        ("stair_up", *roles.stair_up),
        ("stair_down", *roles.stair_down),
        ("ladder", *roles.ladder),
    ];
    for (i, (na, a)) in new_indices.iter().enumerate() {
        for (nb, b) in new_indices.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "the new orientation tiles {na} and {nb} must be DISTINCT sprites (collapsing \
                 two onto one index would mean an orientation/direction is indistinguishable)",
            );
        }
        for (ne, e) in &existing {
            assert_ne!(
                a, e,
                "the new orientation tile {na} must be DISTINCT from the existing {ne} tile \
                 (it is its own AUTHORED sprite, never a reuse of an existing terrain index)",
            );
        }
    }
}

/// GTW-493 C2 — a `Slab` cell's footfall is read from the def's `presenter_kind` (an
/// OPTIONAL `FootfallSound`), and an ABSENT footfall is handled with NO panic and a
/// documented default.
///
/// Drives the REAL `draw_static_battlefield` over two slab cells: one whose terrain entity
/// names a footfall, one whose entity OMITS it (the `None` footfall — the documented
/// silent default). The draw reading the footfall must NOT panic on either, and both slab
/// cells must still render their per-def graphic (here both `"slab"`), proving the footfall
/// read is a non-fatal presentation hook layered onto the same draw.
#[test]
fn slab_footfall_optional_is_read_without_panic() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let slab_with = CellLevel::new(Cell::new(4, 5), l0);
    let slab_without = CellLevel::new(Cell::new(6, 7), l0);

    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_with, SlabState::Present);
    surface.set_slab(slab_without, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // One slab def names a footfall; the other OMITS it (the documented silent default).
    spawn_terrain_entity(&mut app, slab_with, "slab", Some("step_metal"));
    spawn_terrain_entity(&mut app, slab_without, "slab", None);

    // Fire the one-shot draw and settle — the draw reading the OPTIONAL footfall must not
    // panic for either the present-footfall or the absent-footfall slab.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Both slab cells render their per-def `slab` graphic — the absent-footfall slab draws
    // exactly like the present-footfall one (the footfall is a non-visual hook; its absence
    // is the silent default, never a missing tile).
    assert_eq!(
        sprite_index_at(&mut app, slab_with),
        Some(*roles.slab),
        "the slab cell WITH a footfall must render the `slab` graphic",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab_without),
        Some(*roles.slab),
        "the slab cell WITHOUT a footfall must STILL render the `slab` graphic (absent \
         footfall is the documented silent default, not a missing tile)",
    );
}

// ─────────────────────────────────────────────────────────────────────────────────────────
// GTW-519 — UFO:EU-style multi-level terrain draw
// ─────────────────────────────────────────────────────────────────────────────────────────

/// Counts the `TerrainSprite`s currently drawn on `level` (`at.z == level`).
fn terrain_sprite_count_on_level(app: &mut App, level: Level) -> usize {
    let z = i32::from(*level);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).filter(|t| t.at.z == z).count()
}

/// The `Transform.translation.z` of the one `TerrainSprite` at `key`, if present — the C3
/// per-storey Z probe.
fn sprite_z_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, transform)| transform.translation.z)
}

/// The `TerrainFogMaterial.brightness` (as a bare `f32`) of the one `TerrainSprite` at `key`,
/// if present — the C4 storey-darken probe. `Brightness` `Deref`s to its `f32`.
fn sprite_brightness_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let brightness = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .brightness;
    Some(*brightness)
}

/// The `TerrainFogMaterial.saturation` of the one `TerrainSprite` at `key`, if present — the
/// C5 fog-composes probe.
fn sprite_saturation_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &MeshMaterial2d<TerrainFogMaterial>)>();
    let handle = q
        .iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, mat)| mat.id())?;
    let saturation = app
        .world()
        .get_resource::<Assets<TerrainFogMaterial>>()?
        .get(handle)?
        .saturation;
    Some(saturation)
}

/// Overwrite the `SquadVisibility` fog with the given VISIBLE / EXPLORED cells (VISIBLE is
/// folded into EXPLORED to honour the `visible ⊆ explored` accrual invariant), so `present_fog`
/// (which the draw harness now needs to run for the brightness/saturation drive) has real
/// fog to modulate against.
fn set_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible_set: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible_set.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible_set, explored_set));
}

/// GTW-519 C1 — the multi-level draw spawns terrain for EVERY storey in `[0..=active]`
/// (bottom-up) and NONE strictly above `active`.
///
/// Authors a `Present` slab on storeys 0, 1, and 2, sets `ActiveLevel` to 1, fires the draw,
/// and asserts: storey 0 (a lower drawn storey) draws its full floor field (`> 3` sprites),
/// storey 1 (the active storey) draws its full floor field, and storey 2 (strictly above
/// active) draws ZERO sprites (the hard cull). The multi-level presence proves the loop
/// iterates the whole band, not a single active storey.
#[test]
fn multi_level_draws_the_band_below_and_at_active_and_none_above() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let l2 = Level::new(2);

    // Empty occupancy: the ground floor draws a full floor field; upper storeys draw only the
    // real terrain we author (a slab) — the peek-through invariant (C2, proven separately).
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(CellLevel::new(cell, l0), SlabState::Present);
    surface.set_slab(CellLevel::new(cell, l1), SlabState::Present);
    surface.set_slab(CellLevel::new(cell, l2), SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // Active view level = 1: draw storeys 0 and 1, cull 2.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // The ground floor (storey 0) draws its FULL floor field (the ground plane) — proof the
    // band extends below active, not a single storey.
    let ground_count = terrain_sprite_count_on_level(&mut app, l0);
    assert!(
        ground_count > 3,
        "storey 0 (the ground floor, below active) must draw its full floor field in the \
         multi-level band; got {ground_count}",
    );
    // The active storey (an UPPER storey) draws ONLY its real terrain (the one authored slab),
    // NOT a full floor field — the peek-through invariant (C2) applies to every storey above
    // the ground plane, active or not. So it draws exactly 1 (the slab), far fewer than the
    // ground floor's field.
    let active_count = terrain_sprite_count_on_level(&mut app, l1);
    assert_eq!(
        active_count, 1,
        "storey 1 (active, an upper storey) must draw ONLY its real terrain (the slab), not a \
         floor field — peek-through applies to every non-ground storey; got {active_count}",
    );
    // Everything strictly ABOVE active is culled — ZERO sprites on storey 2.
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l2),
        0,
        "storey 2 (strictly above active) must draw NOTHING (the hard cull)",
    );
}

/// GTW-519 C2 (peek-through) — an open/empty UPPER-storey cell emits NO sprite, while the
/// same `(x, y)` on the storey BENEATH it DOES emit; a real terrain fact on the upper storey
/// still emits.
///
/// At `ActiveLevel` 1: authors a slab at `(5,5,0)` (ground floor) with NOTHING at `(5,5,1)`
/// (an empty upper cell — a floor gap), plus a wall at `(7,7,1)` (real upper terrain). Asserts
/// `(5,5,1)` emits NONE (peek-through — the storey-0 cell reads through), `(5,5,0)` emits Some,
/// and `(7,7,1)` emits Some (real terrain still draws on the upper storey).
#[test]
fn upper_storey_gap_peeks_through_to_the_storey_beneath() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let gap_lower = CellLevel::new(Cell::new(5, 5), l0);
    let gap_upper = CellLevel::new(Cell::new(5, 5), l1);
    let wall_upper = CellLevel::new(Cell::new(7, 7), l1);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(wall_upper, TerrainKind::Wall)],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(gap_lower, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // The empty upper cell emits NOTHING — the floor-gap reveals the storey beneath (C2).
    assert_eq!(
        sprite_index_at(&mut app, gap_upper),
        None,
        "an open/empty upper-storey cell must emit NO sprite (peek-through)",
    );
    // The same (x,y) on the storey BENEATH it DOES emit (its slab) — the revealed cell.
    assert_eq!(
        sprite_index_at(&mut app, gap_lower),
        Some(*roles.slab),
        "the storey-0 cell beneath the upper gap must still emit (peek-through reveals it)",
    );
    // Real terrain on the upper storey still draws.
    assert_eq!(
        sprite_index_at(&mut app, wall_upper),
        Some(*roles.wall),
        "a REAL upper-storey terrain cell (a wall) must still emit its sprite",
    );
}

/// GTW-519 C3 (per-storey Z) — a tile on storey *k* sits at `z == cell_to_world(_, k).z`, and
/// the same `(x, y)` on storey *k+1* draws at a STRICTLY GREATER z (painter's occlusion).
///
/// At `ActiveLevel` 1: authors a wall at `(4,4)` on BOTH storey 0 and storey 1, then reads the
/// two tiles' `Transform.z`. Asserts each equals its storey's `cell_to_world` z and that the
/// upper tile's z is strictly greater — the occlusion falls out of the existing per-storey Z.
#[test]
fn per_storey_z_orders_upper_over_lower() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(4, 4);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let lower = CellLevel::new(cell, l0);
    let upper = CellLevel::new(cell, l1);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower, TerrainKind::Wall),
            TerrainPlacement::new(upper, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let lower_z = sprite_z_at(&mut app, lower);
    let upper_z = sprite_z_at(&mut app, upper);
    assert_eq!(
        lower_z,
        Some(cell_to_world(cell, l0).z),
        "the storey-0 tile must sit at cell_to_world(cell, L0).z",
    );
    assert_eq!(
        upper_z,
        Some(cell_to_world(cell, l1).z),
        "the storey-1 tile must sit at cell_to_world(cell, L1).z",
    );
    let (Some(lower_z), Some(upper_z)) = (lower_z, upper_z) else {
        return;
    };
    assert!(
        upper_z > lower_z,
        "the upper storey's tile z ({upper_z}) must be STRICTLY greater than the lower's \
         ({lower_z}) — painter's-algorithm occlusion from the per-storey Z",
    );
}

/// GTW-519 C4/C5 — after `present_fog` runs, a tile on a LOWER drawn storey has `brightness <
/// 1.0` (darkened) while a tile on the ACTIVE storey has `brightness == 1.0` (full-bright),
/// AND a lower-storey EXPLORED cell still has its fog `saturation == 0.0` preserved
/// (fog COMPOSES with the darken, is not replaced by it).
///
/// At `ActiveLevel` 1: authors a wall at `(4,4)` on storey 0 and on storey 1. The storey-1
/// wall is squad-VISIBLE; the storey-0 wall is EXPLORED-only. Drives the REAL `present_fog`
/// (requires `SquadVisibility` + `CombatTuning` — inserted here) and asserts the four facts.
#[test]
fn lower_storey_darkened_active_full_bright_and_explored_saturation_preserved() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(4, 4);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let lower = CellLevel::new(cell, l0);
    let active = CellLevel::new(cell, l1);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower, TerrainKind::Wall),
            TerrainPlacement::new(active, TerrainKind::Wall),
        ],
    );
    app.world_mut().insert_resource(CoverLedger::new());
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);
    // present_fog's gate needs CombatTuning present (the battle-configured witness).
    app.world_mut().insert_resource(CombatTuning::default());

    // The active-storey wall is VISIBLE; the lower-storey wall is EXPLORED-only (remembered).
    set_fog(&mut app, &[active], &[lower]);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    // One update draws; present_fog runs the same update (.after the draw) — settle a second
    // so the in-place material edit is observable (settle-before-read).
    app.update();
    app.update();

    // C4: the active storey is full-bright.
    assert_eq!(
        sprite_brightness_at(&mut app, active),
        Some(1.0),
        "the ACTIVE-storey tile must render at full brightness (1.0)",
    );
    // C4: a lower drawn storey is darkened (< 1.0).
    let lower_brightness = sprite_brightness_at(&mut app, lower);
    assert!(
        lower_brightness.is_some_and(|b| b < 1.0),
        "a LOWER drawn-storey tile must render darkened (brightness < 1.0); got \
         {lower_brightness:?}",
    );
    // C5: the lower-storey EXPLORED cell still desaturates (fog composes, not replaced) —
    // saturation 0.0 is the EXPLORED greyscale, driven ALONGSIDE the darken.
    assert_eq!(
        sprite_saturation_at(&mut app, lower),
        Some(0.0),
        "a lower-storey EXPLORED cell must KEEP its fog greyscale (saturation 0.0) — the \
         darken multiplies ON TOP of the saturation, never instead of it",
    );
    // And the VISIBLE active tile keeps full colour (saturation 1.0) — sanity that fog still
    // drives the active storey too.
    assert_eq!(
        sprite_saturation_at(&mut app, active),
        Some(1.0),
        "the VISIBLE active-storey cell must be full colour (saturation 1.0)",
    );
}

/// GTW-519 C6 — a `CoverDestroyed` on a DRAWN LOWER storey swaps that cover cell to rubble
/// (the drawn-band predicate), while a `CoverDestroyed` STRICTLY ABOVE the active view level
/// is ignored (that terrain is not drawn).
///
/// At `ActiveLevel` 1: authors cover at `(9,8)` on storey 0 (a lower drawn storey) and cover
/// at `(9,8)` on storey 2 (above active — not drawn). Smashes both. Asserts the storey-0 cover
/// swapped to rubble, and the storey-2 cell has no drawn sprite to swap (None) — the above-active
/// destruction is a no-op.
#[test]
fn cover_destroyed_swaps_on_lower_storey_and_ignores_above_active() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let l2 = Level::new(2);
    let lower_cover = CellLevel::new(Cell::new(9, 8), l0);
    let above_cover = CellLevel::new(Cell::new(9, 8), l2);

    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(lower_cover, TerrainKind::Cover),
            TerrainPlacement::new(above_cover, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(lower_cover, low_cover_entry());
    cover_ledger.insert(above_cover, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the lower cover is drawn (a real tile to swap); the above-active cover is
    // NOT drawn (culled), so there is no tile there to begin with.
    assert_eq!(
        sprite_index_at(&mut app, lower_cover),
        Some(*roles.cover),
        "the lower-storey cover must be drawn before the smash",
    );
    assert_eq!(
        sprite_index_at(&mut app, above_cover),
        None,
        "the above-active cover must NOT be drawn (culled)",
    );

    // Smash BOTH cover cells.
    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(lower_cover));
    app.world_mut()
        .resource_mut::<Messages<CoverDestroyed>>()
        .write(CoverDestroyed::new(above_cover));
    app.update();

    // C6: the DRAWN lower-storey cover swapped to rubble.
    assert_eq!(
        sprite_index_at(&mut app, lower_cover),
        Some(*roles.rubble),
        "a cover smashed on a DRAWN lower storey must swap to the rubble tile (C6)",
    );
    // The above-active destruction is ignored — still no drawn tile there.
    assert_eq!(
        sprite_index_at(&mut app, above_cover),
        None,
        "a cover smashed STRICTLY ABOVE the active view level must be ignored (not drawn)",
    );
}

// ─────────────────────────────────────────────────────────────────────────────────────────
// GTW-521 — the ViewMode full-view toggle: DownToActive culls above active, FullView draws
// the whole stack, and toggling back removes the upper storeys again.
// ─────────────────────────────────────────────────────────────────────────────────────────

/// GTW-521 C1/C2/C3 — with two storeys authored and `ActiveLevel = 0`:
/// [`ViewMode::DownToActive`] (the default) draws ONLY storey-0's slab (storey-1 culled),
/// flipping to [`ViewMode::FullView`] redraws the band and NOW draws storey-1's slab too, and
/// flipping BACK removes it again (the round-trip, C3).
///
/// Drives the REAL `draw_static_battlefield` system: the `ViewMode` change is its GTW-521
/// redraw trigger (added alongside the `ActiveLevel::is_changed` trigger), so the drawn set
/// changes with the toggle and no other input. The active level is held at 0 throughout, so
/// this isolates the `ViewMode` ceiling (C5 — the toggle does not move the active storey). The
/// slab on the upper storey is the peek-through-exempt REAL terrain fact, so it is the clean
/// storey-1 sprite to count.
#[test]
fn full_view_toggle_draws_upper_storeys_and_round_trips() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);

    // Empty occupancy: storey 0 draws its full floor field; storey 1 draws ONLY its authored
    // slab (peek-through, C2), which is the clean upper-storey sprite this test counts.
    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(CellLevel::new(slab_cell, l0), SlabState::Present);
    surface.set_slab(CellLevel::new(slab_cell, l1), SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    // ActiveLevel stays at 0 the WHOLE test; only the ViewMode changes. The renderer plugin
    // init_resource-s ViewMode(DownToActive) on build, so the default path runs first.
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    // DEFAULT (DownToActive) at active 0: storey 1 (strictly ABOVE active) is CULLED — ZERO
    // storey-1 sprites — exactly the GTW-519/520 behaviour (C1).
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the renderer plugin seeds the default ViewMode (DownToActive)",
    );
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        0,
        "in DownToActive at active 0, storey 1 (above active) must draw NOTHING (C1)",
    );

    // Flip to FullView: the ViewMode change re-runs the draw for the WHOLE stack, so storey
    // 1's slab NOW draws (C2) — despite the active level being unchanged at 0.
    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::FullView;
    app.update();
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        1,
        "in FullView, storey 1 must NOW draw its real terrain (the slab), regardless of the \
         active level (C2)",
    );
    // The lower storey's slab is still drawn (FullView never drops the band floor).
    assert_eq!(
        sprite_index_at(&mut app, CellLevel::new(slab_cell, l0)),
        tile_roles(&app).map(|r| *r.slab),
        "the storey-0 slab must STILL draw in FullView (the band floor is unchanged)",
    );

    // Flip BACK to DownToActive: the upper storey is culled again (the round-trip, C3).
    *app.world_mut().resource_mut::<ViewMode>() = ViewMode::DownToActive;
    app.update();
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l1),
        0,
        "toggling back to DownToActive at active 0 must REMOVE storey 1 again (C3 round-trip)",
    );
}

/// Reads the presenter [`ViewMode`], or [`ViewMode::DownToActive`] if somehow absent (the
/// renderer plugin always inserts it — this keeps the read panic-free).
fn view_mode(app: &App) -> ViewMode {
    app.world()
        .get_resource::<ViewMode>()
        .copied()
        .unwrap_or(ViewMode::DownToActive)
}

/// Spawns ONE sim-side weapon-EMPLACEMENT terrain entity at `key` (GTW-543) carrying its
/// per-def [`TerrainGraphicKey`] (`"emplacement"`) AND the [`EmplacementState`] the enter/exit
/// toggle flips — mirroring exactly what the sim's `setup_battle` spawns for an `Emplacement`
/// terrain def (a terrain entity with the graphic key + `EmplacementState::Vacant`). Returns the
/// spawned [`Entity`] so the test can flip its state to drive the `Changed<EmplacementState>`
/// swap.
fn spawn_emplacement_entity(app: &mut App, key: CellLevel) -> bevy::ecs::entity::Entity {
    app.world_mut()
        .spawn((
            TerrainCell::new(key),
            TerrainGraphicKey::new("emplacement".to_owned()),
            EmplacementState::Vacant,
        ))
        .id()
}

/// GTW-543 (PRESENTER draw, POSITIVE) — a weapon-emplacement terrain entity (a
/// `TerrainGraphicKey` of `"emplacement"`) draws its DEDICATED emplacement tile, distinct from
/// the generic `cover` tile, through the REAL `draw_static_battlefield` system.
///
/// Both cells are the SAME `TerrainKind::Emplacement` in the occupancy grid; the sim-spawned
/// per-def `TerrainGraphicKey` is what picks the tile. The emplacement cell resolves to
/// `roles.emplacement` (NOT `roles.cover`), so an emplacement reads distinctly from a chest-high
/// crate — the ticket's "a distinct glyph/color/tile". Occlusion-aware: it reads the resolved
/// material `atlas_index` actually carried by the spawned sprite (`sprite_index_at`) after
/// settling the one-shot draw.
#[test]
fn emplacement_draws_its_own_distinct_tile() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let emp_cell = CellLevel::new(Cell::new(8, 7), l0);
    let cover_cell = CellLevel::new(Cell::new(9, 8), l0);

    // Author the emplacement cell as TerrainKind::Emplacement + a plain cover cell alongside, so
    // the test proves the emplacement draws its OWN tile rather than the shared cover default.
    insert_occupancy(
        &mut app,
        vec![
            TerrainPlacement::new(emp_cell, TerrainKind::Emplacement),
            TerrainPlacement::new(cover_cell, TerrainKind::Cover),
        ],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(emp_cell, low_cover_entry());
    cover_ledger.insert(cover_cell, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    // The per-def facts the sim would spawn: the emplacement graphic + a plain cover graphic.
    spawn_emplacement_entity(&mut app, emp_cell);
    spawn_terrain_entity(&mut app, cover_cell, "cover", None);

    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the emplacement tile is DISTINCT from the cover tile (else the pin is vacuous).
    assert_ne!(
        *roles.emplacement, *roles.cover,
        "the `emplacement` and `cover` role indices must differ (else the distinct-tile pin is \
         vacuous)",
    );

    // POSITIVE: the emplacement cell resolves to its OWN dedicated tile, not the cover default.
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement),
        "the emplacement cell must draw the dedicated `emplacement` tile, NOT the generic cover \
         tile",
    );
    // The plain cover cell still draws the cover tile (the emplacement graphic did not leak).
    assert_eq!(
        sprite_index_at(&mut app, cover_cell),
        Some(*roles.cover),
        "the plain cover cell must still draw the `cover` tile",
    );
}

/// GTW-543 (PRESENTER occupied-indicator, POSITIVE) — a `Changed<EmplacementState>` from
/// Vacant→Occupied swaps the emplacement cell's tile IN PLACE to the `emplacement_occupied` tile
/// (the same `Entity`), and vacating swaps it back — through the REAL
/// `indicate_emplacement_occupied` system.
///
/// Spawns one emplacement terrain entity (drawn `Vacant` on the `emplacement` tile), flips its
/// `EmplacementState` to `Occupied` and settles (the settle-before-read rule), asserts the tile
/// re-indexes to `emplacement_occupied` on the SAME entity (mutate-not-respawn), then flips it
/// back to `Vacant` and asserts it restores to `emplacement`. Mirrors the destruction-swap
/// tests' shape but driven by a component state change rather than a destruction message.
#[test]
fn occupying_an_emplacement_swaps_its_tile_in_place() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let emp_cell = CellLevel::new(Cell::new(4, 5), l0);

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(emp_cell, TerrainKind::Emplacement)],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(emp_cell, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);

    let emplacement = spawn_emplacement_entity(&mut app, emp_cell);

    // Fire the one-shot draw so the real plugin spawns the emplacement tile (Vacant).
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let roles = tile_roles(&app);
    assert!(roles.is_some(), "TileRoles must be resident after settle");
    let Some(roles) = roles else { return };

    // Precondition: the occupied tile is a REAL visible swap (distinct atlas index).
    assert_ne!(
        *roles.emplacement_occupied, *roles.emplacement,
        "the `emplacement_occupied` tile index must differ from the vacant `emplacement` index \
         (a real swap)",
    );

    // The emplacement starts on the vacant tile; capture its Entity id for the same-entity check.
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement),
        "an unmanned emplacement's sprite must start on the vacant `emplacement` tile",
    );
    let entity_before = sprite_entity_at(&mut app, emp_cell);
    assert!(
        entity_before.is_some(),
        "the emplacement cell's terrain sprite must exist before it is manned",
    );

    // Man the emplacement — flip the sim state Vacant→Occupied (the enter act's effect), then
    // settle so `indicate_emplacement_occupied` runs on the Changed<EmplacementState>.
    let occupied = app.world_mut().get_mut::<EmplacementState>(emplacement);
    assert!(
        occupied.is_some(),
        "the spawned emplacement entity must carry EmplacementState",
    );
    let Some(mut state) = occupied else { return };
    *state = EmplacementState::Occupied;
    app.update();

    // POSITIVE: the manned emplacement now renders the OCCUPIED tile (the intended content
    // actually renders — not merely "something changed").
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement_occupied),
        "a manned emplacement's sprite must swap to the `emplacement_occupied` tile",
    );
    // The SAME entity persists (in-place mutation, no despawn/respawn).
    assert_eq!(
        sprite_entity_at(&mut app, emp_cell),
        entity_before,
        "the manned emplacement cell must be the SAME Entity after the swap (no despawn/respawn)",
    );

    // Vacate — flip Occupied→Vacant (the exit act's effect), then settle: the tile restores.
    let vacate = app.world_mut().get_mut::<EmplacementState>(emplacement);
    assert!(
        vacate.is_some(),
        "the emplacement entity must still carry EmplacementState",
    );
    let Some(mut state) = vacate else { return };
    *state = EmplacementState::Vacant;
    app.update();
    assert_eq!(
        sprite_index_at(&mut app, emp_cell),
        Some(*roles.emplacement),
        "a vacated emplacement's sprite must restore to the vacant `emplacement` tile",
    );
}
