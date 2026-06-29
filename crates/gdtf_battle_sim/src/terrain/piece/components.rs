//! The terrain-piece **identity newtypes** — [`TerrainName`] (a terrain file's
//! filename stem), [`TerrainGraphicKey`] (the opaque presenter-resolved graphic role),
//! and [`FootfallSound`] (the opaque presenter-resolved footfall sound key).
//!
//! Every newtype here is a `String`-newtype (no-bare-types rule 1 covers `String`
//! domain values): private inner, derived [`Deref`], `#[serde(transparent)]`, and
//! a `new` constructor. None of these carries domain math — they are opaque keys
//! the sim stores and round-trips; resolution to atlas indices / audio clips is
//! the presenter's concern.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A terrain piece's **name** — a terrain file's filename stem.
///
/// A terrain file's filename stem, e.g. `"deck_floor"` from `deck_floor.terrain.ron`;
/// the combat path never reads it. Mirrors [`WeaponName`](crate::weapon::WeaponName) /
/// [`ArmorName`](crate::armor::ArmorName) exactly.
///
/// A terrain-identity newtype over [`String`] (no-bare-types: a name is a domain
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string.
///
/// Implements [`Default`] (empty string sentinel) so [`Situation`](crate::situation::Situation)
/// can use `#[serde(default)]` on its [`default_floor`](crate::situation::Situation::default_floor)
/// field — an omitted field parses as an empty name, which the setup treats as
/// "no authored floor piece; fall back to `CombatTuning::move_costs.open`".
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainName(String);

impl TerrainName {
    /// Build a terrain name from its string key (the filename stem).
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }

    /// Whether this name is the empty-string sentinel (the `#[serde(default)]` for
    /// an omitted `default_floor` field — signals "no authored floor piece, use the
    /// `CombatTuning::move_costs.open` fallback").
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A terrain piece's **graphic role key** — an opaque string the presenter resolves
/// to a tile atlas entry via `TileRoles`.
///
/// The sim stores this key render-free and never resolves it to an atlas index —
/// that is the presenter's `TileRoles` job. The key follows the existing
/// `tile_roles.ron` vocabulary (e.g. `"floor"`, `"wall"`, `"cover"`) so the
/// presenter wire-up is later-trivial.
///
/// Derives [`Component`] so it can be attached to a terrain entity at setup
/// (GTW-396, Decision E: presentation-hook seam). The presenter queries it to
/// look up the tile atlas entry for rendering.
///
/// A presentation-hook newtype over [`String`] (no-bare-types rule 1: a domain key
/// string is wrapped). Private inner + derived [`Deref`];
/// `#[serde(transparent)]` round-trips a bare RON string. `Serialize` is added
/// (GTW-484) so the unified terrain-definition model
/// ([`TerrainPresenterKind`](crate::terrain::def::TerrainPresenterKind)) can reuse
/// this graphic key and round-trip it through serde.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TerrainGraphicKey(String);

impl TerrainGraphicKey {
    /// Build a graphic key from its role string (the `TileRoles` vocabulary key).
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// A terrain piece's **footfall sound key** — an opaque string the presenter
/// resolves to an audio clip.
///
/// No audio system is built yet (memory: *guns-only-no-melee-thrown-yet*); this
/// key is carried now so authored `.terrain.ron` files can specify it and the
/// future footfall-audio pass can consume it without a schema change.
///
/// Derives [`Component`] so it can be attached to a terrain entity at setup
/// (GTW-396, Decision E: presentation-hook seam). The future footfall-audio
/// system reads it to look up and play the correct audio clip when a ganger
/// steps on the cell (stubbed — no audio system yet; see GTW-XXX: footfall audio).
///
/// A presentation-hook newtype over [`String`] (no-bare-types rule 1). Private
/// inner + derived [`Deref`]; `#[serde(transparent)]` round-trips a bare RON string.
/// `Serialize` is added (GTW-484) so the unified terrain-definition model
/// ([`TerrainPresenterKind`](crate::terrain::def::TerrainPresenterKind)) can reuse
/// this slab footfall key and round-trip it through serde.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FootfallSound(String);

impl FootfallSound {
    /// Build a footfall sound key from its audio-asset key string.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}
