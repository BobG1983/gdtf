use bevy::{app::App, ecs::message::Messages, transform::components::Transform};
use gdtf_battle_presenter::{ActiveLevel, TerrainSprite, cell_to_world};
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

#[test]
fn raising_active_level_redraws_the_whole_drawn_band() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let slab_cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let slab0 = CellLevel::new(slab_cell, l0);
    let slab1 = CellLevel::new(slab_cell, l1);

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

    let defs = sprite_defs(&app);
    assert!(defs.is_some(), "the SpriteDefRegistry must be resident");
    let Some(defs) = defs else { return };

    assert_eq!(
        sprite_rect_at(&mut app, slab0),
        def_rect(&defs, "slab"),
        "the level-0 slab sprite must be present at active level 0",
    );
    assert_eq!(
        sprite_entity_at(&mut app, slab1),
        None,
        "the level-1 slab sprite (strictly ABOVE active) must be CULLED at active level 0",
    );
    assert!(
        all_sprites_within_band(&mut app, l0),
        "every terrain sprite must be within the drawn band [0..=0] before the change",
    );

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.update();

    assert_eq!(
        sprite_rect_at(&mut app, slab1),
        def_rect(&defs, "slab"),
        "after raising to level 1, the level-1 (active) slab sprite must be present",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab0),
        def_rect(&defs, "slab"),
        "after raising to level 1, the level-0 slab sprite must STILL be present (a drawn \
         lower storey, not despawned)",
    );
    assert!(
        all_sprites_within_band(&mut app, l1),
        "after raising to level 1, every terrain sprite must be within the drawn band [0..=1]",
    );
}

fn all_sprites_within_band(app: &mut App, active: Level) -> bool {
    let ceiling = i32::from(*active);
    let mut q = app.world_mut().query::<&TerrainSprite>();
    q.iter(app.world()).all(|t| t.at.z <= ceiling)
}

fn sprite_z_at(app: &mut App, key: CellLevel) -> Option<f32> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(t, _)| t.at == key)
        .map(|(_, transform)| transform.translation.z)
}

#[test]
fn multi_level_draws_the_band_below_and_at_active_and_none_above() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cell = Cell::new(2, 2);
    let l0 = Level::new(0);
    let l1 = Level::new(1);
    let l2 = Level::new(2);

    insert_occupancy(&mut app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(CellLevel::new(cell, l0), SlabState::Present);
    surface.set_slab(CellLevel::new(cell, l1), SlabState::Present);
    surface.set_slab(CellLevel::new(cell, l2), SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);

    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(l1);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let ground_count = terrain_sprite_count_on_level(&mut app, l0);
    assert!(
        ground_count > 3,
        "storey 0 (the ground floor, below active) must draw its full floor field in the \
         multi-level band; got {ground_count}",
    );
    let active_count = terrain_sprite_count_on_level(&mut app, l1);
    assert_eq!(
        active_count, 1,
        "storey 1 (active, an upper storey) must draw ONLY its real terrain (the slab), not a \
         floor field — peek-through applies to every non-ground storey; got {active_count}",
    );
    assert_eq!(
        terrain_sprite_count_on_level(&mut app, l2),
        0,
        "storey 2 (strictly above active) must draw NOTHING (the hard cull)",
    );
}

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

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    assert_eq!(
        sprite_entity_at(&mut app, gap_upper),
        None,
        "an open/empty upper-storey cell must emit NO sprite (peek-through)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, gap_lower),
        def_rect(&defs, "slab"),
        "the storey-0 cell beneath the upper gap must still emit (peek-through reveals it)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, wall_upper),
        def_rect(&defs, "wall"),
        "a REAL upper-storey terrain cell (a wall) must still emit its sprite",
    );
}

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
