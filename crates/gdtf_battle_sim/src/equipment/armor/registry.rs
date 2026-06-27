//! The **armor registry** — the name→spec map the folder loader builds and the
//! battle setup resolves ganger armor keys against (GTW-269), the armor mirror of
//! the [`WeaponRegistry`](crate::weapon::WeaponRegistry).

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};
use serde::Deserialize;

use super::ArmorSpec;

/// An armor suit's **name** — its human-facing identity (an authored armor's
/// display name). The loader keys the [`ArmorRegistry`] by it (the armor file's
/// filename stem); the §"Per-hit resolution" armor math never reads it.
///
/// An armor-identity newtype over [`String`] (no-bare-types: a name is a domain
/// value), mirroring [`WeaponName`](crate::weapon::WeaponName) exactly. Private
/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON string.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct ArmorName(String);

impl ArmorName {
    /// Build an armor name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The **armor registry** — a name→spec map the folder loader builds and the
/// battle setup resolves ganger armor keys against (GTW-269), mirroring the
/// [`WeaponRegistry`](crate::weapon::WeaponRegistry).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`ArmorName`]`, `[`ArmorSpec`]`>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`). The sim OWNS
/// the armor model, so the type lives here; the app's `Load` flow POPULATES it from
/// the loaded `assets/content/armor/*.ron` folder (keyed by each file's stem) and inserts it
/// as a resource. It holds the specs BY VALUE ([`ArmorSpec`] is `Copy`), so they
/// survive even if the loaded-folder asset handle is dropped.
///
/// Private inner with small accessors (the registry answers an armor LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref)). The setup
/// resolves a ganger's armor key through [`spec`](ArmorRegistry::spec).
///
/// Mirrors [`WeaponRegistry`](crate::weapon::WeaponRegistry)'s derive set, plus
/// [`Eq`] — an [`ArmorSpec`] is fully `Eq` (only `i32`/enum leaves, no floats,
/// unlike a [`WeaponSpec`](crate::weapon::WeaponSpec)), so the map is `Eq` and the
/// `clippy::derive_partial_eq_without_eq` lint requires it.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ArmorRegistry(HashMap<ArmorName, ArmorSpec>);

impl ArmorRegistry {
    /// Build an armor registry from a `(name, spec)` iterator — the shape the
    /// folder loader (and a test) keys by filename stem.
    #[must_use]
    pub fn new(armors: impl IntoIterator<Item = (ArmorName, ArmorSpec)>) -> Self {
        Self(armors.into_iter().collect())
    }

    /// Insert one armor spec under its [`ArmorName`] key, returning the previous
    /// spec at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: ArmorName, spec: ArmorSpec) -> Option<ArmorSpec> {
        self.0.insert(name, spec)
    }

    /// Look up the [`ArmorSpec`] for an armor KEY, or [`None`] if no armor file
    /// with that stem was loaded — the setup-time resolution the battle reads.
    #[must_use]
    pub fn spec(&self, name: &ArmorName) -> Option<&ArmorSpec> {
        self.0.get(name)
    }

    /// How many armor suits the registry holds — the count the folder-load test
    /// asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no armor suits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over every [`ArmorName`] key in the registry — **for editor
    /// dropdown enumeration (GTW-413/GTW-425)**, so the armor-selector UI can
    /// list all loaded armor suits without exposing the inner map.
    ///
    /// [`HashMap`] iteration order is unspecified; callers that need a stable,
    /// reproducible order (e.g. a sorted dropdown) must collect and sort.
    pub fn keys(&self) -> impl Iterator<Item = &ArmorName> {
        self.0.keys()
    }

    /// Iterate over every `(`[`ArmorName`]`,` [`ArmorSpec`]`)` pair in the
    /// registry — **for editor dropdown enumeration (GTW-413/GTW-425)**, so the
    /// armor-selector UI can display each armor suit's name alongside its spec.
    ///
    /// [`HashMap`] iteration order is unspecified; callers that need a stable,
    /// reproducible order must collect and sort by name.
    pub fn iter(&self) -> impl Iterator<Item = (&ArmorName, &ArmorSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a ArmorRegistry {
    type Item = (&'a ArmorName, &'a ArmorSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, ArmorName, ArmorSpec>;

    /// Iterate over `(`[`ArmorName`]`,` [`ArmorSpec`]`)` pairs via the
    /// [`IntoIterator`] trait — satisfies the `iter_without_into_iter` pedantic
    /// lint that requires a matching trait impl alongside an inherent `iter(&self)`.
    /// Delegates to the inner [`HashMap`]'s owned iterator; order is unspecified.
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
