//! Angle test between facing and target cell.

use bevy::{math::Vec2, prelude::Deref};

use crate::{ganger::Direction, metric::Cell, tuning::FiringArc};

/// Whether the target is inside the half-arc of the facing cone.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetInArc(bool);

impl TargetInArc {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(in_arc: bool) -> Self {
        Self(in_arc)
    }
}

/// True if `target_cell` is within half of `arc` degrees of `facing` from `actor_cell`.
#[must_use]
pub fn target_in_arc(
    facing: Direction,
    actor_cell: Cell,
    target_cell: Cell,
    arc: &FiringArc,
) -> TargetInArc {
    #[expect(
        clippy::cast_precision_loss,
        reason = "grid coords are tiny (0..60); the f32 conversion of the integer cell delta is exact for this range"
    )]
    let delta = Vec2::new(
        (target_cell.x - actor_cell.x) as f32,
        (target_cell.y - actor_cell.y) as f32,
    );
    if delta == Vec2::ZERO {
        return TargetInArc::new(true);
    }

    let forward = *facing.forward_step();
    let facing_dir = Vec2::new(forward.x, forward.y).normalize_or_zero();
    let target_dir = delta.normalize_or_zero();

    let cos = facing_dir.dot(target_dir).clamp(-1.0, 1.0);
    let angle_deg = cos.acos().to_degrees();

    TargetInArc::new(angle_deg <= **arc / 2.0)
}
