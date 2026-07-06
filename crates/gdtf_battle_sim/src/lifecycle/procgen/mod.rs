//! The **space-packing assembler core** — the procgen packer that places deployment
//! prefabs into a battlefield (GTW-424, the FIRST half: anchor selection + opposite-side
//! enemy placement).
//!
//! This module is render-free and deterministic (the *behavioral* render-free constraint —
//! it lives here, beside [`ProcgenRng`](crate::rng::ProcgenRng) and
//! [`Situation`](crate::situation::Situation), because it is sim MODEL logic). It is the
//! UNIT-TESTED packer core of the staged assembler:
//!
//! ```text
//! GTW-424 placement  ->  GTW-427 fill  ->  GTW-431 emit/trigger
//! ```
//!
//! GTW-424 covered the FIRST stage (anchor + opposite placement); GTW-427 added the SECOND
//! (`fill` — random same-theme fill + the no-fit pad-`default_floor` fallback + the OQ-6
//! [`ProcgenTuning`] knobs); GTW-431 adds the THIRD (`emit` — the deterministic seed
//! harness [`generate_level`] + [`emit_level`], which pours a [`FilledPlacement`] into the
//! sim's canonical [`Situation`](crate::situation::Situation) as the terrain entries it
//! holds inline, connectivity by-construction via the seam lattice). A tested core with no
//! live trigger yet is the intended staged build, NOT a dead-feature split. NOTHING here
//! wires a live battle request (the loading-state driver is GTW-433; the debug visualizer
//! is GTW-434).
//!
//! # The locked design (the GTW-424 rulings + authorized defaults — see the concern docs)
//!
//! - **OQ-7 (packer):** [`MaxRectsPacker`] is the shipped, denser packer; a
//!   [`SplitMode::Guillotine`] alternative is INCLUDED behind a flag for A/B comparison.
//! - **OQ-3 (seam):** a 1-cell [`Margin::DEFAULT`] `default_floor` seam is reserved around
//!   every placed prefab — NO abutting prefabs.
//! - **OQ-4 (connectivity):** connectivity is BY CONSTRUCTION — the 1-cell `default_floor`
//!   seam every placement reserves leaves a walkable corridor lattice around every placed
//!   region, so every open board cell is reachable. GTW-497 removed the old fail-closed
//!   connectivity flood / rejection (and the per-prefab opening machinery): there is nothing
//!   to assert or repair — the seam guarantees it structurally.
//! - **OQ-2 (opposite):** the enemy anchor is the STRICT geometric [`Anchor::opposite`] of
//!   the player anchor, with ZERO RNG draw (fairness is structural). The PLAYER anchor is
//!   the one RNG choice ([`Anchor::choose`], from [`ProcgenRng`](crate::rng::ProcgenRng)).
//! - **OQ-5 (size + cap):** a `~10x10` minimum player-spawn footprint ([`MinPlayerSide`]);
//!   the player footprint is capped so the opposite enemy region always fits (enforced via
//!   the packer fit check + the preferred GTW-418 load-time rejection).
//!
//! Mirrors the `level` / `situation` dir-module layout (memory: *code-health-module-layout*):
//! `mod.rs` is wiring-only; per-concern files carry the types; `test/` houses the unit
//! tests.

mod anchor;
mod assembler;
mod emit;
mod error;
mod fill;
mod findings;
mod geometry;
mod packer;
mod tuning;

#[cfg(test)]
mod test;

pub use anchor::Anchor;
pub use assembler::{PlacedPrefab, Placement, assemble_placement, assemble_placement_with};
pub use emit::{emit_level, generate_level};
pub use error::PackingError;
pub use fill::{FilledPlacement, fill_placement, fill_placement_with};
pub use findings::{EmittedLevel, ProcgenFinding};
pub use geometry::{Footprint, Margin, MinPlayerSide, RegionRect};
pub use packer::{MaxRectsPacker, SplitMode};
pub use tuning::{DeadRectScatterCount, LargePrefabAreaThreshold, MinDensityFloor, ProcgenTuning};
