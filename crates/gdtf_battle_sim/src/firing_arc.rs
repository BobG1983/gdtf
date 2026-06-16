//! The **firing-arc containment** geometry helper (GTW-242) — the pure predicate the
//! fire dispatch gates a shot on before deciding whether the shooter must turn.
//!
//! A shooter may fire directly at a target only when that target lies inside its facing
//! arc: the angle between the facing unit vector and the actor→target ground vector is
//! `<= firing_arc / 2`. A target OUTSIDE the arc is not rejected here — it tells the
//! dispatch to compute the turn-into-arc cost and gate the combined turn+fire spend
//! (`docs/combat/resolution.md` §1 / the targeting section). This module owns ONLY the
//! geometry decision; the TU economy + facing mutation live in
//! [`crate::acts::dispatch_fire`].
//!
//! **Continuous angle, NOT 8-way snapping** (the design contract): the test uses the true
//! angle between the [`Direction::forward_step`] facing vector and the cell delta, so a
//! 120° arc admits any target within ±60° of the facing — not just the three nearest
//! compass headings. Pure function over a [`Direction`] + the two [`Cell`]s + a
//! [`FiringArc`]; no [`World`](bevy::ecs::world::World), no RNG — unit-testable with bare
//! values. **Zero pixels** — sim-unit ground-plane voxel coordinates only.

use bevy::math::Vec2;

use crate::{ganger::Direction, metric::Cell, tuning::FiringArc};

