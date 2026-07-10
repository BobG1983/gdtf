//! The **level** model — the foundational typed data the map-editor palette and the
//! procgen assembly both read (GTW-409).
//!
//! - `theme` — the [`GridSize`] dimension newtypes (each axis a named, validated,
//!   private-inner type, bounded by [`MAX_GRID_SPAN`] / [`MAX_LEVELS`](crate::metric::MAX_LEVELS)).
//! - `theme_def` — the UUID-keyed [`UuidThemeDef`] (keyed by a stable [`ThemeUuid`],
//!   referencing the unified terrain model by `TerrainUuid`) and its [`UuidThemeRegistry`]
//!   resource (GTW-485) — the sole theme model after the GTW-496 deletion of the legacy
//!   closed-enum theme types.
//! - `prefab` — UUID-keyed level-fragment types: [`PrefabSpec`] (ONE `placements` list of
//!   [`TerrainPlacementEntry`]), [`PrefabRegistry`] of [`Prefab`]s keyed by
//!   [`PrefabKey`] (`(ThemeUuid, GridSize, SpawnRole)`) (GTW-486/488), and the shared vocab:
//!   [`SpawnRole`] + [`PrefabName`]. Sole model after GTW-496; openings removed in GTW-497
//!   (connectivity is by-construction via the 1-cell `default_floor` seam).
//!
//! Mirrors the `terrain/piece` dir-module layout (memory: *code-health-module-layout*):
//! `mod.rs` is wiring-only; per-concern files carry the types; `test/` houses the unit
//! tests.

mod prefab;
mod theme;
mod theme_def;

#[cfg(test)]
mod test;

pub use prefab::{
    Prefab, PrefabKey, PrefabName, PrefabRegistry, PrefabSpec, SpawnRole, TerrainPlacementEntry,
};
pub use theme::{GridHeight, GridLevels, GridSize, GridSizeError, GridWidth, MAX_GRID_SPAN};
pub use theme_def::{ThemeDisplayName, ThemeName, ThemeUuid, UuidThemeDef, UuidThemeRegistry};
