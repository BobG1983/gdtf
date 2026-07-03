//! The **prefab registry** — the [`ThemeUuid`]-keyed prefab bucket store introduced by
//! the GTW-476 data-model refactor (child T05b) and the SOLE prefab registry after GTW-496
//! retired the legacy theme-enum-keyed one. GTW-557 dropped the now-meaningless `2`/`V2`
//! suffixes.
//!
//! It keys its buckets on the stable [`ThemeUuid`] (GTW-485) — so a theme can be renamed or
//! moved without breaking the prefab buckets that reference it. It holds validated
//! [`Prefab`]s — each a [`PrefabName`] paired with a [`PrefabSpec`] (GTW-486) — and the
//! loader populates it while the assembler reads it.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::PrefabSpec;
use crate::level::{GridSize, PrefabName, SpawnRole, ThemeUuid};

/// One **validated** prefab the [`PrefabRegistry`] holds — its [`PrefabName`] paired
/// with a [`PrefabSpec`] (GTW-488).
///
/// Built through [`Prefab::new`], which runs **NO** opening validation — the schema
/// carries no authored-opening field and no opening-rejection path, because
/// inter-fragment connectivity is by-construction in the assembler (the 1-cell
/// `default_floor` seam every placement reserves), not authored per-prefab and validated
/// fail-closed (the old machinery was removed in GTW-497). An openingless prefab (a
/// [`PrefabSpec`] carrying zero placements) is therefore a VALID `Prefab`. Holds the spec
/// BY VALUE so it survives the loaded folder handle being dropped. `Clone` (it owns the
/// spec); not `Copy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefab {
    /// The prefab's name (the file stem the loader keyed it by).
    name: PrefabName,
    /// The authored spec.
    spec: PrefabSpec,
}

impl Prefab {
    /// Build a v2 prefab from its name + spec.
    ///
    /// Runs NO edge-opening / connectivity validation (there is no such path in the v2
    /// schema), so this is INFALLIBLE — an openingless spec (zero placements) is a valid
    /// `Prefab`.
    #[must_use]
    pub const fn new(name: PrefabName, spec: PrefabSpec) -> Self {
        Self { name, spec }
    }

    /// This prefab's name.
    #[must_use]
    pub const fn name(&self) -> &PrefabName {
        &self.name
    }

    /// This prefab's authored v2 spec.
    #[must_use]
    pub const fn spec(&self) -> &PrefabSpec {
        &self.spec
    }
}

/// The v2 registry **enumeration key** — the `(theme, size, role)` triple the assembler
/// (T07b) lists prefabs by, keyed on the stable [`ThemeUuid`] (GTW-488).
///
/// A named struct (no-bare-types: the lookup key is a domain value, not a bare tuple)
/// whose `theme` field is a stable [`ThemeUuid`] (GTW-485). All three leaf fields are
/// NAMED newtypes ([`ThemeUuid`] / [`GridSize`] / [`SpawnRole`]), each `Copy` + `Hash` +
/// `Eq`, so the key is a cheap copyable `HashMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefabKey {
    /// The stable [`ThemeUuid`] the prefab draws its terrain from.
    pub theme: ThemeUuid,
    /// The prefab's footprint dimensions.
    pub size:  GridSize,
    /// The deployment role the prefab plays.
    pub role:  SpawnRole,
}

impl PrefabKey {
    /// Build a `(theme, size, role)` enumeration key.
    #[must_use]
    pub const fn new(theme: ThemeUuid, size: GridSize, role: SpawnRole) -> Self {
        Self { theme, size, role }
    }
}

/// The **prefab registry** — the per-`(theme, size, role)` bucketed map of validated
/// [`Prefab`]s, keyed on the stable [`ThemeUuid`] (GTW-488).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`PrefabKey`]`, Vec<`[`Prefab`]`>>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), keyed by
/// [`ThemeUuid`].
/// Multiple prefabs can share one `(theme, size, role)` key (the assembler picks among
/// them), so each key maps to a `Vec`. The loader (T05c) POPULATES it from the loaded
/// maps folder and inserts it as a resource; the assembler (T07b) enumerates it. It holds
/// the prefabs BY VALUE ([`Prefab`] is `Clone`), so they survive the loaded-folder asset
/// handle being dropped.
///
/// Private inner with named accessors (the registry answers a prefab ENUMERATION question,
/// not a raw-map one — so no derived [`Deref`](bevy::prelude::Deref)) — the same
/// `prefabs_for` / `keys` / `insert` surface the legacy registry exposes.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefabRegistry(HashMap<PrefabKey, Vec<Prefab>>);

impl PrefabRegistry {
    /// Insert one [`Prefab`] under its `(theme, size, role)` key — the per-file insert the
    /// folder loader (T05c) calls as it iterates the loaded folder.
    ///
    /// Buckets by the prefab's own spec [`theme`](PrefabSpec::theme) /
    /// [`size`](PrefabSpec::size) / [`role`](PrefabSpec::role), appending to any existing
    /// prefabs at that key (multiple prefabs may share one key — the assembler picks among
    /// them).
    pub fn insert(&mut self, prefab: Prefab) {
        let key = PrefabKey::new(prefab.spec().theme, prefab.spec().size, prefab.spec().role);
        self.0.entry(key).or_default().push(prefab);
    }

    /// List every prefab registered for a `(theme, size, role)` — the assembler's
    /// enumeration query (T07b). Empty (an empty slice) if none match.
    #[must_use]
    pub fn prefabs_for(&self, key: &PrefabKey) -> &[Prefab] {
        self.0.get(key).map_or(&[], Vec::as_slice)
    }

    /// How many prefabs the registry holds across every key — the count a folder-load test
    /// asserts is non-empty.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.values().map(Vec::len).sum()
    }

    /// Whether the registry holds no prefabs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.values().all(Vec::is_empty)
    }

    /// Iterate over every `(theme, size, role)` key the registry has prefabs for — for a
    /// prefab-browser enumeration, so it can list available buckets without exposing the
    /// inner map. Iteration order is unspecified.
    pub fn keys(&self) -> impl Iterator<Item = &PrefabKey> {
        self.0.keys()
    }
}
