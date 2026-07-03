//! The **melee weapon registry** — the name→spec map the folder loader builds and the
//! battle setup resolves ganger melee weapon keys against (GTW-505), the melee mirror
//! of the ranged [`WeaponRegistry`](super::super::WeaponRegistry).

use bevy::prelude::Resource;

use super::MeleeWeaponSpec;
use crate::{registry::Registry, weapon::WeaponName};

/// The key the [`fists` default](MeleeWeaponRegistry::fists) resolves under — the
/// filename stem of the shipped `assets/content/weapons/melee/fists.melee_weapon.ron`
/// (GTW-505). A ganger that authors NO melee weapon resolves to this key, so EVERY
/// ganger gets a melee weapon (the GTW-37 D3 ruling — any ganger can melee).
pub const FISTS_KEY: &str = "fists";

/// The **melee weapon registry** — a name→spec map the folder loader builds and the
/// battle setup resolves ganger melee weapon keys against (GTW-505), the melee mirror
/// of the ranged [`WeaponRegistry`](super::super::WeaponRegistry).
///
/// A named [`Resource`] newtype over the foundation [`Registry`]`<`[`WeaponName`]`,
/// `[`MeleeWeaponSpec`]`>` catalog map — see [`Registry`] for the shared name→def
/// surface these one-line wrappers delegate to. The sim OWNS the weapon model, so the
/// type lives here; the app's `Load` flow POPULATES it from the loaded
/// `assets/content/weapons/melee/*.melee_weapon.ron` folder (keyed by each file's
/// stem) and inserts it as a resource. It holds the specs BY VALUE
/// ([`MeleeWeaponSpec`] is `Clone`), so they survive even if the loaded-folder asset
/// handle is dropped. The setup resolves a ganger's melee weapon (`Some(key)` → that
/// entry, `None` → the [`fists`](MeleeWeaponRegistry::fists) default) through
/// [`spec`](MeleeWeaponRegistry::spec).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct MeleeWeaponRegistry(Registry<WeaponName, MeleeWeaponSpec>);

impl MeleeWeaponRegistry {
    /// Build a melee weapon registry from a `(name, spec)` iterator — the shape the
    /// folder loader (and a test) keys by filename stem.
    #[must_use]
    pub fn new(weapons: impl IntoIterator<Item = (WeaponName, MeleeWeaponSpec)>) -> Self {
        Self(Registry::new(weapons))
    }

    /// Insert one melee weapon spec under its [`WeaponName`] key, returning the previous
    /// spec at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: WeaponName, spec: MeleeWeaponSpec) -> Option<MeleeWeaponSpec> {
        self.0.insert(name, spec)
    }

    /// Look up the [`MeleeWeaponSpec`] for a melee weapon KEY, or [`None`] if no melee
    /// weapon file with that stem was loaded — the setup-time resolution the battle reads.
    #[must_use]
    pub fn spec(&self, name: &WeaponName) -> Option<&MeleeWeaponSpec> {
        self.0.get(name)
    }

    /// Look up the [`fists` default](FISTS_KEY) melee weapon — the unarmed fallback every
    /// ganger that authors no melee weapon resolves to (GTW-505 D3). `None` if the
    /// shipped `fists.melee_weapon.ron` did not load (the setup-time
    /// `MeleeWeaponNotFound` trigger, fail-closed exactly like the ranged path).
    #[must_use]
    pub fn fists(&self) -> Option<&MeleeWeaponSpec> {
        self.0.get(&WeaponName::new(FISTS_KEY.to_owned()))
    }

    /// How many melee weapons the registry holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no melee weapons.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Iterate over every [`WeaponName`] key in the registry — for editor dropdown
    /// enumeration (the ranged [`WeaponRegistry::keys`](super::super::WeaponRegistry)
    /// precedent), so a melee-weapon-selector UI can list all loaded melee weapons
    /// without exposing the inner map.
    ///
    /// Iteration order is unspecified (see [`Registry`]); callers that need a stable
    /// order must collect and sort.
    pub fn keys(&self) -> impl Iterator<Item = &WeaponName> {
        self.0.keys()
    }

    /// Iterate over every `(`[`WeaponName`]`,` [`MeleeWeaponSpec`]`)` pair in the
    /// registry — for editor dropdown enumeration (the ranged
    /// [`WeaponRegistry::iter`](super::super::WeaponRegistry) precedent).
    ///
    /// Iteration order is unspecified (see [`Registry`]); callers that need a stable
    /// order must collect and sort by name.
    pub fn iter(&self) -> impl Iterator<Item = (&WeaponName, &MeleeWeaponSpec)> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a MeleeWeaponRegistry {
    type Item = (&'a WeaponName, &'a MeleeWeaponSpec);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, WeaponName, MeleeWeaponSpec>;

    /// Iterate over `(`[`WeaponName`]`,` [`MeleeWeaponSpec`]`)` pairs via the
    /// [`IntoIterator`] trait — satisfies the `iter_without_into_iter` pedantic lint
    /// that requires a matching trait impl alongside an inherent `iter(&self)`. Delegates
    /// to the inner [`Registry`]'s borrowed iterator; order is unspecified.
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
