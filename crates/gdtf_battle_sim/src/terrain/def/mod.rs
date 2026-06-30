//! The **unified terrain-definition model** (GTW-484, child T02 of the GTW-476
//! data-model refactor) — the UUID-keyed terrain definition that is the SOLE terrain
//! model after GTW-496 retired the legacy per-file terrain authoring types.
//!
//! Per the GTW-476 redesign (*terrain-data-model-redesign-2026-06-28*): one terrain
//! definition is keyed by a stable [`TerrainUuid`] and splits into a SIM half
//! ([`TerrainSimKind`] — the HP/armor/band/structural stats the combat path reads,
//! plus the SIM-OWNED [`TerrainTag`]s that drive Pathing/FoV/LoS) and a
//! PRESENTER half ([`TerrainPresenterKind`] — strictly the graphic role key and an
//! optional slab footfall). The one-way sim→presenter dependency means the presenter
//! structurally cannot read the sim-owned tags.
//!
//! It reuses the shared identity newtypes from [`piece`](super::piece)
//! ([`TerrainName`](super::piece::TerrainName) / [`TerrainGraphicKey`](super::piece::TerrainGraphicKey)
//! / [`FootfallSound`](super::piece::FootfallSound)). Tag CONSUMPTION (the actual
//! `Pathing`/`FoV`/`LoS` effects + change-detection) is GTW-482.
//!
//! Mirrors the [`piece`](super::piece) dir-module layout (memory:
//! *code-health-module-layout*): `mod.rs` is wiring-only; per-concern files carry the
//! types; `test/` houses the unit tests.

mod blocking;
mod definition;
mod kind;
mod registry;
mod uuid;

#[cfg(test)]
mod test;

pub use blocking::{derives_path_blocking, sim_kind_blocks_path};
pub use definition::{TerrainDef, TerrainDisplayName};
pub use kind::{TerrainPresenterKind, TerrainSimKind, TerrainTag};
pub use registry::TerrainDefRegistry;
pub use uuid::TerrainUuid;
pub(crate) use uuid::fnv1a64_u128;
