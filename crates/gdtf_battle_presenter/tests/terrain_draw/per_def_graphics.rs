use bevy::ecs::message::Messages;
use gdtf_battle_sim::{
    battle::BattleReady,
    cover::CoverLedger,
    occupancy::{TerrainKind, TerrainPlacement},
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
    terrain::facing::TerrainFacing,
    test_support::test_pieces,
};

use super::harness::*;

#[test]
fn per_def_graphic_distinguishes_same_kind_cells() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let cover_a = CellLevel::new(Cell::new(8, 7), l0);
    let cover_b = CellLevel::new(Cell::new(9, 8), l0);

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

    spawn_terrain_entity(
        &mut app,
        cover_a,
        test_pieces::COVER,
        TerrainFacing::North,
        None,
    );
    spawn_terrain_entity(
        &mut app,
        cover_b,
        test_pieces::RUBBLE_COVER,
        TerrainFacing::North,
        None,
    );

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

    let cover_rect = def_rect(&defs, "cover");
    let rubble_rect = def_rect(&defs, "rubble");
    assert!(
        cover_rect.is_some() && rubble_rect.is_some(),
        "the `cover` + `rubble` seeded defs must resolve with Sheet rects",
    );
    assert_ne!(
        cover_rect, rubble_rect,
        "the `cover` and `rubble` def rects must differ (else the pin is vacuous)",
    );

    let rect_a = sprite_rect_at(&mut app, cover_a);
    let rect_b = sprite_rect_at(&mut app, cover_b);

    assert_eq!(
        rect_a, cover_rect,
        "cell A holds the Cover def whose views name `cover`, so it draws the `cover` def's \
         sheet rect",
    );
    assert_eq!(
        rect_b, rubble_rect,
        "cell B holds the Cover def whose views name `rubble`, so it draws the `rubble` def's \
         sheet rect, NOT the shared TerrainKind::Cover default",
    );
    assert_ne!(
        rect_a, rect_b,
        "two cells of the SAME TerrainKind holding DIFFERENT defs must draw DIFFERENT sprites \
         (kind-keyed-only resolution would make them identical and fail here)",
    );
}

#[test]
fn ns_and_ew_wall_resolve_to_distinct_sprites() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let wall_ns = CellLevel::new(Cell::new(8, 7), l0);
    let wall_ew = CellLevel::new(Cell::new(9, 8), l0);

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

    spawn_terrain_entity(
        &mut app,
        wall_ns,
        test_pieces::FACING_WALL,
        TerrainFacing::North,
        None,
    );
    spawn_terrain_entity(
        &mut app,
        wall_ew,
        test_pieces::FACING_WALL,
        TerrainFacing::East,
        None,
    );

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

    let wall_rect = def_rect(&defs, "wall");
    let wall_ew_rect = def_rect(&defs, "wall_ew");
    assert!(
        wall_rect.is_some() && wall_ew_rect.is_some(),
        "the `wall` + `wall_ew` seeded defs must resolve with Sheet rects",
    );
    assert_ne!(
        wall_rect, wall_ew_rect,
        "the `wall` (NS) and `wall_ew` (EW) def rects must differ (else the pin is vacuous)",
    );

    let rect_ns = sprite_rect_at(&mut app, wall_ns);
    let rect_ew = sprite_rect_at(&mut app, wall_ew);

    assert_eq!(
        rect_ns, wall_rect,
        "the north-facing wall cell must draw its def's `Edge(North)` view, the `wall` def's \
         sheet rect",
    );
    assert_eq!(
        rect_ew, wall_ew_rect,
        "the east-facing wall cell must draw its def's `Edge(East)` view, the `wall_ew` def's \
         sheet rect, NOT the shared TerrainKind::Wall default",
    );
    assert_ne!(
        rect_ns, rect_ew,
        "one wall def turned two ways must draw DIFFERENT sprites (dropping the facing, or \
         resolving by kind alone, would make them identical)",
    );
}

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

    spawn_terrain_entity(
        &mut app,
        slab_with,
        test_pieces::SLAB,
        TerrainFacing::North,
        Some("step_metal"),
    );
    spawn_terrain_entity(
        &mut app,
        slab_without,
        test_pieces::SLAB,
        TerrainFacing::North,
        None,
    );

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
    let slab_rect = def_rect(&defs, "slab");
    assert!(
        slab_rect.is_some(),
        "the `slab` seeded def must resolve with a Sheet rect"
    );

    assert_eq!(
        sprite_rect_at(&mut app, slab_with),
        slab_rect,
        "the slab cell WITH a footfall must render the `slab` graphic",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_without),
        slab_rect,
        "the slab cell WITHOUT a footfall must STILL render the `slab` graphic (absent \
         footfall is the documented silent default, not a missing tile)",
    );
}
