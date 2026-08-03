use super::support::*;

#[test]
fn shooter_own_cell_never_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let entity = spawn_entity();

    let shooter_cell = key(2, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(shooter_cell, Some(entity));
    grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
    let mut cover = CoverLedger::new();
    cover.insert(shooter_cell, cover_entry(HeightBand::High));

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let result = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
        &tuning,
        shooter_cell,
        no_dead(),
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "the round must leave the shooter's own cell (not blocked by its own occupant/cover)",
    );
    assert_ne!(
        result.at, shooter_cell,
        "the round must not have stopped in the shooter's own cell",
    );
}

#[test]
fn friendly_fire_is_real() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let ally = spawn_entity();

    let ally_cell = key(4, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(ally_cell, Some(ally));
    grid.set_occupant_band(ally_cell, Some(HeightBand::Mid));
    let cover = CoverLedger::new();

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
        MarchKind::Ganger(ally),
        "an allied ganger at equal-or-lower band is struck — friendly fire is real",
    );
    assert_eq!(result.at, ally_cell);
}
