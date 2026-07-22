//! GTW-390 acceptance tests — the authored stair-tile eye-offset for height-aware LOS.
//!
//! Three acceptance criteria:
//!
//! * **(a) `stair_kneel_clears_mid_cover_ground_kneel_blocks`** — a KNEELING observer on an
//!   authored stair tile sees over a [`HeightBand::Mid`] cover cell that a ground-level
//!   kneeling observer (same position, no stair) CANNOT. Uses same-level flat geometry
//!   (observer + target both at level 0) to isolate the eye-offset from multi-storey
//!   ray ambiguity.
//!
//! * **(b) `stair_facing_invariant`** — rotating the observer's [`Facing`] across all
//!   eight directions on a stair cell leaves the [`eye_anchor`] result bit-identical
//!   (the stair eye-lift is FACING-NEUTRAL, just like the base eye anchor).
//!
//! * **(c) `non_stair_cell_yields_zero_offset`** — a plain `OccupancyGrid` (no stair cells
//!   registered) returns `StairEyeOffset(0.0)` for every cell, and the resulting
//!   [`eye_anchor`] z is bit-identical to a call using the explicit zero offset.

use super::support::*;
use crate::{los::probe::eye_anchor, occupancy::StairEyeOffset};

/// **(a)** A KNEELING observer on an authored stair tile SEES over a
/// [`HeightBand::Mid`] cover cell; the same position without the stair lift is
/// BLOCKED by that cover (GTW-390 C6a).
///
/// Geometry: observer at `(0, 5, 0)`, cover at `(3, 5, 0)`, target at `(6, 5, 0)`,
/// all on level 0. The ray stays within a single storey so there is no multi-storey
/// slab-boundary ambiguity. The stair offset (`+0.5`) pushes the eye from 0.45
/// (kneel) to 0.95, lifting it above the Mid-band top and clearing the cover; the
/// plain ground-kneel eye at 0.45 is below the Mid-band top and is blocked.
#[test]
fn stair_kneel_clears_mid_cover_ground_kneel_blocks() {
    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();

    // A MID-band cover cell strictly between the observer and the target on the same
    // level — the band the stair lift must clear.
    let mut cover = CoverLedger::new();
    cover.insert(key(3, 5, 0), cover_entry(HeightBand::Mid));

    let obs_pos = position(0, 5, 0);
    let obs_stance = stance(StanceKind::Crouching);
    let obs_facing = facing(Direction::East);
    let tgt_pos = position(6, 5, 0);
    let tgt_stance = stance(StanceKind::Crouching);
    let target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };

    // --- Ground-kneel: NO stair offset — standard occupancy grid with no stair cells.
    let plain_occupancy = OccupancyGrid::new();
    let ground_kneel = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let ground_sighted = has_los(
        &ground_kneel,
        &target,
        &plain_occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*ground_sighted,
        "a ground-kneel observer (no stair) must be BLOCKED by a MID cover \
         (the control geometry that makes the stair effect measurable)"
    );

    // --- Stair-kneel: stair cell registered at the observer's position — grid has the
    //     authored stair endpoint; the observer carries the looked-up offset.
    let mut stair_occupancy = OccupancyGrid::new();
    stair_occupancy.mark_stair_cell(key(0, 5, 0));
    let stair_offset = stair_occupancy.stair_eye_offset_at(&key(0, 5, 0));
    let stair_kneel = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: stair_offset,
        peek_offset:      PeekOffset::default(),
    };
    let stair_sighted = has_los(
        &stair_kneel,
        &target,
        &stair_occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *stair_sighted,
        "a kneeling observer on an authored stair tile must SEE OVER a MID cover \
         (+0.5 eye-lift clears the band the ground-kneel was blocked by)"
    );
}

