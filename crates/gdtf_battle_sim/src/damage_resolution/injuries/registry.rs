//! The **injury registry** — the name→def map the folder loader builds from
//! `assets/injuries/**/*.injury.ron` and the roll resolves a rolled
//! [`InjuryName`] key against (GTW-437).

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::{InjuryDef, InjuryName};

/// The **injury registry** — a name→def map the GTW-437 folder loader builds from
/// every loaded `assets/injuries/**/*.injury.ron` (keyed by each file's stem minus
/// the `.injury` infix) and the GTW-438 roll resolves a rolled
/// [`InjuryName`] key against.
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`InjuryName`]`, `[`InjuryDef`]`>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), mirroring the
/// landed [`WeaponRegistry`](crate::weapon::WeaponRegistry). The sim OWNS the injury
/// model, so the type lives here; the app's `Load` flow POPULATES it from the loaded
/// folder and inserts it as a resource. It holds the defs BY VALUE
/// ([`InjuryDef`] is `Clone`), so they survive even if the loaded-folder asset handle
/// is dropped on `OnExit(Load)`.
///
/// Private inner with small accessors (the registry answers an injury LOOKUP, not a
/// raw-map question — so no derived [`Deref`](bevy::prelude::Deref)).
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-444): its [`InjuryDef`] values may carry a
/// [`MovementCostMul`](super::InjuryEffect::MovementCostMul) whose `f32` payload is not
/// `Eq`. The registry is read by `InjuryName` lookup, never compared as a whole in a
/// hashed/ordered set.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct InjuryRegistry(HashMap<InjuryName, InjuryDef>);

impl InjuryRegistry {
    /// Build an injury registry from a `(name, def)` iterator — the shape the folder
    /// loader (and a test) keys by filename stem.
    #[must_use]
    pub fn new(injuries: impl IntoIterator<Item = (InjuryName, InjuryDef)>) -> Self {
        Self(injuries.into_iter().collect())
    }

    /// Insert one injury def under its [`InjuryName`] key, returning the previous def
    /// at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: InjuryName, def: InjuryDef) -> Option<InjuryDef> {
        self.0.insert(name, def)
    }

    /// Look up the [`InjuryDef`] for an injury KEY, or [`None`] if no injury file with
    /// that stem was loaded — the roll-time resolution the GTW-438 roll reads.
    #[must_use]
    pub fn def(&self, name: &InjuryName) -> Option<&InjuryDef> {
        self.0.get(name)
    }

    /// Whether the registry holds an injury under this KEY — the missing-weighting
    /// audit (GTW-437) checks every weighting entry's key resolves here.
    #[must_use]
    pub fn contains(&self, name: &InjuryName) -> bool {
        self.0.contains_key(name)
    }

    /// Iterate the registry's `(key, def)` pairs — the missing-weighting audit walks
    /// every registered injury to warn on any that no weighting bucket references.
    pub fn iter(&self) -> impl Iterator<Item = (&InjuryName, &InjuryDef)> {
        self.0.iter()
    }

    /// How many injuries the registry holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no injuries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
