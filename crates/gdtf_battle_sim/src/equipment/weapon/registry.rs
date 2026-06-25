//! The **weapon registry** — the name→spec map the folder loader builds and the
//! battle setup resolves ganger weapon keys against (GTW-257).

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{WeaponName, WeaponSpec};

/// The **weapon registry** — a name→spec map the folder loader builds and the
/// battle setup resolves [`GangerSpawn`](crate::situation::GangerSpawn) weapon keys
/// against (GTW-257).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`WeaponName`]`,
/// `[`WeaponSpec`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`). The sim OWNS the weapon model, so the type lives here; the app's
/// `Load` flow POPULATES it from the loaded `assets/weapons/*.ron` folder (keyed by
/// each file's stem) and inserts it as a resource. It holds the specs BY VALUE
/// ([`WeaponSpec`] is `Clone`), so they survive even if the loaded-folder asset
/// handle is dropped.
///
/// Private inner with small accessors (the registry answers a weapon LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref)). The setup
/// resolves [`GangerSpawn::weapon`](crate::situation::GangerSpawn) through
/// [`spec`](WeaponRegistry::spec).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct WeaponRegistry(HashMap<WeaponName, WeaponSpec>);

impl WeaponRegistry {
    /// Build a weapon registry from a `(name, spec)` iterator — the shape the
    /// folder loader (and a test) keys by filename stem.
    #[must_use]
    pub fn new(weapons: impl IntoIterator<Item = (WeaponName, WeaponSpec)>) -> Self {
        Self(weapons.into_iter().collect())
    }

    /// Insert one weapon spec under its [`WeaponName`] key, returning the previous
    /// spec at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: WeaponName, spec: WeaponSpec) -> Option<WeaponSpec> {
        self.0.insert(name, spec)
    }

    /// Look up the [`WeaponSpec`] for a weapon KEY, or [`None`] if no weapon file
    /// with that stem was loaded — the setup-time resolution the battle reads.
    #[must_use]
    pub fn spec(&self, name: &WeaponName) -> Option<&WeaponSpec> {
        self.0.get(name)
    }

    /// How many weapons the registry holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no weapons.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