/// **(b)** Rotating only the observer's `Facing` on a stair cell across all eight
/// directions leaves the [`eye_anchor`] result bit-identical — the stair eye-lift is
/// FACING-NEUTRAL, matching the base anchor invariant (GTW-390 C6b).
///
/// Builds an `Observer` with `stair_eye_offset = StairEyeOffset(0.5)` (the authored
/// value for a non-Prone observer on a stair tile) and asserts that each of the eight
/// compass facings yields the same x/y/z triplet as the North reference, via
/// `to_bits()` for exact floating-point equality.
#[test]
fn stair_facing_invariant() {
    let tuning = CombatTuning::default();
    let pos = position(4, 4, 0);
    let st = stance(StanceKind::Crouching);

    // The stair offset for a non-Prone kneeling observer — +0.5 (the authored value).
    let stair_off = StairEyeOffset::new(0.5);

    let north_facing = facing(Direction::North);
    let reference = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &north_facing,
            stair_eye_offset: stair_off,
            peek_offset:      PeekOffset::default(),
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
                position:         &pos,
                stance:           &st,
                facing:           &f,
                stair_eye_offset: stair_off,
                peek_offset:      PeekOffset::default(),
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
            "rotating facing to {dir:?} on a stair cell must NOT move the \
             facing-neutral stair eye anchor (GTW-390 facing-invariant)"
        );
    }
}

/// **(c)** A plain `OccupancyGrid` with no stair cells returns `StairEyeOffset(0.0)`
/// for every queried cell, and the [`eye_anchor`] z from that zero-offset observer is
/// bit-identical to one built with an explicit `StairEyeOffset(0.0)` field — the
/// non-stair path is a provable no-op (GTW-390 C6c).
///
/// Verifies that `OccupancyGrid::stair_eye_offset_at` returns the zero default and
/// that the resulting eye is identical to the non-stair baseline, confirming the
/// stair feature touches nothing on non-stair cells.
#[test]
fn non_stair_cell_yields_zero_offset() {
    let tuning = CombatTuning::default();
    let plain = OccupancyGrid::new(); // no stair cells registered

    // Any queried cell on a fresh grid should return the zero offset — verified by
    // comparing bit patterns to avoid the float_cmp clippy lint (exact integer
    // equality: 0.0_f32.to_bits() == 0x0000_0000, a manifest constant).
    let queried_cell = key(5, 5, 0);
    let offset = plain.stair_eye_offset_at(&queried_cell);
    assert_eq!(
        (*offset).to_bits(),
        0.0_f32.to_bits(),
        "a plain OccupancyGrid with no stair cells must return StairEyeOffset(0.0) \
         for every cell"
    );

    // The eye_anchor call with the looked-up zero offset must be bit-identical to one
    // with an explicit zero — proving the non-stair path is a provable no-op.
    let pos = position(5, 5, 0);
    let st = stance(StanceKind::Standing);
    let f = facing(Direction::East);
    let baseline = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    let from_grid = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: offset,
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    assert_eq!(
        (
            baseline.x.to_bits(),
            baseline.y.to_bits(),
            baseline.z.to_bits()
        ),
        (
            from_grid.x.to_bits(),
            from_grid.y.to_bits(),
            from_grid.z.to_bits()
        ),
        "a non-stair cell's StairEyeOffset(0.0) must yield a bit-identical eye \
         anchor to an explicit zero-offset (the non-stair path is a no-op)"
    );
}

/// Prone observers on a stair cell get zero eye-lift — Prone → 0.0 even on an
/// authored stair tile (GTW-390 C3: "Prone → 0.0 (no lift)").
///
/// Verifies that `eye_anchor` for a Prone observer on a stair cell (carrying
/// `StairEyeOffset(0.5)`) returns the SAME z as a Prone observer with
/// `StairEyeOffset(0.0)` — the stance gate suppresses the lift.
#[test]
fn prone_stair_no_lift() {
    let tuning = CombatTuning::default();
    let pos = position(3, 3, 0);
    let prone_st = stance(StanceKind::Prone);
    let f = facing(Direction::East);

    let baseline = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &prone_st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    let stair = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &prone_st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.5),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    assert_eq!(
        baseline.z.to_bits(),
        stair.z.to_bits(),
        "a Prone observer on a stair cell must get ZERO eye-lift \
         (stance gate: Prone → 0.0, the stair offset is suppressed)"
    );
}
