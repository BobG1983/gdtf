//! The **unified terrain-definition model** (GTW-484, child T02 of the GTW-476
//! data-model refactor) — the NEW UUID-keyed terrain definition that will replace
//! the legacy `TerrainSpec`/`CatalogTile` pair.
//!
//! Per the GTW-476 redesign (*terrain-data-model-redesign-2026-06-28*): one terrain
//! definition is keyed by a stable [`TerrainUuid`] and splits into a SIM half
//! ([`TerrainSimKind`] — the HP/armor/band/structural stats the combat path reads,
//! plus the SIM-OWNED [`TerrainTag`]s that drive Pathing/FoV/LoS) and a
//! PRESENTER half ([`TerrainPresenterKind`] — strictly the graphic role key and an
//! optional slab footfall). The one-way sim→presenter dependency means the presenter
//! structurally cannot read the sim-owned tags.
//!
//! This module is **purely additive** (GTW-484): it introduces the new model ALONGSIDE
//! the legacy [`piece`](super::piece) types — it deletes nothing, touches no consumer,
//! wires no loader, and migrates no content. The new types are exercised only by their
//! own unit tests and the registry here. Tag CONSUMPTION (the actual `Pathing`/`FoV`/`LoS`
//! effects + change-detection) is GTW-482, NOT here — this slice only makes the
//! sim-owned tag field EXIST, round-trip, and be read-reachable from the sim crate.
//!
//! Mirrors the [`piece`](super::piece) dir-module layout (memory:
//! *code-health-module-layout*): `mod.rs` is wiring-only; per-concern files carry the
//! types; `test/` houses the unit tests.

mod definition;
mod kind;
mod registry;
mod uuid;

#[cfg(test)]
mod test;

pub use definition::{TerrainDef, TerrainDisplayName};
pub use kind::{TerrainPresenterKind, TerrainSimKind, TerrainTag};
pub use registry::TerrainDefRegistry;
pub use uuid::TerrainUuid;
pub(crate) use uuid::fnv1a64_u128;
