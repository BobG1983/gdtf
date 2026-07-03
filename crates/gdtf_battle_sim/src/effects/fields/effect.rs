//! The closed **field-consequence vocabulary** — the [`FieldEffect`] an area-damage
//! field's authored [`FieldDef`](crate::fields::FieldDef) means (GTW-545; GTW-553 re-homes
//! the consequence behaviours into the [`effects`](crate::effects) palette).
//!
//! ## The serde name↔type bridge + effect isolation (GTW-553)
//!
//! This vocabulary is a closed serde enum (the name↔type bridge — RON cannot deserialize
//! trait objects). Unlike its palette siblings it is NOT yet RON-exposed: a shipped
//! `assets/content/fields/*.field.ron` authors the flat
//! [`FieldDef`](crate::fields::FieldDef) `{ damage, damage_type, immune_armor_types,
//! duration }` struct (unchanged by GTW-553), and [`FieldEffect::consequences_of`] is the
//! bridge that PROJECTS that authored def into this vocabulary — so the def stays the
//! authoring surface while every consequence BEHAVIOUR lives isolated in the palette.
//!
//! Each variant's behaviour is a CONCEPTUALLY-ISOLATED type in its OWN sibling file
//! impl-ing the [`ApplyFieldEffect`] trait (its verbs ARE its behaviour — no central logic
//! `match`, no inline tick branch, no authoring step scattered across the tree), and this
//! enum's own [`ApplyFieldEffect`] impl forwards every verb through the ONE
//! purely-mechanical `with_behaviour` match — the ONLY sim-side match over this
//! vocabulary. The fields MECHANICS ([`tick_fields`](crate::fields::tick_fields) + the
//! [`PlacedField`](crate::fields::PlacedField) lifetime) invoke the trait generically.
//!
//! Adding a new consequence means ONE new per-consequence file + ONE variant here + ONE
//! delegation arm + ONE `mod` line (plus its [`consequences_of`](FieldEffect::consequences_of)
//! projection entry once the def authors it) — compile-checked (the delegation match is
//! exhaustive, so a new variant without an arm is a build error, never a silent no-op or a
//! denied panic).

use bevy::prelude::Entity;
use serde::Deserialize;

use super::{
    ApplyDrain, ApplyDuration, ApplyFieldEffect, ApplyImmunity, FieldDamage, FieldDuration,
    FieldTurns, ImmuneArmorTypes, OccupantArmor, OccupantDrain,
};
use crate::{fields::FieldDef, metric::CellLevel, weapon::DamageType};

/// One **consequence** of an area-damage field — the atomic thing standing in (or
/// placing) the field DOES (GTW-545; palette-isolated per GTW-553).
///
/// A named domain enum (no-bare-types: a field consequence is a domain value). NOT yet
/// RON-exposed: the authored surface stays the flat [`FieldDef`](crate::fields::FieldDef)
/// struct, projected into this vocabulary by [`consequences_of`](FieldEffect::consequences_of)
/// (the module docs record the bridge). Each variant's behaviour lives in its isolated
/// sibling per-consequence file impl-ing [`ApplyFieldEffect`]; this enum's own impl
/// forwards every verb through the one mechanical `with_behaviour` delegation match, and
/// the fields mechanics invoke the trait generically — so adding a new consequence kind is
/// ONE per-consequence file + ONE variant + ONE delegation arm + ONE `mod` line (GTW-553),
/// compile-checked end to end.
///
/// Three consequences exist today: [`Drain`](FieldEffect::Drain) (the per-turn flat HP
/// drain + its lethal terminal gate), [`Immunity`](FieldEffect::Immunity) (the
/// whole-armor exemption gate), and [`Duration`](FieldEffect::Duration) (the
/// Turns/Permanent lifetime). NOT `Copy` — the [`Immunity`](FieldEffect::Immunity)
/// [`ImmuneArmorTypes`] owns a set; it is `Clone`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum FieldEffect {
    /// **Drain the occupant** a flat per-turn amount — armor-bypassing, RNG-free, lethal
    /// at an emptied pool (the GTW-544 DOT-kills precedent). Behaviour: [`ApplyDrain`].
    Drain {
        /// The flat per-turn HP the field eats from the occupant.
        damage:      FieldDamage,
        /// The damage type the field inflicts — its wheel-node flavour (presentation
        /// only; the drain BYPASSES the armor matchup, so it is no part of
        /// [`ApplyDrain`]'s behaviour — the [`Explode`](crate::effects::on_death::OnDeathEffect::Explode)
        /// elision precedent).
        damage_type: DamageType,
    },
    /// **Exempt a protected occupant** — a ganger ANY of whose worn armor pieces carries
    /// an immune [`ArmorType`](crate::armor::ArmorType) takes ZERO damage (whole-source
    /// immunity, the GTW-545 new mechanism). Behaviour: [`ApplyImmunity`].
    Immunity {
        /// The armor types that grant whole-source immunity.
        armor_types: ImmuneArmorTypes,
    },
    /// **Bound the field's lifetime** — a `Turns` field counts down one per round and is
    /// removed at zero; a `Permanent` field never expires. Behaviour: [`ApplyDuration`].
    Duration(FieldDuration),
}

