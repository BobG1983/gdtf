//! The **level** model — the foundational typed data the map-editor palette and the
//! procgen assembly both read (GTW-409).
//!
//! - `theme` — the [`LevelTheme`] closed set (GTW-414 adds it to
//!   [`Situation`](crate::situation::Situation)) and the [`GridSize`] dimension
//!   newtypes (each axis a named, validated, private-inner type, bounded by
//!   [`MAX_GRID_SPAN`] / [`MAX_LEVELS`](crate::metric::MAX_LEVELS)).
//! - `tile` — one [`CatalogTile`] (display name + opaque [`TileAtlasIndex`] + the
//!   gameplay stats the editor shows), render-free.
//! - `spec` — the [`ThemeSpec`] a `assets/content/themes/*.theme.ron`
//!   deserializes into (a whole per-theme catalog) + the [`TileKey`] catalog key.
//! - `registry` — the per-theme [`ThemeTileCatalog`] (default-floor resolve + the
//!   enumerable tile list) and the theme-keyed [`ThemeCatalogRegistry`] resource the
//!   app's `Load` flow populates from the loaded themes folder.
//! - `theme_def` — the NEW UUID-keyed [`UuidThemeDef`] (keyed by a stable [`ThemeUuid`],
//!   referencing the unified terrain model by `TerrainUuid`) and its [`UuidThemeRegistry`]
//!   resource (GTW-485). Purely ADDITIVE: it lives alongside the closed [`LevelTheme`] /
//!   [`ThemeSpec`] / [`ThemeCatalogRegistry`], wiring no loader and touching no consumer.
//! - `prefab` — the canonical level-fragment [`PrefabSpec`] a
//!   `assets/content/maps/<theme>/<size>/*.ron` deserializes into (read by both the
//!   GTW-424 assembler and the GTW-432 editor), the [`SpawnRole`]/[`EdgeOpening`] schema
//!   fields, and the per-`(theme, size, role)` [`PrefabRegistry`] resource the game `Load`
//!   flow populates from the loaded maps folder (GTW-418).
//! - `prefab_v2` — the NEW UUID-keyed level-fragment [`PrefabSpecV2`] (theme by
//!   [`ThemeUuid`], every placed piece by `TerrainUuid` in ONE
//!   [`placements`](PrefabSpecV2::placements) list, no `edge_openings` / validation) + its
//!   [`TerrainPlacementEntry`] (GTW-486), and the re-keyed [`PrefabRegistry2`] of
//!   [`Prefab2`]s bucketed by the stable [`ThemeUuid`]-keyed [`PrefabKey2`] (GTW-488 — no
//!   edge-opening validation; an openingless prefab is valid). Purely ADDITIVE: it lives
//!   alongside the legacy [`PrefabSpec`] / [`PrefabKey`] / [`PrefabRegistry`], wiring no
//!   loader / assembler.
//!
//! Mirrors the `terrain/piece` dir-module layout (memory: *code-health-module-layout*):
//! `mod.rs` is wiring-only; per-concern files carry the types; `test/` houses the unit
//! tests.

mod prefab;
mod prefab_v2;
mod registry;
mod spec;
mod theme;
mod theme_def;
mod tile;

#[cfg(test)]
mod test;

pub use prefab::{
    EdgeOpening, Prefab, PrefabKey, PrefabLoadError, PrefabName, PrefabPiece, PrefabRegistry,
    PrefabSpec, SpawnRole,
};
pub use prefab_v2::{Prefab2, PrefabKey2, PrefabRegistry2, PrefabSpecV2, TerrainPlacementEntry};
pub use registry::{ThemeCatalogRegistry, ThemeTileCatalog};
pub use spec::{ThemeSpec, TileKey};
pub use theme::{
    GridHeight, GridLevels, GridSize, GridSizeError, GridWidth, LevelTheme, MAX_GRID_SPAN,
};
pub use theme_def::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry};
pub use tile::{CatalogTile, CatalogTileKind, StructuralStats, TileAtlasIndex, TileDisplayName};
