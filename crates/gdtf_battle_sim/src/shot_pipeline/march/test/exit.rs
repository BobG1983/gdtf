use super::support::*;

#[test]
fn ray_leaving_the_top_is_a_sky_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(3, 3, 0);
    let dir = Vec3::new(0.0, 0.0, 1.0);
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
        "a ray climbing out the top is a clean sky Miss",
    );
}

#[test]
fn ray_leaving_the_bottom_strikes_ground() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(4, 4, 0);
    let dir = Vec3::new(0.0, 0.0, -1.0);
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
        MarchKind::Ground,
        "a ray diving out the bottom strikes the Ground",
    );
    assert_eq!(
        result.at,
        key(4, 4, 0),
        "the Ground result names the exit ground cell",
    );
}

#[test]
fn ray_leaving_laterally_is_a_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(1, 1, 0);
    let dir = Vec3::new(-1.0, 0.0, 0.0);
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
        "a ray leaving laterally is a clean Miss",
    );
}

#[test]
fn no_target_stop_round_continues_to_the_blocker_behind() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let entity = spawn_entity();

    let aim = key(5, 2, 0);
    let behind = key(7, 2, 0);
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(aim, Some(entity));
    grid.set_occupant_band(aim, Some(HeightBand::Low));

    let behind_cover = cover_entry(HeightBand::High);
    let mut cover = CoverLedger::new();
    cover.insert(behind, behind_cover);

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
        MarchKind::Cover(behind_cover),
        "the round must sail over the clearable aim ganger and hit the wall behind",
    );
    assert_eq!(
        result.at, behind,
        "the struck thing is the blocker BEHIND the aim cell"
    );
}
