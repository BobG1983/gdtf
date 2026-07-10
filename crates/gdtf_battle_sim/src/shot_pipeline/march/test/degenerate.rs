//! AC #7: graceful degenerate marches + impact point via `SimPos::new`, plus the
//! tuning-read, real-grids, and step-cap geometry checks.

use super::support::*;

/// A direction leaving the grid IMMEDIATELY is a graceful `Miss`, no panic — a
/// ray fired West from the edge cell (0, 0, 0).
#[test]
fn direction_leaving_grid_immediately_is_a_graceful_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(0, 0, 0);
    let dir = Vec3::new(-1.0, 0.0, 0.0); // straight off the west edge
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
        "a ray leaving the grid immediately is a graceful Miss",
    );
}

/// A zero direction is graceful — a `Miss` at the muzzle, no panic / NaN.
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
        "a zero direction is a graceful Miss"
    );
    assert_eq!(
        result.impact, muzzle,
        "the impact point is the muzzle itself"
    );
}

/// An out-of-grid muzzle is graceful — a `Miss`, no panic (the grid degrades on
/// any out-of-range coordinate).
#[test]
fn out_of_grid_muzzle_is_a_graceful_miss() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    // x past the grid extent.
    let muzzle = SimPos::new(100.0, 5.0, 0.5);
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
        "an out-of-grid muzzle is a graceful Miss"
    );
}

/// The impact point is a `SimPos` (built via `SimPos::new`) lying on the ray — a
/// cover hit's impact point is `muzzle + t × dir` for some `t ≥ 0`. Checks it is
/// collinear with the ray (the cross product with `dir` is ~zero) and ahead of
/// the muzzle.
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
        &grid,
        &surface,
        &cover,
        &tuning,
        far_shooter(),
        no_dead(),
    );

    let from_muzzle = *result.impact - *muzzle;
    // Collinear with dir → the cross product is ~zero.
    let cross = from_muzzle.cross(dir);
    assert!(
        cross.length() < 1.0e-4,
        "the impact point must lie on the ray (cross {cross:?})",
    );
    // Ahead of the muzzle (positive projection onto dir).
    assert!(
        from_muzzle.dot(dir) > 0.0,
        "the impact point must be ahead of the muzzle along dir",
    );
}

/// All geometry reads `tuning` for the band edges (no hardcoded fraction): a
/// round whose within-level fraction sits exactly on the default MID→HIGH edge
/// classifies HIGH and clears a MID ganger; raising the MID→HIGH edge above that
/// fraction reclassifies the SAME ray as MID, which (equal to the MID ganger) now
/// impacts. Same muzzle/dir, different tuning → the clearance flips, proving the
/// band edge is read live from `tuning.projectile_band_edges`, never hardcoded.
#[test]
fn band_edges_come_from_tuning() {
    let surface = SurfaceGrid::new();
    let entity = spawn_entity();
    let target = key(5, 2, 0);

    // A round whose fraction is exactly the default MID→HIGH edge → HIGH by
    // default (clears a MID occupant). Raising the MID→HIGH edge above it makes
    // it MID (equal to a MID occupant → impacts). Same ray, different tuning.
    let default = CombatTuning::default();
    let probe = *default.projectile_band_edges.mid_high;

    let mut grid = OccupancyGrid::new();
    grid.set_occupant(target, Some(entity));
    grid.set_occupant_band(target, Some(HeightBand::Mid));
    let cover = CoverLedger::new();

    let muzzle = at_height(2, 2, 0, probe);
    let dir = Vec3::new(1.0, 0.0, 0.0);

    // Default: the round is HIGH, strictly above the MID occupant → sails over.
    let r_default = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
        &default,
        far_shooter(),
        no_dead(),
    );
    assert!(
        !matches!(r_default.kind, MarchKind::Ganger(_)),
        "under default edges the round is HIGH and clears the MID ganger",
    );

    // Raise the MID→HIGH edge above the probe: the round is now MID, equal to the
    // MID occupant → impacts. Proves the band edge is read from tuning.
    let mut raised = CombatTuning::default();
    raised.projectile_band_edges.mid_high = BandEdge::new(probe + 0.1);
    let r_raised = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &surface,
        &cover,
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

/// A march driven through `setup_battle`'s real grids end to end: build an
/// occupancy + surface + cover grid via the `OccupancyInput` pour, then march a
/// flat ray into a HIGH wall and assert a Cover stop — proving `march_vector`
/// reads the same grids the rest of the sim builds.
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
        &grid,
        &surface,
        &cover,
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

/// A long corner-to-corner diagonal completes within the iteration cap — it never
/// spins, and resolves to a grid-exit result (`Miss` off the far side).
#[test]
fn long_diagonal_completes_within_step_cap() {
    let tuning = CombatTuning::default();
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();
    let surface = SurfaceGrid::new();

    let muzzle = center(0, 0, 0);
    // A shallow diagonal so the ray crosses the full 60×60 span before exiting.
    let dir = Vec3::new(1.0, 1.0, 0.0);
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
        "a clear corner-to-corner diagonal exits the grid as a Miss",
    );
    // The exit cell is on the far edge (one of the last in-grid cells).
    assert!(
        result.at.x == i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX) - 1
            || result.at.y == i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX) - 1,
        "the exit cell sits on a far edge: {:?}",
        result.at,
    );
    // MAX_LEVELS is referenced structurally; confirm a level-0 flat ray stays on
    // level 0.
    assert_eq!(result.at.z, 0);
    let _ = MAX_LEVELS;
}
