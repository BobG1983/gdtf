//! The closed [`OnDeathEffect`] enum + its [`OnDeath`] authoring component + the
//! [`ExplodeDamage`] payload newtype (GTW-547, child GTW-41g).
//!
//! An on-death effect is RON-authored per weapon / gear (as the
//! [`WeaponSpec::on_death`](crate::weapon::WeaponSpec) field, carried onto the wielded-weapon
//! entity as the [`OnDeath`] sibling component) and per cover tile (as the
//! [`TerrainDef::on_death`](crate::terrain::def::TerrainDef) field, seeded into the
//! [`CoverOnDeathRegistry`](super::CoverOnDeathRegistry) keyed by cell). The enum is CLOSED
//! and designed to be extended: add a variant + a per-variant folder function
//! (the `explode` / `leave_field` folder fns in the `resolve` submodule) = done.

use bevy::{
    prelude::{Component, Deref},
    reflect::TypePath,
};
use serde::{Deserialize, Serialize};

use crate::{
    fields::FieldKey,
    weapon::{DamageType, HitType},
};

/// The **flat per-cell HP damage** an [`OnDeathEffect::Explode`] blast deals to each ganger in
/// its fan-out radius (GTW-547).
///
/// A blast damage NUMBER (a small count, `u16` to match the [`Hp`](crate::ganger::Hp) inner).
/// A no-bare-types newtype: private inner + derived [`Deref`]; `#[serde(transparent)]` so an
/// authored [`OnDeathEffect::Explode`] `.ron` names it as a bare integer. Distinct from
/// [`WeaponDamage`](crate::weapon::WeaponDamage) (a signed-`i32` spawn sentinel that runs the
/// full §5/§6 armor+wound fold) — an on-death blast is a DETERMINISTIC, armor-bypassing,
/// RNG-free direct drain (the [`DotDamage`](crate::weapon::DotDamage) /
/// [`FieldDamage`](crate::fields::FieldDamage) precedent), so its magnitude is its OWN
/// positive quantity, keyed to the exploding source.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ExplodeDamage(u16);

impl ExplodeDamage {
    /// Build a per-cell blast damage from its count.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// The **closed on-death effect** a dying source (weapon / gear / cover) fans at the death
/// `(cell, level)` (GTW-547, child GTW-41g).
///
/// A named domain enum (no-bare-types: an on-death effect is a domain value). RON-authored per
/// weapon / gear (the [`WeaponSpec::on_death`](crate::weapon::WeaponSpec) field) and per cover
/// tile (the [`TerrainDef::on_death`](crate::terrain::def::TerrainDef) field). CLOSED and
/// designed to be EXTENDED — add a variant here + a per-variant folder function in the
/// `resolve` submodule (the `explode` / `leave_field` folder fns), and the resolver's
/// exhaustive match wires it up.
///
/// Each variant REUSES an existing sim type where one fits (no parallels invented): [`HitType`]
/// (the GTW-541 [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) template — its
/// radius / range / angle already newtyped) + [`DamageType`] (the seven-node wheel) for
/// [`Explode`](OnDeathEffect::Explode), and [`FieldKey`] (the GTW-545 field catalog key — the
/// ticket's `FieldDefRef`) for [`LeaveField`](OnDeathEffect::LeaveField). NOT `Copy` — the
/// [`LeaveField`](OnDeathEffect::LeaveField) [`FieldKey`] owns a `String`; it is `Clone`.
///
/// Derives [`Serialize`] / [`Deserialize`] so both authoring homes round-trip through RON, and
/// [`TypePath`] because it rides the reflected [`WeaponSpec`](crate::weapon::WeaponSpec) /
/// [`TerrainDef`](crate::terrain::def::TerrainDef) asset payloads (the same bound they satisfy).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub enum OnDeathEffect {
    /// **Fan an `AoE` blast** at the death cell — every ganger in the GTW-541
    /// [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) template's radius takes a flat,
    /// deterministic [`ExplodeDamage`] drain (armor-bypassing, no RNG, the DOT / field-tick
    /// drain model). A blast that empties a victim's [`Hp`](crate::ganger::Hp) KILLS it,
    /// re-emitting [`OnDeathOccurred`](super::OnDeathOccurred) for the cascade.
    Explode {
        /// The `AoE` template shape (GTW-541) — `Blast{radius}` / `Cone{range,angle}` /
        /// `Line{range}` (a `Single` degenerates to just the death cell). The blast is
        /// centred at the death cell (which is also its notional shooter origin, so a cone
        /// falls back to the full disc — no meaningful fire direction from a corpse).
        hit_type:    HitType,
        /// The flat per-cell HP the blast drains from each ganger in the radius.
        damage:      ExplodeDamage,
        /// The damage type the blast inflicts — its wheel-node flavour (presentation only;
        /// the drain BYPASSES the armor matchup, the field-tick precedent).
        damage_type: DamageType,
    },
    /// **Leave a persistent field** at the death cell — spawn the referenced GTW-545 field
    /// ([`FieldRegistry::spawn`](crate::fields::FieldRegistry::spawn)) so the cell becomes a
    /// live hazard that persists + ticks per GTW-545 rules (a fuel barrel leaving burning
    /// ground when it is smashed).
    LeaveField {
        /// The field catalog KEY (the ticket's `FieldDefRef`) resolved against the
        /// [`FieldDefRegistry`](crate::fields::FieldDefRegistry) to the
        /// [`FieldDef`](crate::fields::FieldDef) spawned at the death cell.
        field: FieldKey,
    },
}

