use super::support::*;
use crate::march::MarchGrids;

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
        MarchKind::Cover(entry),
        "a LOW round must impact MID cover, carrying the CoverEntry",
    );
    assert_eq!(result.at, at);
}

#[test]
fn a_successor_cover_entry_stops_the_round_the_destroyed_one_let_through() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let grid = OccupancyGrid::new();

    let at = key(5, 2, 0);
    let mut smashed = cover_entry(HeightBand::High);
    smashed.destroyed = crate::cover::Destroyed::new(true);
    let mut cover = CoverLedger::new();
    cover.insert(at, smashed);

    // `replace_destroyed_piece` seeds the successor's own entry under the same cell.
    let successor = cover_entry(HeightBand::High);
    cover.insert(at, successor);

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
        MarchKind::Cover(successor),
        "the successor's entry is seeded fresh, so the round impacts it rather than passing \
         through the destroyed piece's flagged entry",
    );
    assert_eq!(result.at, at);
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
        MarchKind::Miss,
        "a ledger entry flagged destroyed must not stop the round",
    );
}
