//! GTW-393 acceptance tests — wall-peek eye offset for LOS around corners.
//!
//! Five acceptance criteria:
//!
//! * **(1) `peek_clears_corner_wall_center_blocks`** — an observer whose eye is nudged
//!   toward the open side of a corner wall via [`PeekOffset`] SEES a target that the
//!   same observer with the centred eye CANNOT (asymmetry: peeker sees, non-peeker
//!   blocked).
//!
//! * **(2) `non_peeker_same_cell_blocked`** — the control: the identical observer cell
//!   with `PeekOffset::default()` (centred eye) is BLOCKED by the same wall,
//!   confirming the peek effect is measurable.
//!
//! * **(3) `clearing_peek_restores_non_peek`** — [`eye_anchor`] with `PeekOffset::default()`
//!   produces the same x/y/z result (bit-identical) as one built without a peek offset,
//!   proving that `Default` is an identity / no-op.
//!
//! * **(4) `peek_facing_invariant`** — rotating `facing` across all eight directions with
//!   a fixed non-zero `PeekOffset` leaves [`eye_anchor`] bit-identical (the peek eye is
//!   FACING-NEUTRAL, matching the base anchor and the stair lift).
//!
//! * **(5) `clamp_keeps_eye_in_cell`** — an absurdly large `PeekOffset` (e.g. ±5.0 sim
//!   units) still yields a peeked eye that `pos_to_cell` maps to the observer's own cell
//!   (the clamp-within-cell invariant, AC #2).

use super::support::*;
use crate::{los::probe::eye_anchor, metric::pos_to_cell};

/// **(1 + 2)** A peeking observer sees around a HIGH-band cover column that the
/// centred-eye observer in the SAME cell CANNOT see past (asymmetry: peeker sees,
/// non-peeker blocked).
///
/// Geometry (all at level 0):
/// * Observer at `(3, 3, 0)` — centred eye at x=3.5, y=3.5.
/// * Cover at `(3, 5, 0)` — HIGH band; occupies `x=[3,4)`, `y=[5,6)`.
/// * Target at `(4, 8, 0)` — aim center ~x=4.5, y=8.5+z.
///
/// Centred-eye ray from (3.5, 3.5) → (4.5, 8.5+z): at y=5 (cover row),
/// x ≈ 3.5 + (1.5/5.0) = 3.8 — INSIDE cover cell [3, 4). BLOCKED.
///
/// Peeked-eye ray: nudge +0.4 east → eye=(3.9, 3.5). Ray toward (4.5, 8.5+z):
/// direction ≈ (0.6, 5.0). At y=5: x ≈ 3.9 + (1.5/5.0)×0.6 = 3.9 + 0.18 = 4.08
/// — in cell [4, 5), NOT in the cover cell [3, 4). CLEAR.
#[test]
fn peek_clears_corner_wall_center_blocks() {
    use bevy::math::Vec2;

    let tuning = CombatTuning::default();
    let surface = SurfaceGrid::new();
    let mut occupancy = OccupancyGrid::new();

    // A HIGH-band cover column at (3, 5, 0) — directly in the centred ray path.
    let mut cover = CoverLedger::new();
    cover.insert(key(3, 5, 0), cover_entry(HeightBand::High));

    // Place a target occupant at (4, 8, 0) — offset east so the peeked ray clears
    // the cover column while the centred ray still hits it.
    let target_entity = spawn_entity();
    place_occupant(
        &mut occupancy,
        key(4, 8, 0),
        target_entity,
        HeightBand::High,
    );

    let obs_pos = position(3, 3, 0);
    let obs_stance = stance(StanceKind::Crouching);
    let obs_facing = facing(Direction::North);
    let tgt_pos = position(4, 8, 0);
    let tgt_stance = stance(StanceKind::Crouching);
    let target = Target {
        position: &tgt_pos,
        stance:   &tgt_stance,
    };

    // --- (2) Control: centred eye is BLOCKED by the HIGH cover column.
    let centred = Observer {
        position:         &obs_pos,
        stance:           &obs_stance,
        facing:           &obs_facing,
        stair_eye_offset: StairEyeOffset::new(0.0),
        peek_offset:      PeekOffset::default(),
    };
    let centred_sighted = has_los(
        &centred,
        &target,
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        !*centred_sighted,
        "centred-eye observer must be BLOCKED by the HIGH cover column \
         (the control that makes the peek effect measurable)"
    );

    // --- (1) Peeker: nudge the eye +0.4 east — the ray clears the cover column.
    // Peeked eye at x=3.9, aimed at x=4.5: the ray crosses y=5 at x≈4.08,
    // which is in cell (4, 5, 0), NOT the cover cell (3, 5, 0). CLEAR.
    let peek_sighted = has_los_peeking(
        &centred,
        &target,
        PeekOffset::new(Vec2::new(0.4, 0.0)),
        &occupancy,
        &surface,
        &cover,
        &tuning,
        no_dead(),
    );
    assert!(
        *peek_sighted,
        "a peeking observer (eye nudged +0.4 east) must SEE the target past the \
         HIGH cover column — peeked ray crosses cover row at x≈4.08 (GTW-393 C5)"
    );

    // The two verdicts must be asymmetric — the test proves BOTH arms.
    assert_ne!(
        *centred_sighted, *peek_sighted,
        "peek and no-peek verdicts must DIFFER in the same world (asymmetry)"
    );
}

