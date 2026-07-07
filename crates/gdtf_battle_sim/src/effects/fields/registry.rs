//! The two field registries (GTW-545): the [`FieldDefRegistry`] CATALOG (the name→def map
//! the folder loader builds, the [`ArmorRegistry`](crate::armor::ArmorRegistry) mirror) and
//! the live [`FieldRegistry`] RESOURCE (the per-`(cell, level)` placed-field map the
//! [`tick_fields`](super::tick_fields) clock drains, the
//! [`CoverLedger`](crate::cover::CoverLedger) mirror), plus the [`FieldRegistry::spawn`]
//! placement API the GTW-547 on-death `LeaveField` effect consumes.

use bevy::{
    platform::collections::HashMap,
    prelude::{Deref, Resource},
};
use serde::{Deserialize, Serialize};

use super::FieldDef;
use crate::{
    effects::fields::{ApplyFieldEffect, FieldEffect, FieldTurns},
    metric::CellLevel,
    registry::Registry,
};

/// A field type's **catalog KEY** — its human-facing identity (an authored field's filename
/// stem). The loader keys the [`FieldDefRegistry`] by it; a situation's authored
/// `FieldSpawn` references a field type by it.
///
/// A field-identity newtype over [`String`] (no-bare-types: a key is a domain value),
/// mirroring [`ArmorName`](crate::armor::ArmorName) exactly. Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON string. [`Serialize`] too, so a
/// [`FieldSpawn`](crate::situation::FieldSpawn) round-trips (the editor / procgen serialize
/// path — the [`CoverSpawn`](crate::situation::CoverSpawn) precedent).
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct FieldKey(String);

impl FieldKey {
    /// Build a field key from its stem string.
    #[must_use]
    pub const fn new(key: String) -> Self {
        Self(key)
    }
}

/// The **field-definition catalog** — a name→def map the folder loader builds and a
/// situation's authored `FieldSpawn` (and the GTW-547 spawn effect) resolve a
/// [`FieldKey`] against (GTW-545), mirroring the [`ArmorRegistry`](crate::armor::ArmorRegistry).
///
/// A named [`Resource`] newtype over the foundation [`Registry`]`<`[`FieldKey`]`,
/// `[`FieldDef`]`>` catalog map — see [`Registry`] for the shared name→def surface these
/// one-line wrappers delegate to. The sim OWNS the field model, so the type lives here; the
/// app's `Load` flow POPULATES it from the loaded `assets/content/fields/*.field.ron` folder
/// (keyed by each file's stem) and inserts it as a PERSISTENT resource (it survives past
/// `Load`, like the [`ArmorRegistry`](crate::armor::ArmorRegistry)). It holds the defs BY
/// VALUE (cloned in, so they survive the loaded-folder handle being dropped). A situation's
/// seed loop resolves a [`FieldKey`] through [`def`](FieldDefRegistry::def).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldDefRegistry(Registry<FieldKey, FieldDef>);

impl FieldDefRegistry {
    /// Build a field-def registry from a `(key, def)` iterator — the shape the folder loader
    /// (and a test) keys by filename stem.
    #[must_use]
    pub fn new(defs: impl IntoIterator<Item = (FieldKey, FieldDef)>) -> Self {
        Self(Registry::new(defs))
    }

    /// Insert one field def under its [`FieldKey`], returning the previous def at that key
    /// (if any) — the per-file insert the folder loader calls as it iterates the folder.
    pub fn insert(&mut self, key: FieldKey, def: FieldDef) -> Option<FieldDef> {
        self.0.insert(key, def)
    }

    /// Look up the [`FieldDef`] for a field KEY, or [`None`] if no field file with that stem
    /// was loaded — the setup-time resolution the situation seed reads.
    #[must_use]
    pub fn def(&self, key: &FieldKey) -> Option<&FieldDef> {
        self.0.get(key)
    }

