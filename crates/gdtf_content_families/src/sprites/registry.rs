//! The **sprite-def registry** — the [`SpriteName`]→[`SpriteDef`] map the
//! folder resolve builds and a `graphic_name` foreign key resolves against.

use bevy::prelude::{Deref, Resource};
use gdtf_battle_sim::registry::Registry;
use serde::{Deserialize, Serialize};

use super::def::SpriteDef;

/// A sprite definition's NAME — its registry key, the member's file stem
/// (`floor.spritedef.ron` → `floor`). A terrain def's `graphic_name` is a
/// foreign key carrying exactly this name.
///
/// A key newtype over [`String`] (no-bare-types rule 1). Private inner +
/// derived [`Deref`]; `#[serde(transparent)]` round-trips a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SpriteName(String);

impl SpriteName {
    /// Build a sprite name from its key string (a member file stem).
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The **sprite-def registry** — a [`SpriteName`]→[`SpriteDef`] map
/// (GTW-663), resolved from the `content/sprites/` folder by the generic
/// content-family seam.
///
/// A named [`Resource`] newtype over the foundation
/// [`Registry`]`<`[`SpriteName`]`, `[`SpriteDef`]`>` catalog map — see
/// [`Registry`] for the shared name→def surface these one-line wrappers
/// delegate to. Unlike the sim-owned registries this one lives in the glue
/// crate: the model is PRESENTATION data the render-free sim cannot own (see
/// the [module doc](super)). It holds the defs BY VALUE ([`SpriteDef`] is
/// `Clone`), so they survive the loaded-folder handle being dropped. NOT
/// `Eq`: a def's optional animation carries an `f32` rate.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SpriteDefRegistry(Registry<SpriteName, SpriteDef>);

impl SpriteDefRegistry {
    /// Build a registry from a `(name, def)` iterator — the shape the folder
    /// resolve (and a test) keys by member file stem.
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (SpriteName, SpriteDef)>) -> Self {
        Self(Registry::new(defs))
    }

    /// Insert one definition under its [`SpriteName`] key, returning the
    /// previous definition at that key (if any) — the per-member insert the
    /// folder resolve calls.
    pub fn insert(&mut self, name: SpriteName, def: SpriteDef) -> Option<SpriteDef> {
        self.0.insert(name, def)
    }

    /// Look up the [`SpriteDef`] for a sprite name, or [`None`] if no member
    /// with that stem was loaded — the resolution a consumer (GTW-665) reads.
    #[must_use]
    pub fn def(&self, name: &SpriteName) -> Option<&SpriteDef> {
        self.0.get(name)
    }

    /// Whether the registry holds a definition under this name — the
    /// existence question the `graphic_name` reference-integrity edge asks.
    #[must_use]
    pub fn contains(&self, name: &SpriteName) -> bool {
        self.0.contains(name)
    }

    /// An enumerable iterator over every `(name, def)` the registry holds —
    /// so a consumer (the GTW-664 editor picker) can list the whole loaded
    /// sprite library.
    pub fn defs(&self) -> impl Iterator<Item = (&SpriteName, &SpriteDef)> {
        self.0.iter()
    }

    /// Iterate over every [`SpriteName`] key — enumeration without the defs.
    /// Order is unspecified (see [`Registry`]); sort for a stable listing.
    pub fn keys(&self) -> impl Iterator<Item = &SpriteName> {
        self.0.keys()
    }

    /// How many definitions the registry holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no definitions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
