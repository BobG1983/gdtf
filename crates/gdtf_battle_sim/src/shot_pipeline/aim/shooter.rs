//! Snapshot of the shooter state used by aim composition.

use crate::ganger::{Aiming, Facing, Position, Stance, Suppressed};

/// Read-only view of the fields that affect aim and stability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shooter<'a> {
    /// Current stance.
    pub stance:     &'a Stance,
    /// Whether the shooter is carefully aiming.
    pub aiming:     &'a Aiming,
    /// World position.
    pub position:   &'a Position,
    /// Facing direction.
    pub facing:     &'a Facing,
    /// Present when the shooter is suppressed.
    pub suppressed: Option<&'a Suppressed>,
}
