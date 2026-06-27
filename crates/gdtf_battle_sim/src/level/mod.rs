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
//!
//! Mirrors the `terrain/piece` dir-module layout (memory: *code-health-module-layout*):
//! `mod.rs` is wiring-only; per-concern files carry the types; `test/` houses the unit
//! tests.

mod registry;
mod spec;
mod theme;
mod tile;

#[cfg(test)]
mod test;

pub use registry::{ThemeCatalogRegistry, ThemeTileCatalog};
pub use spec::{ThemeSpec, TileKey};
pub use theme::{
    GridHeight, GridLevels, GridSize, GridSizeError, GridWidth, LevelTheme, MAX_GRID_SPAN,
};
pub use tile::{CatalogTile, CatalogTileKind, StructuralStats, TileAtlasIndex, TileDisplayName};
