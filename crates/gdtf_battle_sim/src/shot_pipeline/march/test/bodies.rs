use super::support::*;
use crate::march::MarchGrids;

/// Lay a body on the floor of `cell`, the way `sync_inactive_gangers` records a corpse.
fn lay_body(grid: &mut OccupancyGrid, cell: CellLevel, body: Entity) {
    grid.set_body(cell, Some(BodyOcclusion::new(body, HeightBand::Low)));
}

#[test]
fn body_on_the_floor_stops_a_floor_level_round() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let (corpse, live) = spawn_two_entities();

    let corpse_cell = key(5, 2, 0);
    let live_cell = key(7, 2, 0);
    let mut grid = OccupancyGrid::new();
    lay_body(&mut grid, corpse_cell, corpse);
    grid.set_occupant(live_cell, Some(live));
    grid.set_occupant_band(live_cell, Some(HeightBand::High));

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert_eq!(
        result.kind,
        MarchKind::Ganger(corpse),
        "a round at the floor stops on the body lying there; got {result:?}",
    );
    assert_eq!(
        result.at, corpse_cell,
        "the impact cell is the body's own cell; got {result:?}",
    );
}

#[test]
fn high_round_crosses_the_body_and_strikes_the_live_ganger_behind() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let (corpse, live) = spawn_two_entities();

    let corpse_cell = key(5, 2, 0);
    let live_cell = key(7, 2, 0);
    let mut grid = OccupancyGrid::new();
    lay_body(&mut grid, corpse_cell, corpse);
    grid.set_occupant(live_cell, Some(live));
    grid.set_occupant_band(live_cell, Some(HeightBand::High));

    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert_eq!(
        result.kind,
        MarchKind::Ganger(live),
        "a round above the floor crosses the body and strikes the live ganger behind it; got \
         {result:?}",
    );
    assert_eq!(
        result.at, live_cell,
        "the impact cell is the live ganger's cell, not the body's; got {result:?}",
    );
}

#[test]
fn no_occupant_dead_round_strikes_first_as_before() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let (first, second) = spawn_two_entities();

    let first_cell = key(5, 2, 0);
    let second_cell = key(7, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(first_cell, Some(first));
    grid.set_occupant_band(first_cell, Some(HeightBand::Low));
    grid.set_occupant(second_cell, Some(second));
    grid.set_occupant_band(second_cell, Some(HeightBand::Low));

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        no_dead(),
    );
    assert_eq!(
        result.kind,
        MarchKind::Ganger(first),
        "with nothing dead, the first occupant still stops the round (today's behavior); got \
         {result:?}",
    );
    assert_eq!(
        result.at, first_cell,
        "the impact cell is the first ganger's cell; got {result:?}",
    );
}

#[test]
fn first_occupant_dead_wall_behind_returns_wall() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let corpse = spawn_entity();

    let corpse_cell = key(5, 2, 0);
    let wall_cell = key(7, 2, 0);
    let mut grid = OccupancyGrid::new();
    lay_body(&mut grid, corpse_cell, corpse);
    let mut cover = CoverLedger::new();
    cover.insert(wall_cell, cover_entry(HeightBand::High));

    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert!(
        matches!(result.kind, MarchKind::Cover(_)),
        "crossing the body above the floor, the round is stopped by the wall behind it; got \
         {result:?}",
    );
    assert_eq!(
        result.at, wall_cell,
        "the impact cell is the wall cell behind the body; got {result:?}",
    );
}

#[test]
fn first_occupant_dead_nothing_behind_misses() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let corpse = spawn_entity();

    let corpse_cell = key(5, 2, 0);
    let mut grid = OccupancyGrid::new();
    lay_body(&mut grid, corpse_cell, corpse);

    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "with nothing behind the body, a round above the floor flies off the grid → Miss; got \
         {result:?}",
    );
}

#[test]
fn single_dead_occupant_is_not_struck() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let corpse = spawn_entity();

    let corpse_cell = key(5, 2, 0);
    let mut grid = OccupancyGrid::new();
    lay_body(&mut grid, corpse_cell, corpse);

    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "a body at the floor does not stop a round above the floor; got {result:?}",
    );
}
