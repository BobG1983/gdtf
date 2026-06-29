//! The **level** model — the foundational typed data the map-editor palette and the
//! procgen assembly both read (GTW-409).
//!
//! - `theme` — the [`GridSize`] dimension newtypes (each axis a named, validated,
//!   private-inner type, bounded by [`MAX_GRID_SPAN`] / [`MAX_LEVELS`](crate::metric::MAX_LEVELS)).
//! - `theme_def` — the UUID-keyed [`UuidThemeDef`] (keyed by a stable [`ThemeUuid`],
//!   referencing the unified terrain model by `TerrainUuid`) and its [`UuidThemeRegistry`]
//!   resource (GTW-485) — the sole theme model after the GTW-496 deletion of the legacy
//!   closed-enum theme types.
//! - `prefab` — the shared prefab schema vocabulary still referenced by `prefab_v2`: the
//!   [`SpawnRole`] deployment role and the [`PrefabName`] key. (The edge-opening
//!   connectivity types were removed in GTW-497 — connectivity is by-construction via the
//!   1-cell `default_floor` seam, no authored per-prefab openings.)
//! - `prefab_v2` — the UUID-keyed level-fragment [`PrefabSpecV2`] (theme by
//!   [`ThemeUuid`], every placed piece by `TerrainUuid` in ONE
//!   [`placements`](PrefabSpecV2::placements) list) + its [`TerrainPlacementEntry`]
//!   (GTW-486), and the re-keyed [`PrefabRegistry2`] of [`Prefab2`]s bucketed by the
//!   stable [`ThemeUuid`]-keyed [`PrefabKey2`] (GTW-488 — an openingless prefab is valid).
//!   The sole prefab model after the GTW-496 deletion of the legacy `TerrainName`-keyed
//!   prefab types.
//!
//! Mirrors the `terrain/piece` dir-module layout (memory: *code-health-module-layout*):
//! `mod.rs` is wiring-only; per-concern files carry the types; `test/` houses the unit
//! tests.

mod prefab;
mod prefab_v2;
mod theme;
mod theme_def;

#[cfg(test)]
mod test;

pub use prefab::{PrefabName, SpawnRole};
pub use prefab_v2::{Prefab2, PrefabKey2, PrefabRegistry2, PrefabSpecV2, TerrainPlacementEntry};
pub use theme::{GridHeight, GridLevels, GridSize, GridSizeError, GridWidth, MAX_GRID_SPAN};
pub use theme_def::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry};
