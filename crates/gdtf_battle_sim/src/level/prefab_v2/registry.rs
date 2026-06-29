//! The **v2 prefab registry** — the [`ThemeUuid`]-keyed counterpart of the legacy
//! [`PrefabRegistry`](crate::level::PrefabRegistry), introduced by the GTW-476 data-model
//! refactor (child T05b), living ALONGSIDE the legacy types rather than replacing them.
//!
//! Where the legacy [`PrefabRegistry`](crate::level::PrefabRegistry) keys its buckets on the
//! closed [`LevelTheme`](crate::level::LevelTheme) enum (via [`PrefabKey`](crate::level::PrefabKey)),
//! this v2 registry keys on the stable [`ThemeUuid`] (GTW-485) — so a theme can be renamed
//! or moved without breaking the prefab buckets that reference it. It holds validated
//! [`Prefab2`]s — each a [`PrefabName`] paired with a [`PrefabSpecV2`] (GTW-486) — and the v2
//! loader (T05c) populates it while the v2 assembler (T07b) reads it.
//!
//! This module is PURELY ADDITIVE (GTW-488): it adds the v2 KEY / PREFAB / REGISTRY ONLY —
//! the v2 loader (T05c) and assembler (T07b) are out of scope. The legacy
//! [`PrefabRegistry`](crate::level::PrefabRegistry) / [`PrefabKey`](crate::level::PrefabKey) /
//! [`Prefab`](crate::level::Prefab) / loader / assembler stay live and untouched.

use bevy::{platform::collections::HashMap, prelude::Resource};

use super::PrefabSpecV2;
use crate::level::{GridSize, PrefabName, SpawnRole, ThemeUuid};

/// One **validated** v2 prefab the [`PrefabRegistry2`] holds — its [`PrefabName`] paired
/// with a [`PrefabSpecV2`] (GTW-488).
///
/// The v2 counterpart of the legacy [`Prefab`](crate::level::Prefab), built through
/// [`Prefab2::new`]. Unlike the legacy constructor, [`Prefab2::new`] runs **NO**
/// edge-opening validation — the v2 schema carries no `edge_openings` field and no
/// [`NoEdgeOpening`](crate::level::PrefabLoadError::NoEdgeOpening) rejection path, because
/// inter-fragment connectivity is by-construction in the v2 assembler (T07b), not authored
/// per-prefab and validated fail-closed. An openingless prefab (a [`PrefabSpecV2`] carrying
/// zero placements) is therefore a VALID `Prefab2`. Holds the spec BY VALUE so it survives
/// the loaded folder handle being dropped. `Clone` (it owns the spec); not `Copy`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prefab2 {
    /// The prefab's name (the file stem the loader keyed it by).
    name: PrefabName,
    /// The authored v2 spec.
    spec: PrefabSpecV2,
}

impl Prefab2 {
    /// Build a v2 prefab from its name + spec.
    ///
    /// Runs NO edge-opening / connectivity validation (there is no such path in the v2
    /// schema), so this is INFALLIBLE — an openingless spec (zero placements) is a valid
    /// `Prefab2`. Contrast the legacy [`Prefab::new`](crate::level::Prefab::new), which
    /// runs the C6 [`validate`](crate::level::PrefabSpec::validate) and can reject.
    #[must_use]
    pub const fn new(name: PrefabName, spec: PrefabSpecV2) -> Self {
        Self { name, spec }
    }

    /// This prefab's name.
    #[must_use]
    pub const fn name(&self) -> &PrefabName {
        &self.name
    }

    /// This prefab's authored v2 spec.
    #[must_use]
    pub const fn spec(&self) -> &PrefabSpecV2 {
        &self.spec
    }
}

/// The v2 registry **enumeration key** — the `(theme, size, role)` triple the v2 assembler
/// (T07b) lists prefabs by, keyed on the stable [`ThemeUuid`] (GTW-488).
///
/// The v2 counterpart of the legacy [`PrefabKey`](crate::level::PrefabKey): a named struct
/// (no-bare-types: the lookup key is a domain value, not a bare tuple) whose `theme` field
/// is a stable [`ThemeUuid`] (GTW-485) rather than the legacy key's closed
/// [`LevelTheme`](crate::level::LevelTheme). All three leaf fields are NAMED newtypes
/// reused verbatim ([`ThemeUuid`] / [`GridSize`] / [`SpawnRole`]), each `Copy` + `Hash` +
/// `Eq`, so the key is a cheap copyable `HashMap` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PrefabKey2 {
    /// The stable [`ThemeUuid`] the prefab draws its terrain from.
    pub theme: ThemeUuid,
    /// The prefab's footprint dimensions.
    pub size:  GridSize,
    /// The deployment role the prefab plays.
    pub role:  SpawnRole,
}

impl PrefabKey2 {
    /// Build a `(theme, size, role)` enumeration key.
    #[must_use]
    pub const fn new(theme: ThemeUuid, size: GridSize, role: SpawnRole) -> Self {
        Self { theme, size, role }
    }
}

/// The **v2 prefab registry** — the per-`(theme, size, role)` bucketed map of validated
/// [`Prefab2`]s, keyed on the stable [`ThemeUuid`] (GTW-488).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`PrefabKey2`]`, Vec<`[`Prefab2`]`>>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), mirroring the legacy
/// [`PrefabRegistry`](crate::level::PrefabRegistry) shape but re-keyed by [`ThemeUuid`].
/// Multiple prefabs can share one `(theme, size, role)` key (the assembler picks among
/// them), so each key maps to a `Vec`. The v2 loader (T05c) POPULATES it from the loaded
/// maps folder and inserts it as a resource; the v2 assembler (T07b) enumerates it. It holds
/// the prefabs BY VALUE ([`Prefab2`] is `Clone`), so they survive the loaded-folder asset
/// handle being dropped.
///
/// Private inner with named accessors (the registry answers a prefab ENUMERATION question,
/// not a raw-map one — so no derived [`Deref`](bevy::prelude::Deref)) — the same
/// `prefabs_for` / `keys` / `insert` surface the legacy registry exposes.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PrefabRegistry2(HashMap<PrefabKey2, Vec<Prefab2>>);

impl PrefabRegistry2 {
    /// Insert one [`Prefab2`] under its `(theme, size, role)` key — the per-file insert the
    /// v2 folder loader (T05c) calls as it iterates the loaded folder.
    ///
    /// Buckets by the prefab's own spec [`theme`](PrefabSpecV2::theme) /
    /// [`size`](PrefabSpecV2::size) / [`role`](PrefabSpecV2::role), appending to any existing
    /// prefabs at that key (multiple prefabs may share one key — the assembler picks among
    /// them).
    pub fn insert(&mut self, prefab: Prefab2) {
        let key = PrefabKey2::new(prefab.spec().theme, prefab.spec().size, prefab.spec().role);
        self.0.entry(key).or_default().push(prefab);
    }

    /// List every prefab registered for a `(theme, size, role)` — the v2 assembler's
    /// enumeration query (T07b). Empty (an empty slice) if none match.
    #[must_use]
    pub fn prefabs_for(&self, key: &PrefabKey2) -> &[Prefab2] {
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
    pub fn keys(&self) -> impl Iterator<Item = &PrefabKey2> {
        self.0.keys()
    }
}
