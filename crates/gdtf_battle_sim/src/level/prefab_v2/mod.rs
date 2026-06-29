//! The **v2 prefab schema** — the UUID-keyed level-fragment `.ron` format introduced by
//! the GTW-476 data-model refactor (child T04), living ALONGSIDE the legacy
//! [`PrefabSpec`](super::PrefabSpec) rather than replacing it.
//!
//! Where the legacy [`PrefabSpec`](super::PrefabSpec) references terrain by the filename-stem
//! [`TerrainName`](crate::terrain::piece::TerrainName) and splits authored geometry across
//! four separate lists (walls / scatter / slabs / floors) plus an explicit
//! `edge_openings` connectivity list, the v2 schema:
//!
//! - references terrain by the stable [`TerrainUuid`](crate::terrain::def::TerrainUuid) key
//!   (the unified terrain model, GTW-484) and the theme by the stable
//!   [`ThemeUuid`](super::ThemeUuid) (GTW-485);
//! - collapses the four split geometry lists into ONE
//!   [`placements`](spec::PrefabSpecV2::placements) list of placed-UUID entries
//!   (each a [`TerrainPlacementEntry`]) — the per-piece sim/presenter behaviour now lives in
//!   the referenced [`TerrainDef`](crate::terrain::def::TerrainDef), so a placement carries
//!   only *which* piece goes *where*;
//! - carries NO `edge_openings` field and NO validation path — inter-fragment connectivity
//!   is by-construction in the v2 assembler (a later child), not authored per-prefab and
//!   validated fail-closed (the GTW-473 walls/scatter merge + serde-default-`Fill` direction).
//!
//! This module is PURELY ADDITIVE (GTW-486): it adds the SPEC TYPES ONLY. The v2 registry
//! / key (T05b), the v2 loader (T05c), and any assembler change are out of scope here — the
//! legacy [`PrefabSpec`](super::PrefabSpec) / [`PrefabRegistry`](super::PrefabRegistry) /
//! [`PrefabKey`](super::PrefabKey) / loader / assembler stay live and untouched.
//!
//! Mirrors the sibling dir-module layout (memory: *code-health-module-layout*): this
//! `mod.rs` is wiring-only; per-concern files carry the types; `test` houses the unit tests.

mod placement;
mod spec;

#[cfg(test)]
mod test;

pub use placement::TerrainPlacementEntry;
pub use spec::PrefabSpecV2;
