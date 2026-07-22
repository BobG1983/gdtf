//! The **prefab schema** — the UUID-keyed level-fragment `.ron` format introduced by
//! the GTW-476 data-model refactor (child T04) and the SOLE prefab model after GTW-496
//! retired the legacy filename-stem-keyed prefab types.
//!
//! The schema:
//!
//! - references terrain by the stable [`TerrainUuid`](crate::terrain::def::TerrainUuid) key
//!   (the unified terrain model, GTW-484) and the theme by the stable
//!   [`ThemeUuid`](super::ThemeUuid) (GTW-485);
//! - collapses all authored geometry into ONE
//!   [`placements`](spec::PrefabSpec::placements) list of placed-UUID entries
//!   (each a [`TerrainPlacementEntry`]) — the per-piece sim/presenter behaviour lives in
//!   the referenced [`TerrainDef`](crate::terrain::def::TerrainDef), so a placement carries
//!   only *which* piece goes *where*;
//! - carries NO authored-opening field and NO per-prefab validation path — inter-fragment
//!   connectivity is by-construction in the assembler (the 1-cell `default_floor` every
//!   placement reserves; the old per-prefab opening machinery was removed in GTW-497), not
//!   authored per-prefab and validated fail-closed.
//!
//! GTW-486 added the SPEC TYPES ([`PrefabSpec`] / [`TerrainPlacementEntry`]); GTW-488
//! (child T05b) added the re-keyed REGISTRY ([`PrefabRegistry`] / [`PrefabKey`] /
//! [`Prefab`]) — keyed by the stable [`ThemeUuid`](super::ThemeUuid). GTW-557 dropped
//! the now-meaningless `_v2`/`V2`/`2` suffixes (the legacy v1 model was fully deleted in
//! GTW-494/496).
//!
//! The **shared prefab schema vocabulary** — [`SpawnRole`] and [`PrefabName`] —
//! lives in the sibling `vocab.rs` (previously merged into this file by GTW-557).
//!
//! Mirrors the sibling dir-module layout (memory: *code-health-module-layout*): this
//! `mod.rs` is wiring-only; per-concern files carry the types; `test` houses the unit tests.

mod placement;
mod registry;
mod spec;
mod vocab;

#[cfg(test)]
mod test;

pub use placement::TerrainPlacementEntry;
pub use registry::{Prefab, PrefabKey, PrefabRegistry};
pub use spec::PrefabSpec;
pub use vocab::{PrefabName, SpawnRole};