/// Whether `target_cell` lies **inside** the shooter's firing arc — the continuous-angle
/// containment predicate the GTW-242 fire dispatch gates on.
///
/// Returns `true` iff the angle between the shooter's facing unit vector (the ground-plane
/// projection of [`Direction::forward_step`]) and the actor→target ground vector
/// `(target_cell − actor_cell)` is `<= *arc / 2.0` degrees (the **inclusive** boundary —
/// a target exactly at the arc edge is in-arc). A **co-located** target (zero delta:
/// `target_cell == actor_cell`) is degenerate → treated as in-arc (no turn needed), with
/// no NaN from the angle computation.
///
/// Continuous-angle, NOT 8-way snapping: the facing vector is the true unit direction, so
/// a 120° arc admits any target within ±60° of the facing. The z axis is irrelevant to a
/// ground facing, so the test is purely on the x/y plane (both vectors are projected to
/// [`Vec2`]). Pure, total, no panic, no NaN.
#[must_use]
pub fn target_in_arc(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    arc: &FiringArc,
) -> bool {
    // The actor→target ground vector (cell units). Cell derefs to IVec2; the delta is the
    // x/y displacement, projected to the ground plane (z is irrelevant to a ground facing).
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer cell delta is exact for this range"
    )]
    let delta = Vec2::new(
        (target_cell.x - actor_cell.x) as f32,
        (target_cell.y - actor_cell.y) as f32,
    );
    // Co-located target — zero delta. Degenerate (no direction toward yourself); treat as
    // in-arc so the shooter never needs to turn to fire at its own cell. Guards the NaN a
    // normalize/acos of a zero vector would otherwise produce.
    if delta == Vec2::ZERO {
        return true;
    }

    // The facing's ground-plane unit vector (forward_step is z = 0 and unit length; project
    // to Vec2 and re-normalize defensively — normalize_or_zero never yields NaN).
    let forward = facing.forward_step();
    let facing_dir = Vec2::new(forward.x, forward.y).normalize_or_zero();
    let target_dir = delta.normalize_or_zero();

    // The cosine of the angle between the two unit vectors, clamped into [-1, 1] so float
    // error can never push acos into NaN territory. acos returns radians in 0..=π.
    let cos = facing_dir.dot(target_dir).clamp(-1.0, 1.0);
    let angle_deg = cos.acos().to_degrees();

    // In-arc iff within HALF the full arc width off the facing (the ±arc/2 half-angle).
    // Inclusive boundary: a target exactly at the edge is in-arc. `arc` is `&FiringArc`;
    // `**arc` reaches the inner degrees (one deref for the `&`, one for the newtype Deref).
    angle_deg <= **arc / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // The actor's cell every case fires from — an arbitrary interior cell (the geometry
    // is translation-invariant, so the absolute position is irrelevant).
    const ACTOR: Cell = Cell::new(5, 5);

    // GTW-242 — a target directly along the facing (0° off-axis) is always in-arc, for any
    // positive arc width. East faces +x, so a target at greater x on the same row is dead
    // ahead.
    #[test]
    fn on_axis_target_is_in_arc() {
        let arc = FiringArc::new(120.0);
        assert!(
            target_in_arc(Direction::East, ACTOR, Cell::new(8, 5), &arc),
            "a target dead ahead (0° off the facing) must be in-arc",
        );
    }

    // GTW-242 — a target 90° off the facing is OUTSIDE a 120° (±60°) arc. East faces +x; a
    // target at greater y (South, +y in the −Y-North convention) is 90° off.
    #[test]
    fn ninety_degrees_off_is_out_of_arc_for_120() {
        let arc = FiringArc::new(120.0);
        assert!(
            !target_in_arc(Direction::East, ACTOR, Cell::new(5, 8), &arc),
            "a target 90° off the facing must be out of a 120° (±60°) arc",
        );
    }

    // GTW-242 boundary (AC4) — a target at EXACTLY arc/2 off the facing is IN-arc (the
    // inclusive ≤ boundary). A 45° diagonal target is exactly arc/2 of a 90° arc; pinning
    // the inclusive boundary in one assertion so a regression to `<` flips it.
    #[test]
    fn target_exactly_at_arc_edge_is_inclusive_in_arc() {
        // East faces +x; SouthEast (the +x,+y diagonal target) is exactly 45° off-axis.
        let edge = Cell::new(8, 8); // +3,+3 from ACTOR — a 45° diagonal
        // A 90° arc has a 45° half-angle — the diagonal sits EXACTLY on the edge.
        let arc_at_edge = FiringArc::new(90.0);
        assert!(
            target_in_arc(Direction::East, ACTOR, edge, &arc_at_edge),
            "a target at exactly arc/2 off the facing must be IN-arc (inclusive ≤ boundary)",
        );
        // One hair NARROWER than the diagonal's 45° must now exclude it — proving the
        // boundary is the discriminator, not a coincidence.
        let arc_just_inside = FiringArc::new(89.0);
        assert!(
            !target_in_arc(Direction::East, ACTOR, edge, &arc_just_inside),
            "a 45° target must be OUT of an 89° (<45° half-angle) arc",
        );
    }

    // GTW-242 AC6 — a co-located target (zero delta) is in-arc, with no NaN/panic from the
    // angle computation (the degenerate zero-vector guard).
    #[test]
    fn co_located_target_is_in_arc_no_nan() {
        let arc = FiringArc::new(120.0);
        assert!(
            target_in_arc(Direction::North, ACTOR, ACTOR, &arc),
            "a co-located target (zero vector) must be in-arc (no turn, no NaN)",
        );
    }

    // GTW-242 AC5 — the arc is the discriminator: a fixed 90°-off-axis target flips from
    // out-of-arc to in-arc as the arc widens past 180°, and a 0° target is in-arc even for
    // a razor-thin arc. Proves the predicate reads the FiringArc, not a hardcoded width.
    #[test]
    fn arc_width_drives_containment() {
        let off_axis = Cell::new(5, 8); // 90° off East (South)
        // A wide arc (200° ⇒ ±100°) admits the 90°-off target.
        assert!(
            target_in_arc(Direction::East, ACTOR, off_axis, &FiringArc::new(200.0)),
            "a wide (200°) arc must admit a 90°-off target",
        );
        // A narrow arc (10° ⇒ ±5°) excludes it.
        assert!(
            !target_in_arc(Direction::East, ACTOR, off_axis, &FiringArc::new(10.0)),
            "a narrow (10°) arc must exclude a 90°-off target",
        );
        // Even a razor-thin arc admits a perfectly on-axis target.
        let on_axis = Cell::new(8, 5);
        assert!(
            target_in_arc(Direction::East, ACTOR, on_axis, &FiringArc::new(1.0)),
            "an on-axis target is in-arc even for a razor-thin arc",
        );
    }
}
