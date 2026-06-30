//! The melee **spawn-side** shapes — the [`MeleeWeaponBundle`] that spawns an armed
//! melee-weapon entity (GTW-505, child GTW-37a of the GTW-37 melee epic).
//!
//! It MIRRORS the ranged [`WeaponBundle`](super::super::WeaponBundle): it REUSES the
//! shared damage newtypes verbatim (the [`WeaponName`] / the [`WeaponDamage`] /
//! [`WeaponPunch`] / [`WeaponShred`] / [`DamageType`] damage group / [`FatalBias`] /
//! [`Handedness`]), DROPS every ranged-only handling field (`base_spread` / `accuracy`
//! / `kickback` / [`Magazine`](crate::magazine::Magazine) / [`Stable`](super::super::Stable)),
//! and ADDS the melee-only [`Reach`] + [`FightMode`] plus the [`MeleeWeapon`] marker
//! (instead of the ranged [`Weapon`](super::super::Weapon) marker), so a ranged lookup
//! can exclude it (GTW-505 C5).

use bevy::prelude::Bundle;

use super::{FightMode, MeleeWeapon, Reach};
use crate::weapon::{
    DamageType, FatalBias, Handedness, WeaponDamage, WeaponName, WeaponPunch, WeaponShred,
};

/// The **spawn bundle for an armed melee-weapon entity** — the [`MeleeWeapon`] marker
/// plus the full set of melee weapon stat components, inserted together (GTW-505), the
/// melee mirror of the ranged [`WeaponBundle`](super::super::WeaponBundle).
///
/// A Bevy [`Bundle`] so spawning an armed melee-weapon entity carries the marker and
/// every stat component in one `commands.spawn(...)` / `entity.insert(...)` call. It
/// SHARES the ranged damage model — the [`WeaponName`], the [`WeaponDamage`] /
/// [`WeaponPunch`] / [`WeaponShred`] / [`DamageType`] damage group, the [`FatalBias`],
/// and the [`Handedness`] — and adds the melee-only [`Reach`] + [`FightMode`]; it
/// carries NONE of the ranged-only handling fields (no `BaseSpread` / `Accuracy` /
/// `Kickback` / `Magazine` / `Stable`). Every field is a weapon newtype, the
/// [`WeaponName`], the [`FightMode`] selector, or the [`MeleeWeapon`] marker (no bare
/// primitive); build one with [`MeleeWeaponBundle::new`].
///
/// **Not `Copy`** — it carries a [`WeaponName`] ([`String`]) and a [`FightMode`] (which
/// holds a `Vec`); the bundle is `Clone`.
#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct MeleeWeaponBundle {
    /// The [`MeleeWeapon`] marker tagging the entity as an armed melee weapon (so a
    /// ranged lookup excludes it — GTW-505 C5).
    pub marker:      MeleeWeapon,
    /// The weapon's human-facing name (the shared ranged newtype, its own sibling component).
    pub name:        WeaponName,
    /// The base damage a strike deals before armor (the shared ranged newtype).
    pub damage:      WeaponDamage,
    /// The armor protection a strike ignores — penetration (the shared ranged newtype).
    pub punch:       WeaponPunch,
    /// The extra integrity damage a strike deals to armor durability (the shared ranged newtype).
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node (the shared ranged newtype).
    pub damage_type: DamageType,
    /// The severity-score addend, consumed by E3 (the shared ranged newtype).
    pub fatal_bias:  FatalBias,
    /// The weapon's [`Handedness`] (the shared ranged newtype) — `OneHanded` / `TwoHanded`.
    pub handedness:  Handedness,
    /// The melee-only [`Reach`] — how many cells away a strike can land (GTW-505).
    pub reach:       Reach,
    /// The melee-only [`FightMode`] selector and its per-mode numbers (GTW-505).
    pub fight_mode:  FightMode,
}

/// A melee weapon's **damage block** for spawning — the three per-hit damage numbers
/// plus the emitted [`DamageType`], handed to [`MeleeWeaponBundle::new`] as one
/// cohesive value (the ranged [`DamageProfile`](super::super::DamageProfile) precedent;
/// the melee weapon SHARES the ranged damage model verbatim).
///
/// An owned ctor-input grouping so [`MeleeWeaponBundle::new`] stays under clippy's
/// argument-count gate. Every field is a shared-with-ranged weapon-number newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeDamageProfile {
    /// The base damage a strike deals before armor.
    pub damage:      WeaponDamage,
    /// The armor protection a strike ignores — penetration.
    pub punch:       WeaponPunch,
    /// The extra integrity damage a strike deals to armor durability.
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: DamageType,
}

impl MeleeDamageProfile {
    /// Build a melee damage block from the three per-hit numbers and the emitted
    /// [`DamageType`] (all magnitudes TBD tuning).
    #[must_use]
    pub const fn new(
        damage: WeaponDamage,
        punch: WeaponPunch,
        shred: WeaponShred,
        damage_type: DamageType,
    ) -> Self {
        Self {
            damage,
            punch,
            shred,
            damage_type,
        }
    }
}

impl MeleeWeaponBundle {
    /// Build an armed melee-weapon-entity bundle — the [`MeleeWeapon`] marker is
    /// supplied automatically; the stats are handed in as the weapon's [`WeaponName`],
    /// a [`MeleeDamageProfile`], the §6 [`FatalBias`], the [`Handedness`], the melee
    /// [`Reach`], and the [`FightMode`] selector.
    ///
    /// Takes the cohesive damage group (the [`DamageProfile`](super::super::DamageProfile)
    /// precedent) rather than a dozen loose params, keeping the ctor under clippy's
    /// argument-count gate while every stat lands as its own component on the spawned
    /// entity.
    #[must_use]
    pub const fn new(
        name: WeaponName,
        damage: MeleeDamageProfile,
        fatal_bias: FatalBias,
        handedness: Handedness,
        reach: Reach,
        fight_mode: FightMode,
    ) -> Self {
        Self {
            marker: MeleeWeapon,
            name,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            fatal_bias,
            handedness,
            reach,
            fight_mode,
        }
    }
}