    /// How many field types the catalog holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the catalog holds no field types.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// One **live placed field** — a resolved [`FieldDef`] plus the live remaining-turn countdown
/// its lifetime tracks (GTW-545).
///
/// The battle-state entry the [`FieldRegistry`] holds per `(cell, level)`: the immutable
/// [`def`](PlacedField::def) it was placed from, plus the [`remaining`](PlacedField::remaining)
/// turns for a [`FieldDuration::Turns`](crate::effects::fields::FieldDuration::Turns) field (a
/// [`FieldDuration::Permanent`](crate::effects::fields::FieldDuration::Permanent) field carries
/// NO countdown — the explicit `None`, never a magic zero (GTW-659) — and never expires).
/// Cloned by value from the catalog def at placement, so it survives the catalog registry
/// being dropped. Both lifetime steps (the countdown seed and the per-round expiry) are the
/// GTW-553 palette's Duration consequence — invoked generically through
/// [`ApplyFieldEffect`], never matched here.
///
/// A named value object (no bare fields): the def is the domain [`FieldDef`], the remaining
/// count an `Option` of the domain [`FieldTurns`] (`Some` = a finite countdown, positive by
/// construction — [`FieldTurns`] wraps a `NonZeroU8`; `None` = no countdown, the
/// [`Option<DotProfile>`](crate::weapon::DotProfile) optional-domain-value precedent). Not a
/// [`Component`](bevy::prelude::Component) — a placed field is a registry entry keyed by
/// cell, not an entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedField {
    /// The immutable field definition this placement was made from.
    def:       FieldDef,
    /// The remaining turns for a finite-lifetime field — decremented-or-expired one per
    /// [`tick_fields`](super::tick_fields) round by the palette's Duration consequence;
    /// `None` for a permanent field (no countdown — it never expires).
    remaining: Option<FieldTurns>,
}

impl PlacedField {
    /// Place a field from its catalog [`FieldDef`] — the remaining countdown is SEEDED by
    /// the def's consequences, invoked generically through the GTW-553 palette (only the
    /// lifetime consequence answers; every other consequence contributes the `None`
    /// default, so the fold's `max` picks the lifetime's seed — `None < Some(_)` — and a
    /// permanent field's placement carries the explicit no-countdown `None`, GTW-659).
    #[must_use]
    pub fn from_def(def: FieldDef) -> Self {
        let remaining = FieldEffect::consequences_of(&def)
            .iter()
            .map(ApplyFieldEffect::initial_countdown)
            .max()
            .flatten();
        Self { def, remaining }
    }

    /// The immutable field definition this placement was made from — the source the tick reads
    /// the per-turn damage + immune set off.
    #[must_use]
    pub const fn def(&self) -> &FieldDef {
        &self.def
    }

    /// The remaining-turns countdown — `Some` for a finite-lifetime field (positive by
    /// construction), `None` for a permanent one (no countdown; GTW-659).
    #[must_use]
    pub const fn remaining(&self) -> Option<FieldTurns> {
        self.remaining
    }

    /// Count this placement's lifetime down one turn and report `true` iff it is now
    /// EXPIRED — the per-round lifetime step, invoked generically through the GTW-553
    /// palette's [`ApplyFieldEffect::count_down_one_turn`] verb (never a duration match
    /// here). At most ONE consequence owns the lifetime (the def's Duration); the others
    /// answer the defaulted never-expires and never touch the countdown, so `any` is
    /// exact — it cannot short-circuit past a decrement.
    pub fn tick_down(&mut self) -> bool {
        FieldEffect::consequences_of(&self.def)
            .iter()
            .any(|consequence| consequence.count_down_one_turn(&mut self.remaining))
    }
}

