//! The **unified theme-definition registry** — the [`ThemeUuid`]→[`UuidThemeDef`] map
//! (GTW-485), the UUID-keyed successor to the legacy
//! [`ThemeCatalogRegistry`](crate::level::ThemeCatalogRegistry).

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{ThemeUuid, UuidThemeDef};
use crate::terrain::def::TerrainUuid;

/// The **unified theme-definition registry** — a [`ThemeUuid`]→[`UuidThemeDef`] map
/// (GTW-485), mirroring the legacy
/// [`ThemeCatalogRegistry`](crate::level::ThemeCatalogRegistry) shape but keyed by the
/// stable UUID instead of the closed [`LevelTheme`](crate::level::LevelTheme) enum.
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`ThemeUuid`]`, `[`UuidThemeDef`]`>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), the theme mirror of
/// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry). The sim OWNS the theme
/// model, so the type lives here. It holds the definitions BY VALUE ([`UuidThemeDef`] is
/// `Clone`), so they survive even if the source asset handle is dropped.
///
/// Private inner with small accessors (a registry answers a theme LOOKUP / resolve /
/// enumeration, not a raw-map question — so no derived [`Deref`](bevy::prelude::Deref), the
/// [`ThemeCatalogRegistry`](crate::level::ThemeCatalogRegistry) /
/// [`TerrainDefRegistry`](crate::terrain::def::TerrainDefRegistry) no-`Deref` precedent).
///
/// **Purely additive (GTW-485)** — nothing consumes this registry yet; no loader populates
/// it. It is exercised only by this ticket's unit tests. Wiring a loader and binding
/// consumers are downstream slices of the GTW-476 refactor.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct UuidThemeRegistry(HashMap<ThemeUuid, UuidThemeDef>);

impl UuidThemeRegistry {
    /// Build a registry from a `(key, def)` iterator — the shape a loader (and tests) key
    /// by [`ThemeUuid`].
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (ThemeUuid, UuidThemeDef)>) -> Self {
        Self(defs.into_iter().collect())
    }

    /// Insert one definition under its [`ThemeUuid`] key, returning the previous definition
    /// at that key (if any) — the per-definition insert a loader calls.
    pub fn insert(&mut self, key: ThemeUuid, def: UuidThemeDef) -> Option<UuidThemeDef> {
        self.0.insert(key, def)
    }

    /// Look up the [`UuidThemeDef`] for a [`ThemeUuid`] key, or [`None`] if no definition
    /// with that key is registered — the resolution a consumer reads.
    #[must_use]
    pub fn def(&self, key: &ThemeUuid) -> Option<&UuidThemeDef> {
        self.0.get(key)
    }

    /// RESOLVE a theme's **default-floor** [`TerrainUuid`] — the terrain definition the
    /// theme nominates as its ground — or [`None`] if no theme with that key is registered.
    /// The referenced terrain's `MoveCost` comes from the
    /// [`TerrainDef`](crate::terrain::def::TerrainDef) it resolves to (there is no `Floor` kind).
    #[must_use]
    pub fn default_floor(&self, key: &ThemeUuid) -> Option<TerrainUuid> {
        self.0.get(key).map(|def| def.default_floor)
    }

    /// ENUMERATE a theme's **terrain palette** — its list of [`TerrainUuid`]s — or [`None`]
    /// if no theme with that key is registered. The slice the editor / procgen iterates to
    /// know which terrain a theme draws from.
    #[must_use]
    pub fn terrain(&self, key: &ThemeUuid) -> Option<&[TerrainUuid]> {
        self.0.get(key).map(|def| def.terrain.as_slice())
    }

    /// An ENUMERABLE iterator over every `(key, def)` the registry holds — so a consumer
    /// can list the themes it can offer.
    pub fn defs(&self) -> impl Iterator<Item = (&ThemeUuid, &UuidThemeDef)> {
        self.0.iter()
    }

    /// How many theme definitions the registry holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no theme definitions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
