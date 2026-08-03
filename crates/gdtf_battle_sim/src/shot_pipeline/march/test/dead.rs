use super::support::*;

#[test]
fn first_occupant_dead_round_strikes_second_live_occupant() {
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
    let is_dead = move |e: Entity| e == first;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert_eq!(
        result.kind,
        MarchKind::Ganger(second),
        "the round passes through the dead first ganger and strikes the live second",
    );
    assert_eq!(
        result.at, second_cell,
        "the impact cell is the live second ganger's cell, not the corpse's",
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
        &grid,
        &surface,
        &cover,
        &tuning,
        far_shooter(),
        no_dead(),
    );
    assert_eq!(
        result.kind,
        MarchKind::Ganger(first),
        "with nothing dead, the first occupant still stops the round (today's behavior)",
    );
    assert_eq!(
        result.at, first_cell,
        "the impact cell is the first ganger's cell"
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
    grid.set_occupant(corpse_cell, Some(corpse));
    grid.set_occupant_band(corpse_cell, Some(HeightBand::Low));
    let mut cover = CoverLedger::new();
    cover.insert(wall_cell, cover_entry(HeightBand::High)); 

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert!(
        matches!(result.kind, MarchKind::Cover(_)),
        "passing through the corpse, the round is stopped by the wall behind it",
    );
    assert_eq!(
        result.at, wall_cell,
        "the impact cell is the wall cell behind the corpse"
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
    grid.set_occupant(corpse_cell, Some(corpse));
    grid.set_occupant_band(corpse_cell, Some(HeightBand::Low));

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "with nothing behind the corpse, the round flies off the grid → Miss",
    );
    assert!(
        !matches!(result.kind, MarchKind::Ganger(_)),
        "the round must not strike the corpse",
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
    grid.set_occupant(corpse_cell, Some(corpse));
    grid.set_occupant_band(corpse_cell, Some(HeightBand::Low));

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let is_dead = move |e: Entity| e == corpse;

    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
        &tuning,
        far_shooter(),
        is_dead,
    );
    assert!(
        !matches!(result.kind, MarchKind::Ganger(_)),
        "a corpse the predicate marks dead is never struck",
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "the lone corpse is transparent → the round misses off the grid",
    );
}
