use super::support::*;

#[test]
fn flat_axis_aligned_ray_walks_expected_cell_sequence() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let muzzle = center(2, 2, 0);
    let dir = Vec3::new(1.0, 0.0, 0.0);

    for next_x in 3..=8 {
        let mut cover = CoverLedger::new();
        cover.insert(key(next_x, 2, 0), cover_entry(HeightBand::High));
        let grid = OccupancyGrid::new();

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
            result.at,
            key(next_x, 2, 0),
            "the flat East ray must cross cell ({next_x}, 2, 0) in sequence",
        );
        assert!(
            matches!(result.kind, MarchKind::Cover(_)),
            "the HIGH cover at ({next_x},2,0) must stop the round",
        );
        assert_eq!(result.at.y, 2);
        assert_eq!(result.at.z, 0);
    }
}

#[test]
fn diagonal_climbing_ray_walks_expected_staircase() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let muzzle = center(0, 0, 0);
    let dir = Vec3::new(1.0, 1.0, 1.0);

    let expected = [
        key(1, 0, 0),
        key(1, 1, 0),
        key(1, 1, 1),
        key(2, 1, 1),
        key(2, 2, 1),
        key(2, 2, 2),
        key(3, 2, 2),
        key(3, 3, 2),
        key(3, 3, 3),
    ];

    for cell in expected {
        let mut cover = CoverLedger::new();
        cover.insert(cell, cover_entry(HeightBand::High));
        let grid = OccupancyGrid::new();

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
            result.at, cell,
            "the diagonal climbing ray must cross {cell:?} in DDA order",
        );
        assert!(
            matches!(result.kind, MarchKind::Cover(_)),
            "the HIGH cover at {cell:?} must stop the round",
        );
    }
}
