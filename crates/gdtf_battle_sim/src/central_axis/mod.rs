//! The §1 **central axis** — the muzzle point, the target aim point, and the
//! recoil-climb-tilted firing axis, all in **sim units / level-fractions**
//! (`docs/combat/resolution.md` §1 "muzzle/aim-point paragraph" + "What's pure
//! math vs sim"; `docs/combat/battle-space.md` §"Stance / cover / muzzle / aim
//! heights" + §"Sub-cell precision on the ground plane").
//!
//! Aiming centres a dispersion cone whose central axis runs from the **3D muzzle
//! point** to the **aim point** (resolution.md §1). This module derives those two
//! points and the recoil-climb tilt of the axis between them — the geometry the
//! cone sample (E2.5) and the coarse march (§2) build on. Everything here is in the
//! cubic-voxel metric ([`crate::metric::SimPos`], one cell = one level = 1.0 sim
//! unit): vertical datums are **level-fractions**, sub-cell offsets are
//! **cell-fractions**, and "up" is `+z` ([`bevy::math::Vec3::Z`]). **Zero pixels.**
//!
//! Three derivations, each reading its magnitudes from tuning / E1 data — nothing
//! hardcoded:
//!
//! 1. [`muzzle_position`] — the shooter's [`crate::metric::cell_center`] plus the
//!    per-facing forward offset ([`crate::tuning::MuzzleForwardOffset`], a
//!    cell-fraction along [`crate::ganger::Direction::forward_step`]), **clamped** so
//!    the muzzle never leaves the shooter's cell; `z = level as f32 + the per-stance
//!    muzzle level-fraction` ([`crate::tuning::MuzzleHeights`]).
//! 2. [`target_aim_point`] — the target's [`crate::metric::cell_center`] with a z
//!    derived per-case: a **ganger** target aims at its per-stance silhouette-top
//!    level-fraction (the dedicated [`crate::tuning::SilhouetteTops`] tuning) ×
//!    [`crate::tuning::AimHeightFrac`]; a **cover-occupied** cell aims at the cover
//!    [`crate::cover::HeightBand`]'s midpoint level-fraction (derived from the
//!    [`crate::tuning::ProjectileBandEdges`] band edges, never a literal — cover
//!    height genuinely IS band-based).
//! 3. [`climb_aim_dir`] — the unit muzzle→aim axis, tilted **up** (`+z`) by
//!    `prior_shots × recoil_climb × recoil_growth` radians (resolution.md §1a recoil
//!    climb); zero prior shots yields the untilted axis exactly. Returns the named
//!    unit-direction newtype [`AimDir`].

mod aim_dir;
mod aim_point;
mod muzzle;

#[cfg(test)]
mod test;

pub use aim_dir::{AimDir, climb_aim_dir};
pub use aim_point::target_aim_point;
pub use muzzle::muzzle_position;
