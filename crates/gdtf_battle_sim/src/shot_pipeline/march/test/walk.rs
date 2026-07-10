//! AC #1: hand-computed cell walks, asserted cell-by-cell.

use super::support::*;

/// A flat axis-aligned ray (East along +x) walks the EXACT cell sequence
/// `(2,2,0) → (3,2,0) → (4,2,0) → …` until it leaves the grid laterally. The
/// walk is captured by standing a HIGH wall-cover in each successive cell and a
/// LOW round, asserting the round impacts the NEXT cell each time (since a HIGH
/// cover stops a LOW round) — so the impact cell IS the next cell in the DDA
/// sequence.
#[test]
fn flat_axis_aligned_ray_walks_expected_cell_sequence() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let muzzle = center(2, 2, 0); // (2.5, 2.5, 0.5)
    let dir = Vec3::new(1.0, 0.0, 0.0); // due East

    // For each expected next cell, stand a HIGH cover there alone and assert the
    // LOW round (the flat ray sits LOW at z = 0.5? no — z=0.5 is HIGH under
    // default edges). Use a HIGH cover so any round impacts; the flat ray's band
    // does not matter for the WALK, only that it stops at that cell.
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
        // The y / level never change on a flat East ray.
        assert_eq!(result.at.y, 2);
        assert_eq!(result.at.z, 0);
    }
}

/// A diagonal + climbing ray (dir = (1,1,1) from the center of (0,0,0)) walks the
/// EXACT Amanatides–Woo staircase `(0,0,0) (1,0,0) (1,1,0) (1,1,1) (2,1,1)
/// (2,2,1) (2,2,2) (3,2,2) (3,3,2) (3,3,3) …`. Each expected cell is probed by
/// standing a HIGH cover ALONE in it (no other cover) and asserting the round
/// stops there — proving that cell is on the walk and in this order.
#[test]
fn diagonal_climbing_ray_walks_expected_staircase() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new(); // no slabs — climbing is free
    let muzzle = center(0, 0, 0); // (0.5, 0.5, 0.5)
    let dir = Vec3::new(1.0, 1.0, 1.0);

    // The hand-computed DDA sequence (ties resolve X, then Y, then Z).
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
