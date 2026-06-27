//! The per-theme **tile catalog** + the theme-keyed **catalog registry** the folder
//! loader builds (GTW-409) — the theme mirror of
//! [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry), but two-level: a
//! registry of [`LevelTheme`] → [`ThemeTileCatalog`], each catalog a named-tile palette.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{CatalogTile, LevelTheme, TileKey, spec::ThemeSpec};

/// A resolved **per-theme tile catalog** — one theme's named-tile palette plus its
/// default floor (GTW-409).
///
/// Built from a [`ThemeSpec`] (the loaded `.theme.ron`), it answers the two palette /
/// procgen questions: [`default_floor`](ThemeTileCatalog::default_floor) resolves the
/// theme's default-floor tile (C3), and [`tiles`](ThemeTileCatalog::tiles) enumerates
/// every tile (each: display name + atlas index + gameplay stats) for the editor
/// palette (C4). Private inners with small accessors (the catalog answers a tile LOOKUP
/// / enumeration, not a raw-map question — the [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry)
/// no-`Deref` precedent). Holds tiles BY VALUE ([`CatalogTile`] is `Clone`), so they
/// survive the loaded-folder asset handle being dropped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThemeTileCatalog {
    /// The [`TileKey`] of the default-floor tile (the C3 resolve target).
    default_floor: TileKey,
    /// Every named tile in the palette, keyed by its [`TileKey`].
    tiles:         HashMap<TileKey, CatalogTile>,
}

impl ThemeTileCatalog {
    /// Build a catalog from a loaded [`ThemeSpec`] — moves its tiles + default floor in.
    #[must_use]
    pub fn from_spec(spec: ThemeSpec) -> Self {
        Self {
            default_floor: spec.default_floor,
            tiles:         spec.tiles,
        }
    }

    /// Resolve this theme's **default-floor tile** from the catalog (C3), or [`None`] if
    /// the authored `default_floor` key names no tile in the catalog (an inconsistent
    /// `.theme.ron`; the loader logs this and the consumer falls back).
    #[must_use]
    pub fn default_floor(&self) -> Option<&CatalogTile> {
        self.tiles.get(&self.default_floor)
    }

    /// The [`TileKey`] this catalog declares as its default floor — the key
    /// [`default_floor`](ThemeTileCatalog::default_floor) resolves against (exposed so
    /// the loader can assert the consistency invariant without re-resolving).
    #[must_use]
    pub const fn default_floor_key(&self) -> &TileKey {
        &self.default_floor
    }

    /// Look up one tile by its [`TileKey`], or [`None`] if absent.
    #[must_use]
    pub fn tile(&self, key: &TileKey) -> Option<&CatalogTile> {
        self.tiles.get(key)
    }

    /// An ENUMERABLE iterator over every `(key, tile)` in the palette (C4) — the editor
    /// lists each tile's display name + atlas index + gameplay stats from this.
    pub fn tiles(&self) -> impl Iterator<Item = (&TileKey, &CatalogTile)> {
        self.tiles.iter()
    }

    /// How many tiles the catalog holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    /// Whether the catalog holds no tiles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }
}

/// The **theme-keyed catalog registry** — a [`LevelTheme`] → [`ThemeTileCatalog`] map
/// the folder loader builds from `assets/content/themes/*.theme.ron` (GTW-409).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`LevelTheme`]`,
/// `[`ThemeTileCatalog`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`), the theme mirror of [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry).
/// The sim OWNS the theme catalog model, so the type lives here; the app's `Load` flow
/// POPULATES it from the loaded `assets/content/themes/*.theme.ron` folder (keyed by
/// each file's DECLARED [`LevelTheme`], not its filename — a theme's catalog is keyed by
/// the theme it belongs to) and inserts it as a PERSISTENT resource (not despawned on
/// state exit). It holds the catalogs BY VALUE, so they survive the loaded-folder asset
/// handle being dropped on `OnExit(Load)`.
///
/// Private inner with small accessors (a catalog LOOKUP / enumeration, not a raw-map
/// question — the [`TerrainRegistry`](crate::terrain::piece::TerrainRegistry) no-`Deref`
/// precedent). Consumers guard with `Option<Res<ThemeCatalogRegistry>>` /
/// `run_if(resource_exists::<…>)` (bevy-traps #1).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeCatalogRegistry(HashMap<LevelTheme, ThemeTileCatalog>);

impl ThemeCatalogRegistry {
    /// Build a catalog registry from a `(theme, catalog)` iterator — the shape the
    /// folder loader (and tests) key by each file's declared [`LevelTheme`].
    #[must_use]
    pub fn new(catalogs: impl IntoIterator<Item = (LevelTheme, ThemeTileCatalog)>) -> Self {
        Self(catalogs.into_iter().collect())
    }

    /// Insert one theme's catalog under its [`LevelTheme`] key, returning the previous
    /// catalog at that key (if any) — the per-file insert the folder loader calls.
    pub fn insert(
        &mut self,
        theme: LevelTheme,
        catalog: ThemeTileCatalog,
    ) -> Option<ThemeTileCatalog> {
        self.0.insert(theme, catalog)
    }

    /// Look up the [`ThemeTileCatalog`] for a [`LevelTheme`], or [`None`] if no catalog
    /// file declared that theme — the editor / procgen entry point.
    #[must_use]
    pub fn catalog(&self, theme: LevelTheme) -> Option<&ThemeTileCatalog> {
        self.0.get(&theme)
    }

    /// An ENUMERABLE iterator over every `(theme, catalog)` the registry holds — so the
    /// editor can list the themes it can offer.
    pub fn catalogs(&self) -> impl Iterator<Item = (&LevelTheme, &ThemeTileCatalog)> {
        self.0.iter()
    }

    /// How many theme catalogs the registry holds — the count the folder-load test
    /// asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no theme catalogs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
