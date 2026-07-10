//! Persistent surface grid: the model's authoritative store of floor/roof **slab**
//! existence and per-cell **ground** damage — battle *state* carried across every
//! occupancy rebuild. The **persistent surface grid**: floor/roof slabs + ground
//! records are battle *state*, carried across every rebuild, so a destroyed slab
//! stays destroyed and ground damage accrues (a model-authoritative store in the
//! model/view split — ADR-0001, `docs/decisions/0001-rust-bevy-rewrite.md`).
//!
//! This is the E1.5 surface-grid slice. It is a **separate** resource from the
//! coarse occupancy (E1.6 / GTW-156, not built yet): the surface grid carries slab
//! *existence* and ground *damage* ONLY — occupant presence is the occupancy grid's
//! job and is out of scope here. The occupancy grid is **rebuilt fresh per shot**;
//! the surface grid is **mutated in place** and survives that rebuild, which is the
//! whole reason it is a distinct, persistent store.
//!
//! Two persistence invariants, each modelled so the type *cannot* violate it:
//!
//! 1. **A destroyed slab stays destroyed.** [`SlabState`] is `Present` / `Destroyed`
//!    / `Absent`; [`SurfaceGrid::destroy_slab`] sets `Destroyed` and there is **no
//!    API that reverts it** — [`SurfaceGrid::set_slab`] refuses to overwrite a
//!    `Destroyed` entry (a destroyed slab stays destroyed; slab destroyed at zero).
//!    Destruction is permanent by construction.
//! 2. **Ground is damaged, never destroyed, and damage only accrues.**
//!    [`GroundDamage`] is a `u32` accumulator; [`SurfaceGrid::accrue_ground_damage`]
//!    is **additive (saturating)** so the total is monotonically non-decreasing —
//!    there is no API that lowers it, and the guarded [`SurfaceGrid::set_ground_damage`]
//!    rejects any value below the current total (the ground-hit verb: damaged,
//!    never destroyed).
//!
//! The slab key is the E1.1 [`CellLevel`](crate::metric::CellLevel) (the
//! `(cell, level)` of the slab between storeys); the ground key is the E1.1
//! [`Cell`](crate::metric::Cell) (the ground plane has no storey). Both maps are
//! lazily populated: an absent slab key reads as [`SlabState::Absent`] (no slab
//! authored there) and an absent ground key reads as zero damage.

mod grid;

#[cfg(test)]
mod test;

pub use grid::{DamageApplied, GroundDamage, SlabState, SurfaceGrid};
