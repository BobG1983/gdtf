//! The **theme-definition key** — [`ThemeUuid`], a stable UUID the unified theme
//! model ([`UuidThemeDef`](super::UuidThemeDef)) is keyed by (GTW-485).

use bevy::{asset::uuid::Uuid, prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// A theme definition's **stable key** — the UUID that identifies one
/// [`UuidThemeDef`](super::UuidThemeDef) across the registry and (later) the consumers
/// that reference a theme.
///
/// Per the GTW-476 redesign, a theme is referenced by a stable UUID rather than the
/// closed [`LevelTheme`](crate::level::LevelTheme) enum it will eventually replace — so a
/// theme can be renamed or moved without breaking references. The
/// [`UuidThemeRegistry`](super::UuidThemeRegistry) keys definitions by this value.
///
/// A UUID newtype (no-bare-types rule 1: a key is a domain value, not a bare `Uuid`) and
/// — per rule 3 — a DISTINCT type from [`TerrainUuid`](crate::terrain::def::TerrainUuid): both
/// wrap the same inner `Uuid`, but a theme key is never a terrain key. The inner is the
/// [`Uuid`] **re-exported by Bevy** at `bevy::asset::uuid` — gdtf depends on the `uuid`
/// crate ONLY through Bevy (no-bare-types: no direct dep on a crate Bevy re-exports), the
/// same path [`TerrainUuid`](crate::terrain::def::TerrainUuid) uses. Private inner + **derived**
/// [`Deref`] (house style — the `Deref` is the derive, never hand-written); construct it
/// via [`new`](ThemeUuid::new) (wrap an existing `Uuid`) or
/// [`generate`](ThemeUuid::generate) (mint a fresh one).
///
/// `#[serde(transparent)]` round-trips it as the bare `Uuid` wire form (a string in RON's
/// human-readable encoding), and [`TypePath`] lets it ride a reflected payload the way the
/// sibling def types do.
///
/// Implements [`Default`] (the NIL-UUID sentinel) so
/// [`Situation`](crate::situation::Situation) can use `#[serde(default)]` on its
/// [`theme`](crate::situation::Situation) field — an omitted theme parses as the nil key
/// (GTW-491).
#[derive(
    Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize, TypePath,
)]
#[serde(transparent)]
pub struct ThemeUuid(Uuid);

impl ThemeUuid {
    /// Wrap an existing [`Uuid`] as a theme key — used when the UUID is supplied (an
    /// authored definition's key, or a reconstructed one).
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// The **nil** theme key — the [`Default`] sentinel (an all-zero UUID) signalling "no
    /// authored theme" (the GTW-491 successor to the omitted-`LevelTheme` default).
    #[must_use]
    pub const fn nil() -> Self {
        Self(Uuid::nil())
    }

    /// Whether this key is the [`nil`](ThemeUuid::nil) sentinel.
    #[must_use]
    pub const fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    /// Mint a **fresh, random** theme key (UUID v4) — used when authoring a new theme
    /// definition that has no key yet.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    /// A **deterministic** theme key derived from a legacy
    /// [`LevelTheme`](crate::level::LevelTheme)'s identifier string — the GTW-491 procgen
    /// SHIM bridge.
    ///
    /// The legacy procgen path ([`emit_level`](crate::procgen::emit_level)) is still keyed by
    /// the closed [`LevelTheme`](crate::level::LevelTheme) enum yet must populate the
    /// now-UUID-keyed [`Situation::theme`](crate::situation::Situation); this folds a theme
    /// identifier into a stable v8-shaped UUID via the same FNV-1a hash the terrain shim uses,
    /// so the same theme always yields the same key. It does NOT match a migrated
    /// [`UuidThemeDef`](super::UuidThemeDef)'s authored key — the full procgen switch onto real
    /// UUID-keyed themes is GTW-492 (T07b), which removes this shim.
    #[must_use]
    pub fn from_legacy_theme(name: &str) -> Self {
        Self(Uuid::from_u128(crate::terrain::def::fnv1a64_u128(
            name.as_bytes(),
        )))
    }
}
