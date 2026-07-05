//! The **terrain-brace gate** — [`terrain_braces`], [`TerrainBraced`], and
//! [`cell_above`](gate::cell_above) (GTW-392, `docs/combat/resolution.md` §1a terrain-brace clause).
//!
//! A kneeling occupant on the LOWER endpoint of an authored stair cell earns the
//! stair-brace bonus when the slab directly overhead is
//! [`SlabState::Present`](crate::surface::SlabState::Present). [`terrain_braces`]
//! is a **pure function** over the live grids already in scope at the fire call
//! site — no maintained per-shooter marker, no reactor, stale-free revocation.
//! The [`TerrainBraced`] output feeds the same `brace_engages` OR-clause
//! (`stability::gate::brace_engages`) as the weapon's [`crate::weapon::Stable`] tag,
//! granting the IDENTICAL brace contribution — one `tuning.brace_contribution` quantum, no
//! duplicate stat.

mod gate;

#[cfg(test)]
mod test;

pub use gate::{TerrainBraced, terrain_braces};
