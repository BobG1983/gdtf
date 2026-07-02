//! The **area-damage field** model (GTW-545, child GTW-41f) — the catalog-authoring
//! [`FieldDef`] a field type carries, the newtypes its magnitudes wrap, the
//! [`FieldDuration`] lifetime it lives for, and the whole-source-immunity
//! [`ImmuneArmorTypes`] set that skips its drain.
//!
//! An area-damage field is a persistent per-tile damage zone (a toxic waste pool, an
//! electrified floor, a patch of burning ground): a `(cell, level)` that eats the
//! [`Hp`](crate::ganger::Hp) of whatever ganger stands on it, once per turn, until the
//! field expires (`docs/combat/resolution.md` — the area-damage-field beat of GTW-41).
//! The shapes split cleanly along the model/runtime line, mirroring the GTW-544 DOT model:
//!
//! - [`FieldDef`] is the **catalog authoring** side — the `{ damage, DamageType,
//!   immune_armor_types, duration }` an `assets/content/fields/*.field.ron` authors, keyed
//!   by its filename stem in the [`FieldDefRegistry`](super::FieldDefRegistry). It is the
//!   IMMUTABLE definition a placed field references.
//! - The **battle-state** side (the live per-cell placements + their remaining-turn
//!   countdown) is the [`FieldRegistry`](super::FieldRegistry) — the per-`(cell, level)`
//!   [`Resource`](bevy::prelude::Resource) the [`tick_fields`](super::tick_fields) clock
//!   drains, mirroring the [`CoverLedger`](crate::cover::CoverLedger) shape.
//!
//! **Whole-source immunity** ([`ImmuneArmorTypes`]) is a NEW mechanism (GTW-545): a field is
//! gated ONLY by whole-armor-type immunity — a ganger ANY of whose worn armor pieces carries
//! an [`ArmorType`] in the field's immune set takes ZERO damage (the sealed suit protects
//! you), with NO per-hit armor matchup, NO injury roll, and NO RNG (the deterministic field
//! tick). The seven-node damage wheel has no whole-source-skip equivalent today.

use bevy::{platform::collections::HashSet, prelude::Deref, reflect::TypePath};
use serde::Deserialize;

use crate::{armor::ArmorType, weapon::DamageType};

/// The **per-turn HP damage** an area-damage field deals each turn a ganger stands in it —
/// the flat amount the field eats from the occupant's [`Hp`](crate::ganger::Hp) pool every
/// turn, bypassing armor entirely (gated only by whole-source immunity, never a matchup).
///
/// A field damage NUMBER (a small per-turn count, `u16` to match the [`Hp`](crate::ganger::Hp)
/// inner). A no-bare-types newtype: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so a field's authored [`FieldDef`] `.ron` names it as a bare
/// integer. Distinct from [`crate::weapon::DotDamage`] (the DOT per-turn tick) — a field's
/// per-turn drain is its OWN quantity, keyed to the field type, never a weapon's.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct FieldDamage(u16);

impl FieldDamage {
    /// Build a per-turn field damage from its count.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// The **duration in turns** a [`FieldDuration::Turns`] field lingers — how many turns the
/// zone ticks before it is removed (`docs/combat/resolution.md` — the area-damage-field beat).
///
/// A field duration NUMBER (a small turn count, `u8` — a field lasts a handful of turns). A
/// no-bare-types newtype: private inner + derived [`Deref`]; `#[serde(transparent)]` so a
/// field's authored [`FieldDuration::Turns`] `.ron` names it as a bare integer. Distinct from
/// every [`Tu`](crate::ganger::Tu)-domain count — this is a count of TURNS the field runs, not
/// a TU cost, and distinct from [`crate::weapon::DotTurns`] (the DOT clock).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct FieldTurns(u8);

impl FieldTurns {
    /// Build a field duration from its turn count.
    #[must_use]
    pub const fn new(turns: u8) -> Self {
        Self(turns)
    }
}

/// How long an area-damage field persists — a fixed number of [`FieldTurns`], or forever.
///
/// The lifetime arm of the field model (`docs/combat/resolution.md` — the area-damage-field
/// beat): a [`Turns`](FieldDuration::Turns) field counts down one turn per
/// [`tick_fields`](super::tick_fields) round and is removed at zero (a thrown gas grenade's
/// dissipating cloud); a [`Permanent`](FieldDuration::Permanent) field NEVER expires (a
/// toxic-waste pool seeded as fixed terrain). A named domain enum (no-bare-types: a field
/// lifetime is a domain value, not a bare `Option<u8>`).
///
/// Derives [`Deserialize`] so an authored `.ron` writes `duration: Turns(3)` or
/// `duration: Permanent`. Derives the sentinel [`Default`] ([`Turns`](FieldDuration::Turns)
/// of zero — an immediately-expiring no-op) so a [`FieldDef`] composes cleanly; the default is
/// never authored (a real field authors either a positive `Turns` count or `Permanent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum FieldDuration {
    /// The field ticks for exactly this many turns, then is removed.
    Turns(FieldTurns),
    /// The field never expires — a fixed terrain hazard (a toxic-waste pool).
    Permanent,
}

