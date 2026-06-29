//! The **terrain-definition key** — [`TerrainUuid`], a stable UUID the unified
//! terrain model ([`TerrainDef`](super::TerrainDef)) is keyed by (GTW-484).

use bevy::{asset::uuid::Uuid, prelude::Deref, reflect::TypePath};
use serde::{Deserialize, Serialize};

/// A terrain definition's **stable key** — the UUID that identifies one
/// [`TerrainDef`](super::TerrainDef) across themes, prefabs, and the registry.
///
/// Per the GTW-476 redesign, terrain is referenced by a stable UUID (not by a
/// filename stem like the legacy [`TerrainName`](crate::terrain::piece::TerrainName))
/// so a definition can be renamed or moved without breaking the references that point
/// at it. The [`TerrainDefRegistry`](super::TerrainDefRegistry) keys definitions by
/// this value.
///
/// A UUID newtype (no-bare-types rule 1: a key is a domain value, not a bare `Uuid`).
/// The inner is the [`Uuid`] **re-exported by Bevy** at `bevy::asset::uuid` — gdtf
/// depends on the `uuid` crate ONLY through Bevy (no-bare-types: no direct dep on a
/// crate Bevy re-exports), the same path `gdtf_ui` already uses for weak asset
/// handles. Private inner + **derived** [`Deref`] (house style — the `Deref` is the
/// derive, never hand-written); construct it via [`new`](TerrainUuid::new) (wrap an
/// existing `Uuid`) or [`generate`](TerrainUuid::generate) (mint a fresh one).
///
/// `#[serde(transparent)]` round-trips it as the bare `Uuid` wire form (a string in
/// RON's human-readable encoding), and [`TypePath`] lets it ride a reflected asset
/// payload the same way the sibling def types do.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, TypePath)]
#[serde(transparent)]
pub struct TerrainUuid(Uuid);

impl TerrainUuid {
    /// Wrap an existing [`Uuid`] as a terrain key — used when the UUID is supplied
    /// (an authored definition's key, or a reconstructed one).
    #[must_use]
    pub const fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Mint a **fresh, random** terrain key (UUID v4) — used when authoring a new
    /// terrain definition that has no key yet.
    #[must_use]
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}
