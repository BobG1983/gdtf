//! The terrain **piece** model — the `TerrainSpec` authoring struct + the
//! `TerrainRegistry` resource the folder loader builds (GTW-394).
//!
//! A "terrain piece" is an authored `.terrain.ron` file: one of the five C1
//! piece kinds (FLOOR / WALL / COVER / SCATTER / SLAB), carrying the stats
//! the combat path reads (HP / armor / height band / move cost) plus the
//! presentation hooks (graphic key + footfall sound key) the presenter resolves.
//!
//! Mirrors the `equipment/weapon` dir-module layout (memory:
//! *code-health-module-layout*): `mod.rs` is wiring-only; per-concern files carry
//! the types; `test/` houses the unit tests.
//!
//! The registry is **dormant after GTW-394** — nothing consumes it yet (binding a
//! generated cell's `TerrainName` → its spec is a downstream epic ticket).

mod components;
mod registry;
mod spec;

#[cfg(test)]
mod test;

pub use components::{FootfallSound, TerrainGraphicKey, TerrainName};
pub use registry::TerrainRegistry;
pub use spec::{FloorSpec, SlabPieceSpec, StructuralSpec, TerrainKindSpec, TerrainSpec};