impl Default for FieldDuration {
    /// The sentinel default: a zero-turn (immediately-expiring) field. Never authored — a
    /// real field authors a positive [`Turns`](FieldDuration::Turns) count or
    /// [`Permanent`](FieldDuration::Permanent); the default exists only so [`FieldDef`]
    /// derives [`Default`] cleanly.
    fn default() -> Self {
        Self::Turns(FieldTurns::new(0))
    }
}

/// The set of [`ArmorType`]s that make a ganger **wholly immune** to a field — the GTW-545
/// whole-source-immunity mechanism (`docs/combat/resolution.md` — the area-damage-field beat).
///
/// A ganger ANY of whose worn armor pieces carries an [`ArmorType`] in this set takes ZERO
/// damage from the field (the sealed suit protects you) — a whole-SOURCE skip, distinct from
/// the per-hit armor matchup (which soaks partial damage per wheel node). The seven-node
/// damage wheel has no whole-source-skip equivalent today; this is the new mechanism.
///
/// A named newtype [`HashSet`] (no-bare-types: the immune set is a domain value, not a bare
/// `HashSet<ArmorType>` in a `pub` field). Private inner with a [`contains`](ImmuneArmorTypes::contains)
/// accessor (the set answers a membership question, not a raw-set one). Derives
/// [`Deserialize`] so an authored `.ron` names it as a bare RON list of [`ArmorType`] variants
/// (`immune_armor_types: [Flak, Hazard]`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ImmuneArmorTypes(HashSet<ArmorType>);

impl ImmuneArmorTypes {
    /// Build an immune-armor-type set from an [`ArmorType`] iterator — the shape a test (and a
    /// field spawn call site) constructs.
    #[must_use]
    pub fn new(types: impl IntoIterator<Item = ArmorType>) -> Self {
        Self(types.into_iter().collect())
    }

    /// Whether this set contains `armor_type` — the membership check
    /// [`tick_fields`](super::tick_fields) runs per worn armor piece to decide whole-source
    /// immunity (a match on ANY worn piece skips the field's drain entirely).
    #[must_use]
    pub fn contains(&self, armor_type: &ArmorType) -> bool {
        self.0.contains(armor_type)
    }

    /// Whether this set is empty (no armor type grants immunity) — the common case for a
    /// field nothing protects against.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// An area-damage field's **catalog definition** — the `{ damage, DamageType,
/// immune_armor_types, duration }` a field type authors (GTW-545).
///
/// The catalog-authoring side of the field model: a `RonAsset<FieldDef>` loaded from an
/// `assets/content/fields/*.field.ron` file (folder-loaded + hot-reloadable, mirroring the
/// [`ArmorSpec`](crate::armor::ArmorSpec) loader), keyed by its filename stem in the
/// [`FieldDefRegistry`](super::FieldDefRegistry). A placed field
/// ([`FieldRegistry`](super::FieldRegistry)) references a resolved copy of one of these plus a
/// live remaining-turn countdown.
///
/// A value object of named domain types (no bare primitive): the per-turn [`FieldDamage`], the
/// [`DamageType`] the field inflicts (reused — a field is a flavour of the same seven wheel
/// nodes, but the drain BYPASSES the matchup), the [`ImmuneArmorTypes`] whole-source-immunity
/// set, and the [`FieldDuration`] lifetime. NOT `Copy` (it owns a [`HashSet`] via
/// [`ImmuneArmorTypes`]), so it is passed and stored by [`Clone`]. Derives [`Deserialize`] so
/// the loose `.ron` parses it; NOT a [`Component`](bevy::prelude::Component) (a field is a
/// registry/resource entry, not an entity's component). The `DamageType` here is a presentation
/// / flavour tag, never a soak lookup — the field tick bypasses armor entirely (gated only by
/// [`immune_armor_types`](FieldDef::immune_armor_types)).
///
/// Derives [`TypePath`] (render-free reflection metadata, no rendering) because the
/// `RonAsset<FieldDef>` the folder loader wraps it in requires its payload to be [`TypePath`]
/// — the same bound the [`Situation`](crate::situation::Situation) / armor spec satisfy.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, TypePath)]
pub struct FieldDef {
    /// The per-turn HP damage each tick deals (bypasses the matchup wheel).
    pub damage:             FieldDamage,
    /// The damage type the field inflicts — its wheel-node flavour (presentation only).
    pub damage_type:        DamageType,
    /// The armor types that grant WHOLE-SOURCE immunity — a ganger wearing ANY piece of one
    /// of these takes zero damage (the GTW-545 new mechanism).
    pub immune_armor_types: ImmuneArmorTypes,
    /// How long the field persists — a fixed turn count, or forever.
    pub duration:           FieldDuration,
}

impl FieldDef {
    /// Build a field definition from its per-turn damage, damage type, immune set, and
    /// duration — the shape the catalog loader (and a test) constructs.
    #[must_use]
    pub const fn new(
        damage: FieldDamage,
        damage_type: DamageType,
        immune_armor_types: ImmuneArmorTypes,
        duration: FieldDuration,
    ) -> Self {
        Self {
            damage,
            damage_type,
            immune_armor_types,
            duration,
        }
    }
}
