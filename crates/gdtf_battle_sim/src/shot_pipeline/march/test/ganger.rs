use super::support::*;
use crate::march::MarchGrids;

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

#[test]
fn upper_shooter_march_terminates_on_stair_upper_cell() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let entity = spawn_entity();

    let upper_cell = key(5, 2, 1);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(upper_cell, Some(entity));
    grid.set_occupant_band(upper_cell, Some(HeightBand::Low));

    let muzzle = at_height(2, 2, 1, low_above_floor(&tuning));
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
        MarchKind::Ganger(entity),
        "a shot through the upper stair cell must terminate on the entity \
         (upper-cell Low band is hittable from an upper-level shooter)",
    );
    assert_eq!(
        result.at, upper_cell,
        "impact cell must be the upper stair cell",
    );
}

#[test]
fn ground_shooter_resolves_lower_band_unchanged() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let cover = CoverLedger::new();
    let entity = spawn_entity();

    let lower_cell = key(5, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(lower_cell, Some(entity));
    grid.set_occupant_band(lower_cell, Some(HeightBand::High));
    let upper_cell = key(5, 2, 1);
    grid.set_occupant(upper_cell, Some(entity));
    grid.set_occupant_band(upper_cell, Some(HeightBand::Low));

    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
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
        MarchKind::Ganger(entity),
        "a ground-level HIGH shot must hit the stair occupant's lower cell (Test 4)",
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

    let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
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
