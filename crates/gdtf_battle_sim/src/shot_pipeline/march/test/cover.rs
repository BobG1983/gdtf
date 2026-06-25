//! AC #3: cover stops a non-strictly-higher round; destroyed cover passes.

use super::support::*;

/// Cover in the path stops a round not flying strictly higher (a LOW round vs a
/// MID cover impacts — `Cover` result carrying the `CoverEntry`).
#[test]
fn cover_stops_non_strictly_higher_round() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let grid = OccupancyGrid::new();

    let at = key(5, 2, 0);
    let entry = cover_entry(HeightBand::Mid);
    let mut cover = CoverLedger::new();
    cover.insert(at, entry);

    // A LOW round flat East — LOW is not strictly higher than MID → impacts.
    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);

    let result = march_vector(
        muzzle,
        dir,
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

/// A DESTROYED cover cell does NOT stop the round — destroyed cover is excluded
/// via `is_cover_destroyed`, so the round passes through.
#[test]
fn destroyed_cover_passes_through() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();

    let at = key(5, 2, 0);
    let entry = cover_entry(HeightBand::High);
    let mut cover = CoverLedger::new();
    cover.insert(at, entry);

    // The occupancy grid marks the cover cell destroyed.
    let mut grid = OccupancyGrid::new();
    grid.mark_cover_destroyed(at);

    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);

    let result = march_vector(
        muzzle,
        dir,
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

/// A cover entry whose own `destroyed` flag is set also passes through — the
/// march checks the ledger entry's flag as well as the grid's exclusion set.
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
        dir,
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