/// The **live area-damage-field registry** — the per-`(cell, level)` map of placed fields the
/// [`tick_fields`](super::tick_fields) clock drains, and the [`spawn`](FieldRegistry::spawn)
/// placement API the GTW-547 on-death effect consumes (GTW-545).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`CellLevel`]`, `[`PlacedField`]`>`
/// (no-bare-types: a registry is a domain value, not a bare `HashMap`), the
/// [`CoverLedger`](crate::cover::CoverLedger) mirror. It is a BATTLE-LIFETIME resource:
/// [`setup_battle`](crate::situation::setup_battle) inserts it (seeded from the situation's
/// authored `fields:` list) on the same `Ok` path as the other battle grids, and teardown
/// removes it. It PERSISTS across turns — a `Turns` field survives until its countdown
/// empties, a `Permanent` field forever.
///
/// Private inner with placement / iteration accessors (the registry answers a field-placement
/// question, not a raw-map one — so no derived [`Deref`](bevy::prelude::Deref)). One field per
/// cell (last-write-wins on a re-[`spawn`](FieldRegistry::spawn) at an occupied cell — a fresh
/// hazard replaces the old, mirroring the DOT refresh-not-stack ruling).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct FieldRegistry(HashMap<CellLevel, PlacedField>);

impl FieldRegistry {
    /// Build an empty field registry — the setup seed starts here and
    /// [`spawn`](FieldRegistry::spawn)s each authored placement.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// **Place** a field at `at` from its catalog [`FieldDef`] — the GTW-545 spawn API the
    /// GTW-547 on-death `LeaveField` effect (and the [`setup_battle`](crate::situation::setup_battle)
    /// seed loop) call to put a live field on a cell.
    ///
    /// A fresh placement at an already-fielded cell REPLACES the old (last-write-wins — a
    /// new hazard supersedes the prior, mirroring the DOT refresh-not-stack ruling). Returns
    /// the previous [`PlacedField`] at that cell (if any).
    pub fn spawn(&mut self, at: CellLevel, def: FieldDef) -> Option<PlacedField> {
        self.0.insert(at, PlacedField::from_def(def))
    }

    /// The placed field at `at`, or [`None`] if the cell carries no field — a read-only peek.
    #[must_use]
    pub fn field_at(&self, at: &CellLevel) -> Option<&PlacedField> {
        self.0.get(at)
    }

    /// Iterate over every `(`[`CellLevel`]`, `[`PlacedField`]`)` placement — the shape
    /// [`tick_fields`](super::tick_fields) folds over to drain each fielded cell's occupant.
    /// [`HashMap`] iteration order is unspecified; the tick's per-cell drain is
    /// order-independent (each cell has at most one occupant).
    pub fn iter(&self) -> impl Iterator<Item = (&CellLevel, &PlacedField)> {
        self.0.iter()
    }

    /// Decrement every placed field's countdown one turn and REMOVE the ones that expired —
    /// the GTW-545 per-round lifetime step [`tick_fields`](super::tick_fields) runs after it
    /// drains occupants. A [`FieldDuration::Turns`](crate::effects::fields::FieldDuration::Turns)
    /// field whose countdown spends its LAST round is removed (decrement-or-expire — a
    /// zero-valued countdown is never stored, GTW-659; `Turns(n)` = exactly `n` draining
    /// rounds); a [`FieldDuration::Permanent`](crate::effects::fields::FieldDuration::Permanent)
    /// field never counts down and never expires (each placement's step is the palette's
    /// Duration consequence, via [`PlacedField::tick_down`]).
    pub fn tick_down_and_expire(&mut self) {
        self.0.retain(|_cell, placed| !placed.tick_down());
    }

    /// How many live fields the registry holds — the count a seed / expiry test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no live fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a FieldRegistry {
    type Item = (&'a CellLevel, &'a PlacedField);
    type IntoIter = bevy::platform::collections::hash_map::Iter<'a, CellLevel, PlacedField>;

    /// Iterate over `(`[`CellLevel`]`, `[`PlacedField`]`)` pairs via the [`IntoIterator`]
    /// trait — satisfies the `iter_without_into_iter` pedantic lint that requires a matching
    /// trait impl alongside an inherent `iter(&self)`. Delegates to the inner [`HashMap`]'s
    /// owned iterator; order is unspecified.
    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}
