//! The [`Shooter`] borrow-view — the bundle of ganger components both composers
//! reason a shot's stability and cone width over.

use crate::ganger::{Aiming, Facing, Position, Stance, Suppressed};

/// The shooter read-state both composers reason over — the ganger components a
/// shot's stability and cone width depend on, bundled into one named record.
///
/// Grouping these borrowed components keeps [`crate::aim::stability_for`] /
/// [`crate::aim::cone_for`] under clippy's argument-count gate (the
/// [`crate::resolve_coarse::ShotInputs`] bundle precedent), and states the
/// shooter's contribution to a shot as one value rather than several loose params.
/// Every field is a borrowed named ganger [`crate::ganger`] component (no bare
/// primitive); the bundle is a transparent borrow record, not itself a wrapped
/// domain scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shooter<'a> {
    /// The shooter's [`Stance`] — the §1a per-stance stability contribution and
    /// the brace gate's per-stance min-height band.
    pub stance:     &'a Stance,
    /// The shooter's [`Aiming`] flag — selects the §1a aim cone multiplier
    /// (aimed ×0.6 / hip-fired ×1).
    pub aiming:     &'a Aiming,
    /// The shooter's grid [`Position`] — the origin the faced cell is stepped from.
    pub position:   &'a Position,
    /// The shooter's [`Facing`] — the direction the faced cell is stepped along.
    pub facing:     &'a Facing,
    /// The shooter's optional [`Suppressed`] state (GTW-526) — `Some` when the shooter
    /// is pinned under opposing fire, `None` otherwise. When `Some`, the composer feeds
    /// the negated tunable suppression penalty into `stability` as a subtractive
    /// [`SuppressionStability`](crate::stability::SuppressionStability) term (a wider
    /// cone); when `None`, it passes the zero identity so the score is identical to
    /// a run without the term.
    pub suppressed: Option<&'a Suppressed>,
}
