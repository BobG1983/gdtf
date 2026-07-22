//! AC #2: ganger banding — equal-or-lower impacts, strictly-higher sails.
//! GTW-391: upper-cell stair presence is hittable via [`march_vector`](crate::march::march_vector).

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

/// GTW-391 Test 2 (revised): an upper-cell stair presence is hittable — a round
/// fired FROM level z+1 through cell `(x,y,z+1)` terminates on the Low band written
/// there by `register_stair_presence`, returning `MarchKind::Ganger(entity)` at the
/// upper cell. This is the FIRE path the design confirms correct (fire→upper-cell Low
/// band→`impact_at`→`round_clears_occupant(test_band, Low)` == Impacts for a flat
/// shot at level z+1).
#[test]
fn upper_shooter_march_terminates_on_stair_upper_cell() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let entity = spawn_entity();

    // The stair ganger's upper cell: (x=5, y=2, z=1).
    let upper_cell = key(5, 2, 1);
    let mut grid = OccupancyGrid::new();
    // Simulate what register_stair_presence writes at the upper cell:
    // occupant = entity, band = Low.
    grid.set_occupant(upper_cell, Some(entity));
    grid.set_occupant_band(upper_cell, Some(HeightBand::Low));

    // Shoot flat from level 1 (above the stair occupant's lower cell at z=0)
    // through the upper cell — a LOW round at storey-1 floor level.
    let muzzle = at_height(2, 2, 1, low_above_floor(&tuning));
    let dir = Vec3::new(1.0, 0.0, 0.0); // flat East at storey-1 low z

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
        MarchKind::Ganger(entity),
        "a shot through the upper stair cell must terminate on the entity \
         (upper-cell Low band is hittable from an upper-level shooter, GTW-391)",
    );
    assert_eq!(
        result.at, upper_cell,
        "impact cell must be the upper stair cell",
    );
}

/// GTW-391 Test 4: a ground-level shot through the LOWER cell of a stair occupant
/// resolves the lower stance band unchanged — the dual-cell upper presence does NOT
/// alter the lower-cell fire path.
///
/// A stair occupant's lower cell carries its actual stance band (e.g. `High` for
/// Standing); `register_stair_presence` writes the upper `(cell, level+1)` with `Low`.
/// A flat shot from level z through the lower cell must terminate on the lower-cell
/// band, not the upper, proving the lower path is identical to a
/// non-stair control.
#[test]
fn ground_shooter_resolves_lower_band_unchanged() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let entity = spawn_entity();

    // Stair occupant at lower cell (z=0): Standing → High band.
    // The upper cell (z=1) would carry Low band, but we test the LOWER cell path.
    let lower_cell = key(5, 2, 0);
    let mut grid = OccupancyGrid::new();
    // Simulate the lower cell as register_stair_presence would write it.
    grid.set_occupant(lower_cell, Some(entity));
    grid.set_occupant_band(lower_cell, Some(HeightBand::High));
    // Upper cell is also written as the stair presence — but the ground shot
    // must hit the lower cell, not the upper.
    let upper_cell = key(5, 2, 1);
    grid.set_occupant(upper_cell, Some(entity));
    grid.set_occupant_band(upper_cell, Some(HeightBand::Low));

    // Shoot HIGH (standing-level) from level 0, flat East — the same shot that would
    // hit a non-stair standing occupant at this cell (identical control outcome).
    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
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
        MarchKind::Ganger(entity),
        "a ground-level HIGH shot must hit the stair occupant's lower cell (GTW-391 Test 4)",
    );
    assert_eq!(
        result.at, lower_cell,
        "the impact cell must be the lower stair cell, not the upper",
    );
    assert_eq!(
        result.band,
        HeightBand::High,
        "the impact band must be the lower-cell stance band (High for Standing)",
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
        !matches!(result.kind, MarchKind::Ganger(_)),
        "a HIGH round must sail over a LOW ganger, not impact it",
    );
    assert_eq!(
        result.kind,
        MarchKind::Miss,
        "with nothing behind, the round flies off the grid laterally → Miss",
    );
}
