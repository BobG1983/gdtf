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
    asset::AssetPlugin,
    ecs::message::Messages,
    math::Vec2,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    sprite::Sprite,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    ActiveLevel, CELL_PX, TerrainSprite, TileRoles, TopDownRendererPlugin, cell_to_world,
};
use gdtf_battle_sim::{
    ArmorHardness, ArmorProtection, BattleInProgress, BattleReady, Cell, CellLevel, CoverDestroyed,
    CoverEntry, CoverHp, CoverLedger, HeightBand, Level, OccupancyGrid, OccupancyInput, SlabState,
    SurfaceGrid, TerrainKind, TerrainPlacement,
};

/// Generous settle headroom so a slow CI box never flakes on the async atlas /
/// tile-role loads.
const MAX_UPDATES: u32 = 128;

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
    app
}

/// Drives `update()`s until `TileRoles` + `TopDownAtlases` are resident (the async
/// load chain has settled). Returns whether they settled within `MAX_UPDATES`.
fn settle_resources(app: &mut App) -> bool {
    for _ in 0..MAX_UPDATES {
        app.update();
        let has_roles = app.world().get_resource::<TileRoles>().is_some();
        let has_atlases = app
            .world()
            .get_resource::<gdtf_battle_presenter::TopDownAtlases>()
            .is_some();
        if has_roles && has_atlases {
            return true;
        }
    }
    false
}

/// Authors an `OccupancyGrid` with the given terrain placements (built through the real
/// `OccupancyInput` → `build_from_occupancy_input` path) and inserts it.
fn insert_occupancy(app: &mut App, terrain: Vec<TerrainPlacement>) {
    let input = OccupancyInput {
        terrain,
        occupants: Vec::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(&input);
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

/// Reads the atlas index of the one `TerrainSprite` at `key`, if present.
fn sprite_index_at(app: &mut App, key: CellLevel) -> Option<usize> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Sprite)>();
    for (terrain, sprite) in q.iter(app.world()) {
        if terrain.at == key {
            return sprite.texture_atlas.as_ref().map(|a| a.index);
        }
    }
    None
}

/// Reads the (`custom_size`, translation) of the one `TerrainSprite` at `key`.
fn sprite_geometry_at(app: &mut App, key: CellLevel) -> Option<(Option<Vec2>, bevy::math::Vec3)> {
    let mut q = app
        .world_mut()
        .query::<(&TerrainSprite, &Sprite, &Transform)>();
    for (terrain, sprite, transform) in q.iter(app.world()) {
        if terrain.at == key {
            return Some((sprite.custom_size, transform.translation));
        }
    }
    None
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
    assert!(
        settle_resources(&mut app),
        "TileRoles + TopDownAtlases must resolve within {MAX_UPDATES} updates",
    );

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
    assert!(settle_resources(&mut app), "resources must resolve");

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
    assert!(settle_resources(&mut app), "resources must resolve");

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
    app.world_mut().resource_mut::<ActiveLevel>().0 = l1;
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
