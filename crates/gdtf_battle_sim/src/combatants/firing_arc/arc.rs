//! The firing-arc containment implementation — [`target_in_arc`]. See the module docs
//! (`super`) for the continuous-angle contract and the degenerate-target guard.

use bevy::{math::Vec2, prelude::Deref};

use crate::{ganger::Direction, metric::Cell, tuning::FiringArc};

/// Whether a target lies inside the shooter's firing arc — the [`target_in_arc`]
/// containment verdict the GTW-242 fire dispatch gates on.
///
/// `true` means the target is within `±arc/2` of the facing (a direct shot is legal);
/// `false` means the shooter must turn first. A distinct arc-containment predicate, not a
/// bare `bool`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetInArc(bool);

impl TargetInArc {
    /// Build the arc-containment verdict from the computed angle test.
    #[must_use]
    pub const fn new(in_arc: bool) -> Self {
        Self(in_arc)
    }
}

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
) -> TargetInArc {
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
        return TargetInArc::new(true);
    }

    // The facing's ground-plane unit vector (forward_step is z = 0 and unit length; project
    // to Vec2 and re-normalize defensively — normalize_or_zero never yields NaN).
    let forward = *facing.forward_step();
    let facing_dir = Vec2::new(forward.x, forward.y).normalize_or_zero();
    let target_dir = delta.normalize_or_zero();

    // The cosine of the angle between the two unit vectors, clamped into [-1, 1] so float
    // error can never push acos into NaN territory. acos returns radians in 0..=π.
    let cos = facing_dir.dot(target_dir).clamp(-1.0, 1.0);
    let angle_deg = cos.acos().to_degrees();

    // In-arc iff within HALF the full arc width off the facing (the ±arc/2 half-angle).
    // Inclusive boundary: a target exactly at the edge is in-arc. `arc` is `&FiringArc`;
    // `**arc` reaches the inner degrees (one deref for the `&`, one for the newtype Deref).
    TargetInArc::new(angle_deg <= **arc / 2.0)
}
