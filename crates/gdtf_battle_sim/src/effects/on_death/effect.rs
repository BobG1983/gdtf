//! The closed **on-death effect vocabulary** — the [`OnDeathEffect`] a weapon / gear /
//! cover def authors (GTW-547, child GTW-41g; GTW-552 re-homes it into the
//! [`effects`](crate::effects) palette).
//!
//! ## The serde name↔type bridge + effect isolation (GTW-552)
//!
//! This vocabulary is a closed, serde-round-tripping enum (RON cannot deserialize trait
//! objects, so the on-disk shape is this closed enum keyed by variant name). Each variant's
//! BEHAVIOUR is a CONCEPTUALLY-ISOLATED type in its OWN sibling file impl-ing the
//! [`ApplyOnDeathEffect`] trait (its one verb IS its behaviour — no central logic `match`,
//! no per-variant folder-fn in the resolver, no authoring step scattered across the tree),
//! and this enum's own [`ApplyOnDeathEffect`] impl is the ONE purely-mechanical delegation
//! match — the ONLY sim-side match over this vocabulary. The resolver
//! ([`resolve_on_death`](crate::on_death::resolve_on_death)) invokes the trait generically.
//!
//! Adding a new effect means ONE new per-effect file + ONE variant here + ONE delegation
//! arm + ONE `mod` line — compile-checked (the delegation match is exhaustive, so a new
//! variant without an arm is a build error, never a silent no-op or a denied panic).

use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

use super::{ApplyExplode, ApplyLeaveField, ApplyOnDeathEffect, DeathFanOut, ExplodeDamage};
use crate::{
    fields::FieldKey,
    metric::CellLevel,
    weapon::{DamageType, HitType},
};

/// The **closed on-death effect** a dying source (weapon / gear / cover) fans at the death
/// `(cell, level)` (GTW-547, child GTW-41g; palette-isolated per GTW-552).
///
/// A named domain enum (no-bare-types: an on-death effect is a domain value). RON-authored
/// per weapon / gear (the [`WeaponSpec::on_death`](crate::weapon::WeaponSpec) field) and per
/// cover tile (the [`TerrainDef::on_death`](crate::terrain::def::TerrainDef) field). Each
/// variant's behaviour lives in its isolated sibling per-effect file impl-ing
/// [`ApplyOnDeathEffect`]; this enum's own impl forwards its one verb through the one
/// mechanical delegation match, and [`resolve_on_death`](crate::on_death::resolve_on_death)
/// invokes the trait generically — so adding a new effect kind is ONE per-effect file + ONE
/// variant + ONE delegation arm + ONE `mod` line (GTW-552), compile-checked end to end.
///
/// Each variant REUSES an existing sim type where one fits (no parallels invented):
/// [`HitType`] (the GTW-541 [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected)
/// template — its radius / range / angle already newtyped) + [`DamageType`] (the seven-node
/// wheel) for [`Explode`](OnDeathEffect::Explode), and [`FieldKey`] (the GTW-545 field
/// catalog key — the ticket's `FieldDefRef`) for [`LeaveField`](OnDeathEffect::LeaveField).
/// NOT `Copy` — the [`LeaveField`](OnDeathEffect::LeaveField) [`FieldKey`] owns a `String`;
/// it is `Clone`.
///
/// Derives [`Serialize`] / [`Deserialize`] so both authoring homes round-trip through RON,
/// and [`TypePath`] because it rides the reflected [`WeaponSpec`](crate::weapon::WeaponSpec)
/// / [`TerrainDef`](crate::terrain::def::TerrainDef) asset payloads (the same bound they
/// satisfy).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, TypePath)]
pub enum OnDeathEffect {
    /// **Fan an `AoE` blast** at the death cell — every ganger in the GTW-541
    /// [`aoe_affected`](crate::shot_pipeline::aoe::aoe_affected) template's radius takes a
    /// flat, deterministic [`ExplodeDamage`] drain (armor-bypassing, no RNG, the DOT /
    /// field-tick drain model). A blast that empties a victim's
    /// [`Hp`](crate::ganger::Hp) KILLS it, pushing the fresh
    /// [`OnDeathOccurred`](crate::on_death::OnDeathOccurred) onto the resolver's cascade
    /// work-queue. Behaviour: [`ApplyExplode`].
    Explode {
        /// The `AoE` template shape (GTW-541) — `Blast{radius}` / `Cone{range,angle}` /
        /// `Line{range}` (a `Single` degenerates to just the death cell). The blast is
        /// centred at the death cell (which is also its notional shooter origin, so a cone
        /// falls back to the full disc — no meaningful fire direction from a corpse).
        hit_type:    HitType,
        /// The flat per-cell HP the blast drains from each ganger in the radius.
        damage:      ExplodeDamage,
        /// The damage type the blast inflicts — its wheel-node flavour (presentation only;
        /// the drain BYPASSES the armor matchup, the field-tick precedent — so it is not
        /// part of [`ApplyExplode`]'s behaviour).
        damage_type: DamageType,
    },
    /// **Leave a persistent field** at the death cell — spawn the referenced GTW-545 field
    /// ([`FieldRegistry::spawn`](crate::fields::FieldRegistry::spawn)) so the cell becomes
    /// a live hazard that persists + ticks per GTW-545 rules (a fuel barrel leaving burning
    /// ground when it is smashed). Behaviour: [`ApplyLeaveField`].
    LeaveField {
        /// The field catalog KEY (the ticket's `FieldDefRef`) resolved against the
        /// [`FieldDefRegistry`](crate::fields::FieldDefRegistry) to the
        /// [`FieldDef`](crate::fields::FieldDef) spawned at the death cell.
        field: FieldKey,
    },
}

impl ApplyOnDeathEffect for OnDeathEffect {
    /// Fan this effect at the death cell by DELEGATING to the isolated behaviour type in
    /// its sibling per-effect file (the effect-isolation architecture).
    ///
    /// This `match` carries NO logic — every arm is a one-line mechanical delegation that
    /// constructs the variant's isolated [`ApplyOnDeathEffect`] type from its payload and
    /// forwards the call (the [`Explode`](OnDeathEffect::Explode) arm elides its
    /// presentation-only `damage_type`, which is no part of the drain behaviour). The
    /// BEHAVIOUR (the blast drain discipline, the cascade push, the fail-closed field
    /// spawn) lives entirely in that isolated type, so adding an effect touches ONE variant
    /// + ONE per-effect file + this ONE delegation line — never a central logic branch.
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>) {
        match self {
            Self::Explode {
                hit_type, damage, ..
            } => ApplyExplode::new(*hit_type, *damage).fan_at(at, fan_out),
            Self::LeaveField { field } => ApplyLeaveField::new(field).fan_at(at, fan_out),
        }
    }
}
