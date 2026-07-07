//! One-shot `BattleReady` static draw: role-correct, `CELL_PX`-sized, cell-positioned
//! sprites (AC2).

use bevy::{
    app::App, asset::Assets, ecs::message::Messages, math::Vec2, prelude::MeshMaterial2d,
    transform::components::Transform,
};
use gdtf_battle_presenter::{CELL_PX, TerrainFogMaterial, TerrainSprite, cell_to_world};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

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
/// For each, asserts (a) the sprite's material rect equals the role key's SEEDED sprite
/// def's rect READ FROM THE REGISTRY (structural, never a literal — GTW-665), (b)
/// `custom_size == Some(Vec2::splat(CELL_PX))`, and (c) `Transform.translation ==
/// cell_to_world(cell, L0)` (the seeded CENTER anchors add a zero offset — the
/// identical-pixels claim).
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

    // Read the resolved def rects FROM the registry — never a hardcoded literal.
    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    // (a) role-correct sheet rects, structural against the seeded defs.
    assert_eq!(
        sprite_rect_at(&mut app, wall_key),
        def_rect(&defs, "wall"),
        "the wall cell's material rect must equal the `wall` def's seeded rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "cover"),
        "the cover cell's material rect must equal the `cover` def's seeded rect",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab cell's material rect must equal the `slab` def's seeded rect",
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
