//! The **unified theme-definition model** (GTW-485, child T03 of the GTW-476
//! data-model refactor) — the UUID-keyed theme definition that is the SOLE theme model
//! after GTW-496 retired the closed-enum theme types.
//!
//! Per the GTW-476 redesign (*terrain-data-model-redesign-2026-06-28*): a theme is keyed
//! by a stable [`ThemeUuid`] and references the unified terrain model by
//! [`TerrainUuid`](crate::terrain::def::TerrainUuid) — its [`default_floor`](UuidThemeDef::default_floor)
//! is the terrain that supplies the per-theme floor (its move cost comes from the
//! referenced [`TerrainDef`](crate::terrain::def::TerrainDef) — no `Floor` kind), and its
//! [`terrain`](UuidThemeDef::terrain) is the palette of terrain UUIDs the theme draws from.
//!
//! It reuses [`TerrainUuid`](crate::terrain::def::TerrainUuid) from the GTW-484 terrain
//! model (imported, not redeclared); [`ThemeUuid`] is a DISTINCT key.
//!
//! Mirrors the [`terrain::def`](crate::terrain::def) dir-module layout (memory:
//! *code-health-module-layout*): `mod.rs` is wiring-only; per-concern files carry the
//! types; `test/` houses the unit tests.

mod definition;
mod registry;
mod uuid;

#[cfg(test)]
mod test;

pub use definition::{ThemeDisplayName, UuidThemeDef};
pub use registry::UuidThemeRegistry;
pub use uuid::ThemeUuid;
