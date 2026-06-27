//! The **authoring spec** — the `ThemeSpec` a `assets/content/themes/*.theme.ron`
//! deserializes into, plus the `TileKey` catalog-internal key (GTW-409). The terrain
//! mirror of [`TerrainSpec`](crate::terrain::piece::TerrainSpec), but a theme file
//! carries a WHOLE catalog (many named tiles + a declared theme + a default floor),
//! not one piece.

use bevy::{platform::collections::HashMap, prelude::Deref, reflect::TypePath};
use serde::Deserialize;

use super::{CatalogTile, LevelTheme};

/// A catalog-internal **tile key** — the stable identifier a [`ThemeSpec`] keys its
/// tiles by, and the key [`default_floor`](ThemeSpec::default_floor) names (GTW-409).
///
/// Distinct from [`TileDisplayName`](super::TileDisplayName) (the human label the editor
/// shows) and from [`TerrainName`](crate::terrain::piece::TerrainName) (a terrain-file
/// stem): a `TileKey` is the catalog's OWN lookup key (e.g. `"deck_floor"`), so the
/// `default_floor` can reference a tile by a stable id rather than its display string.
///
/// A key newtype over [`String`] (no-bare-types rule 1). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON string (so it works as a
/// `HashMap` key in authored RON and as the `default_floor` scalar).
///
/// Implements [`Default`] (empty-string sentinel, the
/// [`TerrainName`](crate::terrain::piece::TerrainName) precedent) so a
/// [`ThemeTileCatalog`](super::ThemeTileCatalog) can derive [`Default`] (an empty
/// catalog's default-floor key is the sentinel, which resolves to no tile).
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct TileKey(String);

impl TileKey {
    /// Build a tile key from its catalog-internal id string.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// The **authoring struct** a `assets/content/themes/*.theme.ron` deserializes into —
/// a whole per-theme tile catalog (GTW-409).
///
/// Carries the declared [`theme`](ThemeSpec::theme) (the [`LevelTheme`] this file's
/// catalog belongs to — the registry keys on it), the [`tiles`](ThemeSpec::tiles) map
/// (every named [`CatalogTile`] in the palette, keyed by [`TileKey`]), and the
/// [`default_floor`](ThemeSpec::default_floor) (the [`TileKey`] of the tile a generated
/// level fills empty ground with — C3). The terrain mirror of
/// [`TerrainSpec`](crate::terrain::piece::TerrainSpec), but theme-grained (a catalog of
/// many tiles, not one piece).
///
/// Every authored magnitude (each tile's stats / atlas index) is tuning DATA, NOT pinned
/// by tests (the brittle-test rule). Derives [`Deserialize`] so the `.ron` parses and
/// [`TypePath`] because `RonAsset<ThemeSpec>` requires its payload to be [`TypePath`]
/// (the same bound [`TerrainSpec`](crate::terrain::piece::TerrainSpec) satisfies).
///
/// **Not `Copy`** — it owns a `String`-keyed `HashMap`; it is `Clone` so the registry
/// can hold catalogs by value.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TypePath)]
pub struct ThemeSpec {
    /// The [`LevelTheme`] this catalog belongs to — the registry key.
    pub theme:         LevelTheme,
    /// The [`TileKey`] of the tile a generated level fills empty ground with (C3 — the
    /// theme's default floor). Must name a tile present in [`tiles`](ThemeSpec::tiles)
    /// (the loader verifies this consistency invariant).
    pub default_floor: TileKey,
    /// Every named tile in this theme's palette, keyed by its [`TileKey`].
    pub tiles:         HashMap<TileKey, CatalogTile>,
}
