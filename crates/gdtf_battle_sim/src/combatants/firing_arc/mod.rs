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
//! angle between the [`Direction::forward_step`](crate::ganger::Direction::forward_step)
//! facing vector and the cell delta, so a 120° arc admits any target within ±60° of the
//! facing — not just the three nearest compass headings. Pure function over a
//! [`Direction`](crate::ganger::Direction) + the two [`Cell`](crate::metric::Cell)s + a
//! [`FiringArc`](crate::tuning::FiringArc); no [`World`](bevy::ecs::world::World), no RNG —
//! unit-testable with bare values. **Zero pixels** — sim-unit ground-plane voxel
//! coordinates only.

mod arc;
#[cfg(test)]
mod test;

pub use arc::{TargetInArc, target_in_arc};
