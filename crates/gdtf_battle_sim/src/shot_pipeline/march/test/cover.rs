use super::support::*;

#[test]
fn cover_stops_non_strictly_higher_round() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let grid = OccupancyGrid::new();

    let at = key(5, 2, 0);
    let entry = cover_entry(HeightBand::Mid);
    let mut cover = CoverLedger::new();
    cover.insert(at, entry);

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
        MarchKind::Cover(entry),
        "a LOW round must impact MID cover, carrying the CoverEntry",
    );
    assert_eq!(result.at, at);
}

#[test]
fn destroyed_cover_passes_through() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();

    let at = key(5, 2, 0);
    let entry = cover_entry(HeightBand::High);
    let mut cover = CoverLedger::new();
    cover.insert(at, entry);

    let mut grid = OccupancyGrid::new();
    grid.mark_cover_destroyed(at);

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
    assert!(
        !matches!(result.kind, MarchKind::Cover(_)),
        "destroyed cover must NOT stop the round",
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "with the cover destroyed and nothing else, the round misses off-grid",
    );
}

#[test]
fn ledger_destroyed_flag_passes_through() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let grid = OccupancyGrid::new();

    let at = key(5, 2, 0);
    let mut entry = cover_entry(HeightBand::High);
    entry.destroyed = crate::cover::Destroyed::new(true);
    let mut cover = CoverLedger::new();
    cover.insert(at, entry);

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
        MarchKind::Miss,
        "a ledger entry flagged destroyed must not stop the round",
    );
}
