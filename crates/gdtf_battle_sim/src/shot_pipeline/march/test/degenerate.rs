use super::support::*;
use crate::march::MarchGrids;

#[test]
fn direction_leaving_grid_immediately_is_a_graceful_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(0, 0, 0);
    let dir = Vec3::new(-1.0, 0.0, 0.0);
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
        "a ray leaving the grid immediately is a graceful Miss",
    );
}

#[test]
fn zero_direction_is_a_graceful_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(3, 3, 0);
    let result = march_vector(
        muzzle,
        MarchDir::new(Vec3::ZERO),
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
        "a zero direction is a graceful Miss"
    );
    assert_eq!(
        result.impact, muzzle,
        "the impact point is the muzzle itself"
    );
}

#[test]
fn out_of_grid_muzzle_is_a_graceful_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = SimPos::new(100.0, 5.0, 0.5);
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
        "an out-of-grid muzzle is a graceful Miss"
    );
}

#[test]
fn impact_point_lies_on_the_ray() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let grid = OccupancyGrid::new();

    let at = key(5, 2, 0);
    let mut cover = CoverLedger::new();
    cover.insert(at, cover_entry(HeightBand::High));

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

    let from_muzzle = *result.impact - *muzzle;
    let cross = from_muzzle.cross(dir);
    assert!(
        cross.length() < 1.0e-4,
        "the impact point must lie on the ray (cross {cross:?})",
    );
    assert!(
        from_muzzle.dot(dir) > 0.0,
        "the impact point must be ahead of the muzzle along dir",
    );
}

#[test]
fn band_edges_come_from_tuning() {
    let surface = SurfaceGrid::new();
    let entity = spawn_entity();
    let target = key(5, 2, 0);

    let default = CombatTuning::default();
    let probe = *default.projectile_band_edges.mid_high;

    let mut grid = OccupancyGrid::new();
    grid.set_occupant(target, Some(entity));
    grid.set_occupant_band(target, Some(HeightBand::Mid));
    let cover = CoverLedger::new();

    let muzzle = at_height(2, 2, 0, probe);
    let dir = Vec3::new(1.0, 0.0, 0.0);

    let r_default = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &default,
        far_shooter(),
        no_dead(),
    );
    assert!(
        !matches!(r_default.kind, MarchKind::Ganger(_)),
        "under default edges the round is HIGH and clears the MID ganger",
    );

    let mut raised = CombatTuning::default();
    raised.projectile_band_edges.mid_high = BandEdge::new(probe + 0.1);
    let r_raised = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &surface,
            cover:     &cover,
        },
        &raised,
        far_shooter(),
        no_dead(),
    );
    assert_eq!(
        r_raised.kind,
        MarchKind::Ganger(entity),
        "raising the MID→HIGH edge reclassifies the round MID → it impacts the MID ganger",
    );
}

#[test]
fn marches_over_real_built_grids() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();

    let wall_at = key(6, 3, 0);
    let input = OccupancyInput {
        terrain:   vec![TerrainPlacement::new(wall_at, TerrainKind::Wall)],
        occupants: Vec::<OccupantPlacement>::new(),
    };
    let grid = OccupancyGrid::build_from_occupancy_input(
        &input,
        &bevy::platform::collections::HashSet::default(),
    );

    let wall_entry = cover_entry(HeightBand::High);
    let mut cover = CoverLedger::new();
    cover.insert(wall_at, wall_entry);

    let muzzle = at_height(2, 3, 0, low_above_floor(&tuning));
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
        MarchKind::Cover(wall_entry),
        "the march must stop on the wall built via the occupancy input",
    );
    assert_eq!(result.at, wall_at);
}

#[test]
fn long_diagonal_completes_within_step_cap() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(0, 0, 0);
    let dir = Vec3::new(1.0, 1.0, 0.0);
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
        "a clear corner-to-corner diagonal exits the grid as a Miss",
    );
    assert!(
        result.at.x == i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX) - 1
            || result.at.y == i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX) - 1,
        "the exit cell sits on a far edge: {:?}",
        result.at,
    );
    assert_eq!(result.at.z, 0);
    let _ = MAX_LEVELS;
}
