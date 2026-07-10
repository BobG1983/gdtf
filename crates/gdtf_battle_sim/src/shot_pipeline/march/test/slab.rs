//! AC #4: an intact slab stops a climbing ray; a destroyed/absent slab crosses.

use super::support::*;

/// A climbing ray crossing a z-boundary with an intact `Present` slab is stopped
/// (`Slab` result); a `Destroyed` slab at the same boundary lets it cross.
#[test]
fn present_slab_stops_climbing_ray_destroyed_crosses() {
    let tuning = CombatTuning::default();
    let muzzle = center(0, 0, 0);
    let dir = Vec3::new(1.0, 1.0, 1.0);
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();

    // The first z-boundary the staircase crosses is into level 1 at (1,1,*) —
    // the slab keyed at the UPPER level (1,1,1) (floor of the upper storey).
    let slab_at = key(1, 1, 1);

    // Present → stops the round at the boundary.
    let mut present = SurfaceGrid::new();
    present.set_slab(slab_at, SlabState::Present);
    let r_present = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &present,
        &cover,
        &tuning,
        far_shooter(),
        no_dead(),
    );
    assert_eq!(
        r_present.kind,
        MarchKind::Slab,
        "an intact Present slab must stop the climbing ray",
    );
    assert_eq!(r_present.at, slab_at, "the slab result names the slab cell");

    // Destroyed → the round crosses (it climbs on out the top → Miss).
    let mut destroyed = SurfaceGrid::new();
    destroyed.destroy_slab(slab_at);
    let r_destroyed = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &destroyed,
        &cover,
        &tuning,
        far_shooter(),
        no_dead(),
    );
    assert_ne!(
        r_destroyed.kind,
        MarchKind::Slab,
        "a Destroyed slab must NOT stop the round",
    );

    // Absent (the default) → the round also crosses freely.
    let absent = SurfaceGrid::new();
    let r_absent = march_vector(
        muzzle,
        MarchDir::new(dir),
        &grid,
        &absent,
        &cover,
        &tuning,
        far_shooter(),
        no_dead(),
    );
    assert_ne!(
        r_absent.kind,
        MarchKind::Slab,
        "an Absent slab must NOT stop the round",
    );
}
