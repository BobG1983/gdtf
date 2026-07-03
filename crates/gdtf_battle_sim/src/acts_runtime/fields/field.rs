//! The **area-damage field** catalog model (GTW-545, child GTW-41f) — the
//! catalog-authoring [`FieldDef`] a field type carries.
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
//!   IMMUTABLE definition a placed field references. Its payload newtypes
//!   ([`FieldDamage`] / [`ImmuneArmorTypes`] / [`FieldDuration`]) live with their
//!   consequence BEHAVIOURS in the GTW-553 [`effects::fields`](crate::effects::fields)
//!   palette (re-exported from this module's parent, so `crate::fields::*` resolves
//!   unchanged), and [`FieldEffect::consequences_of`](crate::effects::fields::FieldEffect::consequences_of)
//!   projects an authored def into that consequence vocabulary.
//! - The **battle-state** side (the live per-cell placements + their remaining-turn
//!   countdown) is the [`FieldRegistry`](super::FieldRegistry) — the per-`(cell, level)`
//!   [`Resource`](bevy::prelude::Resource) the [`tick_fields`](super::tick_fields) clock
//!   drains, mirroring the [`CoverLedger`](crate::cover::CoverLedger) shape.

use bevy::reflect::TypePath;
use serde::Deserialize;

use crate::{
    effects::fields::{FieldDamage, FieldDuration, ImmuneArmorTypes},
    weapon::DamageType,
};

/// An area-damage field's **catalog definition** — the `{ damage, DamageType,
/// immune_armor_types, duration }` a field type authors (GTW-545).
///
/// The catalog-authoring side of the field model: a `RonAsset<FieldDef>` loaded from an
/// `assets/content/fields/*.field.ron` file (folder-loaded + hot-reloadable, mirroring the
/// [`ArmorSpec`](crate::armor::ArmorSpec) loader), keyed by its filename stem in the
/// [`FieldDefRegistry`](super::FieldDefRegistry). A placed field
/// ([`FieldRegistry`](super::FieldRegistry)) references a resolved copy of one of these plus a
/// live remaining-turn countdown. This flat struct IS the authored RON surface (GTW-553
/// keeps it unchanged); the consequence BEHAVIOURS it means live in the
/// [`effects::fields`](crate::effects::fields) palette, reached generically through
/// [`FieldEffect::consequences_of`](crate::effects::fields::FieldEffect::consequences_of).
///
/// A value object of named domain types (no bare primitive): the per-turn [`FieldDamage`], the
/// [`DamageType`] the field inflicts (reused — a field is a flavour of the same seven wheel
/// nodes, but the drain BYPASSES the matchup), the [`ImmuneArmorTypes`] whole-source-immunity
/// set, and the [`FieldDuration`] lifetime. NOT `Copy` (it owns a [`HashSet`](bevy::platform::collections::HashSet)
/// via [`ImmuneArmorTypes`]), so it is passed and stored by [`Clone`]. Derives [`Deserialize`] so
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
