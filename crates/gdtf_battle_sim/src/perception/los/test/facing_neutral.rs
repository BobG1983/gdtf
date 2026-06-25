//! AC: the eye is facing-neutral — rotating ONLY the observer's `Facing` (same
//! `Position` + `Stance`) changes neither the eye anchor nor the `has_los` verdict
//! (GTW-337 clause 3). FOV is omni-directional: a watcher facing away still SEES.

use super::support::*;
use crate::{ganger::Direction, los::probe::eye_anchor};

/// Rotating only the observer's facing across all eight directions leaves the eye
/// anchor BIT-IDENTICAL — the eye reads `cell_center + per-stance muzzle height`, never
/// the per-facing forward offset.
#[test]
fn eye_anchor_is_facing_invariant() {
    let tuning = CombatTuning::default();
    let pos = position(5, 5, 0);
    let st = stance(StanceKind::Standing);

    let north_facing = facing(Direction::North);
    let reference = eye_anchor(
        &Observer {
            position: &pos,
            stance:   &st,
            facing:   &north_facing,
        },
        &tuning,
    );

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
        let f = facing(dir);
        let eye = eye_anchor(
            &Observer {
                position: &pos,
                stance:   &st,
                facing:   &f,
            },
            &tuning,
        );
        assert_eq!(
            (eye.x.to_bits(), eye.y.to_bits(), eye.z.to_bits()),
            (
                reference.x.to_bits(),
                reference.y.to_bits(),
                reference.z.to_bits()
            ),
            "rotating facing to {dir:?} must NOT move the facing-neutral eye anchor",
        );
    }
}

/// Rotating only the observer's facing leaves the `has_los` VERDICT unchanged — proven
/// against a geometry that is genuinely blocked (a HIGH wall strictly between), so the
/// invariant holds for a non-trivial verdict, not just an open line.
#[test]
fn verdict_is_facing_invariant() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let occupancy = OccupancyGrid::new();
    let mut cover = CoverLedger::new();
    cover.insert(key(5, 5, 0), cover_entry(HeightBand::High));

    let from_pos = position(2, 5, 0);
    let from_stance = stance(StanceKind::Standing);
    let to_pos = position(8, 5, 0);
    let to_stance = stance(StanceKind::Standing);
    let target = Target {
        position: &to_pos,
        stance:   &to_stance,
    };

    let mut verdicts = Vec::new();
    for dir in [
        Direction::East,
        Direction::West,
        Direction::North,
        Direction::South,
    ] {
        let f = facing(dir);
        let observer = Observer {
            position: &from_pos,
            stance:   &from_stance,
            facing:   &f,
        };
        verdicts.push(*has_los(
            &observer,
            &target,
            &occupancy,
            &surface,
            &cover,
            &tuning,
            no_dead(),
        ));
    }

    assert!(
        verdicts.windows(2).all(|w| w[0] == w[1]),
        "the has_los verdict must be identical across all observer facings, got {verdicts:?}",
    );
    // And the underlying geometry is non-trivially BLOCKED (the wall stops it), so this
    // is not a vacuous all-CLEAR.
    assert!(
        !verdicts[0],
        "the control geometry must be BLOCKED (a HIGH wall between)"
    );
}