/// The **authoring component** carrying a weapon / gear entity's [`OnDeathEffect`] (GTW-547).
///
/// Composed onto the wielded-weapon entity at the wielded-weapon scene seam (the GTW-544
/// [`Dot`](crate::weapon::Dot) sibling precedent) from the authored
/// [`WeaponSpec::on_death`](crate::weapon::WeaponSpec) field. When the WIELDING ganger dies,
/// [`resolve_on_death`](super::resolve_on_death) reads this off the ganger's
/// [`Wields`](crate::weapon::Wields) weapon entity and fans its effect at the death cell —
/// modelling "the dying ganger's live grenade / unstable power cell detonates". Cover on-death
/// lives in the [`CoverOnDeathRegistry`](super::CoverOnDeathRegistry) (cover is not an entity),
/// NOT this component.
///
/// A newtype [`Component`] over the domain [`OnDeathEffect`] (no-bare-types: the wrapped value
/// is a domain enum). Private inner + derived [`Deref`]; derives [`Default`] (an
/// [`OnDeathEffect::LeaveField`] with an empty key — a spawn-seed sentinel that never fires:
/// the seam only composes this component when the spec authored a real effect) so the
/// `template_value` sibling seam's `Clone + Default` bound is satisfied.
#[derive(Component, Deref, Debug, Clone, PartialEq)]
pub struct OnDeath(OnDeathEffect);

impl OnDeath {
    /// Build an on-death component from its authored effect.
    #[must_use]
    pub const fn new(effect: OnDeathEffect) -> Self {
        Self(effect)
    }

    /// Borrow the authored [`OnDeathEffect`] — the resolver reads this to pick the per-variant
    /// folder function.
    #[must_use]
    pub const fn effect(&self) -> &OnDeathEffect {
        &self.0
    }
}

impl Default for OnDeath {
    /// The spawn-seed sentinel: a [`LeaveField`](OnDeathEffect::LeaveField) with an empty
    /// [`FieldKey`] (never resolves to a field, so it fires nothing). Never authored — the
    /// wielded-weapon scene seam composes this component ONLY when the weapon spec authored a
    /// real [`on_death`](crate::weapon::WeaponSpec) effect; the default exists only so the
    /// `template_value` sibling composition's `Default` bound is satisfied (the
    /// [`DotProfile`](crate::weapon::DotProfile) default-sentinel precedent).
    fn default() -> Self {
        Self(OnDeathEffect::LeaveField {
            field: FieldKey::new(String::new()),
        })
    }
}
