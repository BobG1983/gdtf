//! AC #2: `muzzle_position` — forward-of-center in the facing dir, stays WITHIN
//! the cell (the clamp holds), z matches the per-stance level-fraction datum.

use super::support::position;
use crate::{
    central_axis::muzzle_position,
    ganger::{Direction, Facing, Stance, StanceKind},
    metric::{Cell, Level, cell_center, pos_to_cell},
    tuning::CombatTuning,
};

#[test]
fn muzzle_is_forward_of_center_in_the_facing_direction() {
    let tuning = CombatTuning::default();
    let pos = position(4, 7, 2);
    let center = cell_center(Cell::new(4, 7), Level::new(2));

    for dir in [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ] {
        let muzzle = muzzle_position(
            pos,
            Facing::new(dir),
            Stance::new(StanceKind::Standing),
            &tuning,
        );
        let step = dir.forward_step();
        // The ground-plane displacement from center must point ALONG the facing's
        // forward step (positive dot product) — i.e. forward of center.
        let dx = muzzle.x - center.x;
        let dy = muzzle.y - center.y;
        let along = dx * step.x + dy * step.y;
        assert!(
            along > 0.0,
            "{dir:?}: muzzle must be forward of center along the facing (dot {along})",
        );
    }
}

#[test]
fn muzzle_stays_within_the_shooter_cell_for_every_facing() {
    let tuning = CombatTuning::default();
    let cell = Cell::new(4, 7);
    let pos = position(4, 7, 2);

    for dir in [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ] {
        let muzzle = muzzle_position(
            pos,
            Facing::new(dir),
            Stance::new(StanceKind::Standing),
            &tuning,
        );
        // The muzzle's (cell) must equal the shooter's cell — the clamp holds.
        let (muzzle_cell, _) = pos_to_cell(muzzle);
        assert_eq!(
            muzzle_cell, cell,
            "{dir:?}: the muzzle must stay within the shooter's cell",
        );
    }
}

#[test]
fn muzzle_stays_within_cell_even_with_an_oversized_offset() {
    // A forward offset larger than half a cell would, unclamped, leave the cell
    // for some facings. The clamp must still keep it inside. Build a tuning with
    // a deliberately oversized cell-fraction offset (arbitrary, > 0.5).
    let mut tuning = CombatTuning::default();
    tuning.cone_stability.muzzle_forward_offset = crate::tuning::MuzzleForwardOffset::new(0.9);
    let cell = Cell::new(10, 10);
    let pos = position(10, 10, 0);

    for dir in [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ] {
        let muzzle = muzzle_position(
            pos,
            Facing::new(dir),
            Stance::new(StanceKind::Standing),
            &tuning,
        );
        let (muzzle_cell, _) = pos_to_cell(muzzle);
        assert_eq!(
            muzzle_cell, cell,
            "{dir:?}: the clamp must hold even for an oversized offset",
        );
    }
}

#[test]
fn muzzle_z_matches_the_per_stance_level_fraction_datum() {
    let tuning = CombatTuning::default();
    let level = 3u8;
    let pos = position(2, 2, level);
    let heights = &tuning.cone_stability.muzzle_heights;

    // For each stance, z must equal (level as f32) + the stance's muzzle
    // level-fraction (per-axis exact where the datum is exact).
    let cases = [
        (StanceKind::Prone, heights.prone),
        (StanceKind::Crouching, heights.kneel),
        (StanceKind::Standing, heights.stand),
    ];
    for (kind, height) in cases {
        let muzzle = muzzle_position(
            pos,
            Facing::new(Direction::North),
            Stance::new(kind),
            &tuning,
        );
        let expected = f32::from(level) + *height;
        assert_eq!(
            muzzle.z.to_bits(),
            expected.to_bits(),
            "{kind:?}: muzzle z must be level + per-stance muzzle level-fraction",
        );
    }
}