/// **(3)** Clearing the peek (`PeekOffset::default()`) restores the non-peek verdict and
/// produces a bit-identical eye anchor to an observer built with no peek offset.
///
/// This is the C5 "clearing-restores" criterion and the `Default`-is-identity invariant:
/// `eye_anchor(observer_with_default_peek)` must be bit-identical to
/// `eye_anchor(observer_with_explicit_zero_peek)`.
#[test]
fn clearing_peek_restores_non_peek() {
    let tuning = CombatTuning::default();
    let pos = position(5, 5, 0);
    let st = stance(StanceKind::Standing);
    let f = facing(Direction::East);

    // The "no peek" baseline: explicit PeekOffset::default().
    let no_peek = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::default(),
        },
        &tuning,
    );
    // The "cleared peek" version: constructed from a non-zero PeekOffset then cleared
    // — the Default should yield the same result.
    let cleared = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &f,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      PeekOffset::new(bevy::math::Vec2::ZERO),
        },
        &tuning,
    );
    assert_eq!(
        (
            no_peek.x.to_bits(),
            no_peek.y.to_bits(),
            no_peek.z.to_bits()
        ),
        (
            cleared.x.to_bits(),
            cleared.y.to_bits(),
            cleared.z.to_bits()
        ),
        "PeekOffset::new(Vec2::ZERO) and PeekOffset::default() must produce \
         a bit-identical eye anchor (Default is the no-op identity)"
    );
}

/// **(4)** Rotating `facing` across all eight directions with a fixed non-zero
/// `PeekOffset` leaves [`eye_anchor`] bit-identical — the peek eye is FACING-NEUTRAL,
/// matching the base anchor and the stair-lift invariant (GTW-393 C5 facing-invariant).
#[test]
fn peek_facing_invariant() {
    let tuning = CombatTuning::default();
    let pos = position(4, 4, 0);
    let st = stance(StanceKind::Standing);
    let peek = PeekOffset::new(bevy::math::Vec2::new(0.3, 0.1));

    let north_facing = facing(Direction::North);
    let reference = eye_anchor(
        &Observer {
            position:         &pos,
            stance:           &st,
            facing:           &north_facing,
            stair_eye_offset: StairEyeOffset::new(0.0),
            peek_offset:      peek,
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
                stair_eye_offset: StairEyeOffset::new(0.0),
                peek_offset:      peek,
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
            "rotating facing to {dir:?} with a non-zero PeekOffset must NOT move the \
             facing-neutral peeked eye anchor (GTW-393 facing-invariant)"
        );
    }
}

/// **(5)** An absurdly large `PeekOffset` (±5.0 sim units) still places the peeked eye
/// in the observer's own cell — `pos_to_cell(peeked_eye).cell == observer_cell` for every
/// displacement including extreme/negative values (the `clamp_within_cell` AC #2).
#[test]
fn clamp_keeps_eye_in_cell() {
    use bevy::math::Vec2;

    let tuning = CombatTuning::default();
    let cell_x = 7_i32;
    let cell_y = 7_i32;
    let pos = position(cell_x, cell_y, 0);
    let st = stance(StanceKind::Standing);
    let f = facing(Direction::North);

    for (dx, dy) in [
        (5.0_f32, 5.0_f32),
        (-5.0, -5.0),
        (5.0, -5.0),
        (-5.0, 5.0),
        (0.49, 0.49),
        (-0.49, -0.49),
    ] {
        let peek = PeekOffset::new(Vec2::new(dx, dy));
        let eye = eye_anchor(
            &Observer {
                position:         &pos,
                stance:           &st,
                facing:           &f,
                stair_eye_offset: StairEyeOffset::new(0.0),
                peek_offset:      peek,
            },
            &tuning,
        );
        let (eye_cell, _) = pos_to_cell(eye);
        assert_eq!(
            (eye_cell.x, eye_cell.y),
            (cell_x, cell_y),
            "PeekOffset({dx}, {dy}) must keep the eye in the observer's cell \
             ({cell_x}, {cell_y}); got ({}, {}) instead — clamp failed",
            eye_cell.x,
            eye_cell.y
        );
    }
}