impl FieldEffect {
    /// Project an authored [`FieldDef`] into its consequence list — the def→vocabulary
    /// bridge (GTW-553). The flat authored struct stays the RON surface; this projection
    /// is how the mechanics obtain the palette vocabulary GENERICALLY (one call, never a
    /// per-consequence match at a tick/spawn site).
    ///
    /// Owned (the immune set is cloned) so the caller can release its registry borrow —
    /// the same per-round snapshot cost the pre-palette tick paid.
    #[must_use]
    pub fn consequences_of(def: &FieldDef) -> Vec<Self> {
        vec![
            Self::Drain {
                damage:      def.damage,
                damage_type: def.damage_type,
            },
            Self::Immunity {
                armor_types: def.immune_armor_types.clone(),
            },
            Self::Duration(def.duration),
        ]
    }

    /// Run `visit` over this variant's isolated behaviour type — THE one delegation
    /// match over the vocabulary. Every arm is a one-line mechanical construction of the
    /// variant's [`ApplyFieldEffect`] type; NO logic lives here, and every trait verb
    /// below forwards through this single match (so adding a variant touches exactly one
    /// arm). The [`Drain`](FieldEffect::Drain) arm elides its presentation-only
    /// `damage_type` (the drain bypasses the armor matchup — the
    /// [`Explode`](crate::effects::on_death::OnDeathEffect::Explode) precedent).
    fn with_behaviour<R>(&self, visit: impl FnOnce(&dyn ApplyFieldEffect) -> R) -> R {
        match self {
            Self::Drain { damage, .. } => visit(&ApplyDrain::new(*damage)),
            Self::Immunity { armor_types } => visit(&ApplyImmunity::new(armor_types)),
            Self::Duration(duration) => visit(&ApplyDuration::new(*duration)),
        }
    }
}

impl ApplyFieldEffect for FieldEffect {
    /// The exemption gate, DELEGATED to the isolated behaviour type (only
    /// [`ApplyImmunity`] overrides the defaulted `false`).
    fn exempts_occupant(&self, armor: &OccupantArmor<'_, '_, '_>) -> bool {
        self.with_behaviour(|behaviour| behaviour.exempts_occupant(armor))
    }

    /// The per-turn occupant drain, DELEGATED to the isolated behaviour type (only
    /// [`ApplyDrain`] overrides the defaulted no-op).
    fn drain_occupant(
        &self,
        at: CellLevel,
        occupant: Entity,
        drain: &mut OccupantDrain<'_, '_, '_, '_, '_>,
    ) {
        self.with_behaviour(|behaviour| behaviour.drain_occupant(at, occupant, drain));
    }

    /// The placement-time countdown seed, DELEGATED to the isolated behaviour type (only
    /// [`ApplyDuration`] overrides the defaulted zero).
    fn initial_countdown(&self) -> FieldTurns {
        self.with_behaviour(|behaviour| behaviour.initial_countdown())
    }

    /// The per-round lifetime step, DELEGATED to the isolated behaviour type (only
    /// [`ApplyDuration`] overrides the defaulted never-expires).
    fn count_down_one_turn(&self, remaining: &mut FieldTurns) -> bool {
        self.with_behaviour(|behaviour| behaviour.count_down_one_turn(remaining))
    }
}
