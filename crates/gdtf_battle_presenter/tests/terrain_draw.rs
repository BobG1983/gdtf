//! GTW-218 (GTW-48 S4): headless draw-LOGIC tests for the static-battlefield terrain
//! draw — AC2 (one-shot draw of role-correct, cell-positioned, CELL_PX-sized sprites),
//! AC3 (a `CoverDestroyed` swaps the cover cell to the rubble tile), AC4 (an
//! `ActiveLevel` change redraws the new level and despawns off-level terrain).
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
    prelude::{MeshMaterial2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, CELL_PX, TerrainFogMaterial, TerrainSprite, TileRoles, TopDownAtlases,
    TopDownRendererPlugin, cell_to_world,
};
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, BattleInProgress, BattleReady, Cell, CellLevel, CoverDestroyed,
    CoverEntry, CoverHp, CoverLedger, FootfallSound, HeightBand, Level, OccupancyGrid,
    OccupancyInput, SlabDestroyed, SlabState, SurfaceGrid, TerrainCell, TerrainGraphicKey,
    TerrainKind, TerrainPlacement,
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

/// AC4 — an `ActiveLevel` change redraws for the new level; off-active-level terrain is
/// gone.
///
/// Authors a `Present` slab on level 0 AND one on level 1 (mirroring skirmish's
/// `(2,2,0)` + `(2,2,1)`). At `ActiveLevel` 0 only the level-0 slab sprite draws; after
/// setting `ActiveLevel(Level::new(1))` and updating, only the level-1 slab sprite
/// exists (the level-0 terrain was despawned).
#[test]
fn active_level_change_redraws_only_the_new_level() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let slab0 = CellLevel::new(slab_cell, l0);
    let slab1 = CellLevel::new(slab_cell, l1);

    // No walls/cover: an empty occupancy grid means every in-range cell is floor, so the
    // slab cell is the only DISTINCT role on each level — but the floor field is drawn on
    // BOTH levels, so the level-scoping is proven by the slab sprite's PRESENCE per level
    // and by every drawn sprite carrying the active level's z.
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

    // At active level 0: the level-0 slab sprite draws, the level-1 one does not.
    assert_eq!(
        sprite_index_at(&mut app, slab0),
        Some(*roles.slab),
        "the level-0 slab sprite must be present at active level 0",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab1),
        None,
        "the level-1 slab sprite must NOT be present at active level 0",
    );
    // Every drawn sprite is on level 0 (z == 0): no off-level terrain.
    assert!(
        all_sprites_on_level(&mut app, l0),
        "every terrain sprite must be on the active level (0) before the change",
    );

    // Change the active level to 1.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.update();

    assert_eq!(
        sprite_index_at(&mut app, slab1),
        Some(*roles.slab),
        "after the level change, the level-1 slab sprite must be present",
    );
    assert_eq!(
        sprite_index_at(&mut app, slab0),
        None,
        "after the level change, the level-0 slab sprite must be gone (despawned)",
    );
    assert!(
        all_sprites_on_level(&mut app, l1),
        "after the level change, every terrain sprite must be on the new level (1)",
    );
}

/// Whether every `TerrainSprite` in the world carries `level` as its `at.z`.
fn all_sprites_on_level(app: &mut App, level: Level) -> bool {
    let z = i32::from(*level);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).all(|t| t.at.z == z)
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
