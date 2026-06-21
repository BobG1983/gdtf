//! The level-aware coarse-geometry **sight probe** — [`has_los`], its borrow-view
//! inputs ([`Observer`] / [`Target`]), and the [`Sighted`] verdict (GTW-337, leaf 1
//! of the GTW-13 FOV epic).
//!
//! `has_los` answers "can this watcher SEE that ganger along the cubic-voxel grid?"
//! by WRAPPING the one geometry truth — [`march_vector`](crate::march::march_vector),
//! the 3-axis voxel DDA — and REUSING the shot pipeline's z-anchoring. It is NOT a
//! second LOS implementation: the probe builds an eye anchor and an aim anchor, marches
//! the ray between them once, and reads the verdict off where the march stopped
//! (`docs/combat/resolution.md` §2 clearance). Pure: render-free, RNG-free,
//! deterministic, no `&mut World` / `Commands`.
//!
//! Two resolved fidelity decisions live in the anchoring (see [`has_los`] for the full
//! WHY):
//!
//! 1. The **eye is facing-neutral** — `cell_center + per-stance muzzle level-fraction`
//!    z, WITHOUT [`muzzle_position`](crate::central_axis::muzzle_position)'s per-facing
//!    forward XY offset. FOV is omni-directional: a watcher facing away must still SEE
//!    for the squad sight union, so the eye must not move with facing; only the ray is
//!    directional.
//! 2. The **aim mirrors the shot pipeline EXACTLY** — the
//!    `cover.peek().or_else(occupant_band)` band derivation of
//!    [`TargetGeometry::compose`](crate::fire) fed into
//!    [`target_aim_point`](crate::central_axis::target_aim_point), NOT a bare cover
//!    peek (which would aim at a different z for a no-cover banded ganger).
//!
//! Asymmetric sight (a low watcher sees a tall target, but not the reverse) falls out
//! of anchoring eye-vs-aim and folds in HERE — it needs no extra rule.

mod probe;

#[cfg(test)]
mod test;

pub use probe::{Observer, Sighted, Target, has_los};
