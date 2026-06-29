//! The unified theme **definition** struct — [`UuidThemeDef`] — and its
//! [`ThemeDisplayName`] newtype (GTW-485).

use bevy::{prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

use super::ThemeUuid;
use crate::terrain::def::TerrainUuid;

/// A theme definition's **human-readable display name** — the label shown for a theme in
/// tooling / authoring (the editor palette's theme picker, etc.).
///
/// A name newtype over [`String`] (no-bare-types rule 1: a display name is a domain
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
/// string. The display name is for humans; the [`ThemeUuid`] is the key.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct ThemeDisplayName(String);

impl ThemeDisplayName {
    /// Build a theme display name from its label string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The **unified theme definition** — the NEW UUID-keyed theme model that will replace
/// the closed [`LevelTheme`](crate::level::LevelTheme) enum + its
/// [`ThemeSpec`](crate::level::ThemeSpec) catalog (GTW-485, child T03 of the GTW-476
/// refactor).
///
/// A theme names a stable key, a human label, its default floor terrain, and the terrain
/// palette it draws from — all by UUID into the unified terrain model:
/// - [`key`](UuidThemeDef::key) — the stable [`ThemeUuid`] the registry (and, later,
///   consumers) reference it by.
/// - [`display_name`](UuidThemeDef::display_name) — the human label ([`ThemeDisplayName`]).
/// - [`default_floor`](UuidThemeDef::default_floor) — the [`TerrainUuid`] of the terrain
///   definition that supplies this theme's per-theme floor. Its `MoveCost` comes from the
///   referenced [`TerrainDef`](crate::terrain::def::TerrainDef) — there is NO `Floor` kind; a
///   floor is just a terrain definition the theme nominates as its default ground.
/// - [`terrain`](UuidThemeDef::terrain) — the list of [`TerrainUuid`]s in this theme's
///   palette (the terrain a generated level / the editor draws from for this theme).
///
/// Derives [`Serialize`] / [`Deserialize`] (so a definition round-trips through RON) and
/// [`TypePath`] (so it can ride a reflected asset payload like the sibling def types).
///
/// **Not `Copy`** — [`ThemeDisplayName`] and the `terrain` [`Vec`] own heap data; it is
/// `Clone` so the registry can hold definitions by value.
///
/// **Purely additive (GTW-485)** — it lives ALONGSIDE the existing
/// [`LevelTheme`](crate::level::LevelTheme) / [`ThemeSpec`](crate::level::ThemeSpec) /
/// [`ThemeCatalogRegistry`](crate::level::ThemeCatalogRegistry); consumers switch over in
/// later tickets, NOT here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, TypePath)]
pub struct UuidThemeDef {
    /// The stable UUID key the registry (and, later, consumers) reference this theme by.
    pub key:           ThemeUuid,
    /// The human-readable display name (tooling / authoring label).
    pub display_name:  ThemeDisplayName,
    /// The [`TerrainUuid`] of the terrain definition that supplies this theme's default
    /// floor — its move cost comes from the referenced
    /// [`TerrainDef`](crate::terrain::def::TerrainDef), so there is no `Floor` kind.
    pub default_floor: TerrainUuid,
    /// The [`TerrainUuid`]s in this theme's terrain palette — what a generated level / the
    /// editor draws from for this theme.
    pub terrain:       Vec<TerrainUuid>,
}
