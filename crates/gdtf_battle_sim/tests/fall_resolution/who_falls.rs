use gdtf_battle_sim::{
    entity::TerrainPieceKind,
    prelude::{CellLevel, Level, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    tuning::PerStoreyDamage,
};

use super::harness::*;

#[test]
fn faller_on_destroyed_slab_level_falls_roof_occupant_does_not() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let faller = spawn_faller(app.world_mut(), 2);
    let roof_decoy = spawn_faller(app.world_mut(), 3);
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 2, TerrainPieceKind::Slab);

    assert_eq!(
        level_of(&app, faller),
        0,
        "the level-2 faller drops to the ground"
    );
    assert_eq!(
        level_of(&app, roof_decoy),
        3,
        "the roof occupant (level+1) is the WRONG actor and must NOT fall"
    );
    let signals = fall_signals(&app);
    assert_eq!(
        signals.len(),
        1,
        "exactly one fall (the roof decoy did not fall)"
    );
    let signal = signals[0];
    assert_eq!(signal.ganger, faller);
    assert_eq!(signal.from_level, Level::new(2));
    assert_eq!(signal.to_level, Level::new(0));
    assert_eq!(*signal.storeys, 2, "start 2 → land 0 is a 2-storey fall");
}

#[test]
fn a_destroyed_cover_on_the_faller_s_cell_drops_nobody() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let faller = spawn_faller(app.world_mut(), 2);
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 2, TerrainPieceKind::Cover);

    assert_eq!(
        level_of(&app, faller),
        2,
        "a destroyed COVER is not a floor going away, so the ganger standing on that cell stays \
         at level 2",
    );
    assert!(
        fall_signals(&app).is_empty(),
        "a destroyed cover fires no FallOccurred",
    );
}

#[test]
fn multi_storey_drop_through_absent_lands_on_first_present() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(6));
    let faller = spawn_faller(app.world_mut(), 4);
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 4, TerrainPieceKind::Slab);

    assert_eq!(
        level_of(&app, faller),
        1,
        "the faller falls through the Absent intermediate and lands on the Present level-1 slab"
    );
    let signals = fall_signals(&app);
    assert_eq!(signals.len(), 1);
    assert_eq!(
        *signals[0].storeys, 3,
        "start 4 → land 1 is a 3-storey fall"
    );
}

#[test]
fn ganger_on_different_level_does_not_fall() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let elsewhere = spawn_faller(app.world_mut(), 1);
    let mut surface = SurfaceGrid::new();
    surface.set_slab(
        CellLevel::new(column_cell(), Level::new(1)),
        SlabState::Present,
    );
    app.insert_resource(surface);
    app.insert_resource(OccupancyGrid::new());

    destroy_slab_and_settle(&mut app, 3, TerrainPieceKind::Slab);

    assert_eq!(
        level_of(&app, elsewhere),
        1,
        "a ganger on a different level does not move"
    );
    assert!(
        fall_signals(&app).is_empty(),
        "no fall fires for a ganger off the destroyed level"
    );
}

#[test]
fn stair_lower_endpoint_occupant_is_braced() {
    let mut app = falls_app(SEED, PerStoreyDamage::new(10));
    let braced = spawn_faller(app.world_mut(), 2);
    app.insert_resource(SurfaceGrid::new());
    let mut occupancy = OccupancyGrid::new();
    occupancy.mark_stair_cell(CellLevel::new(column_cell(), Level::new(2)));
    app.insert_resource(occupancy);
    let hp_before = hp_of(&app, braced);

    destroy_slab_and_settle(&mut app, 2, TerrainPieceKind::Slab);

    assert_eq!(
        level_of(&app, braced),
        2,
        "a braced stair occupant does not fall"
    );
    assert_eq!(
        hp_of(&app, braced),
        hp_before,
        "a braced occupant takes no fall damage"
    );
    assert!(
        fall_signals(&app).is_empty(),
        "a braced occupant fires no FallOccurred"
    );
}
