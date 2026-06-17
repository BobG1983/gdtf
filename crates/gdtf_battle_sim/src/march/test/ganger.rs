//! AC #2: ganger banding — equal-or-lower impacts, strictly-higher sails.

use super::support::*;

/// A ganger occupant at an EQUAL-or-lower band impacts: a LOW round vs a LOW
/// ganger returns `Ganger(entity)` at that cell.
#[test]
fn ganger_at_equal_band_impacts_returning_entity() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let entity = spawn_entity();

    let target = key(5, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(target, Some(entity));
    grid.set_occupant_band(target, Some(HeightBand::Low));
    let cover = CoverLedger::new();

    // A round flying at LOW (z just above the floor) through the target cell.
    let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0); // flat East at constant low z

    let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
    assert_eq!(
        result.kind,
        MarchKind::Ganger(entity),
        "a LOW round vs a LOW ganger must impact and carry the Entity",
    );
    assert_eq!(result.at, target, "the impact cell is the ganger's cell");
    assert_eq!(
        result.band,
        HeightBand::Low,
        "the round's band at the crossing"
    );
}

/// A strictly-higher round sails over a ganger and continues: a HIGH round vs a
/// LOW ganger does NOT impact the ganger — it flies past (here off the grid → a
/// Miss, since nothing is behind it).
#[test]
fn strictly_higher_round_sails_over_ganger() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let entity = spawn_entity();

    let target = key(5, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(target, Some(entity));
    grid.set_occupant_band(target, Some(HeightBand::Low));
    let cover = CoverLedger::new();

    // A HIGH round (z high within the storey) flat East through the target cell.
    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0);

    let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
    assert!(
        !matches!(result.kind, MarchKind::Ganger(_)),
        "a HIGH round must sail over a LOW ganger, not impact it",
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "with nothing behind, the round flies off the grid laterally → Miss",
    );
}
