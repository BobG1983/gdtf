use super::support::*;
use crate::march::MarchGrids;

#[test]
fn present_slab_stops_climbing_ray_destroyed_crosses() {
    let tuning = CombatTuning::default();
    let muzzle = center(0, 0, 0);
    let dir = Vec3::new(1.0, 1.0, 1.0);
    let grid = OccupancyGrid::new();
    let cover = CoverLedger::new();

    let slab_at = key(1, 1, 1);

    let mut present = SurfaceGrid::new();
    present.set_slab(slab_at, SlabState::Present);
    let r_present = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &present,
            cover:     &cover,
        },
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

    let mut destroyed = SurfaceGrid::new();
    destroyed.destroy_slab(slab_at);
    let r_destroyed = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &destroyed,
            cover:     &cover,
        },
        &tuning,
        far_shooter(),
        no_dead(),
    );
    assert_ne!(
        r_destroyed.kind,
        MarchKind::Slab,
        "a Destroyed slab must NOT stop the round",
    );

    let absent = SurfaceGrid::new();
    let r_absent = march_vector(
        muzzle,
        MarchDir::new(dir),
        MarchGrids {
            occupancy: &grid,
            surface:   &absent,
            cover:     &cover,
        },
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
