//! The **spawn-side** weapon shapes — the [`WeaponBundle`] that spawns an armed
//! entity, the owned ctor-input groupings ([`DamageProfile`] / [`HandlingProfile`])
//! it takes, and the transient [`WeaponStats`] borrow-view the §1/§6 readers take
//! (GTW-200).

use bevy::prelude::Bundle;

use super::{
    Accuracy, BaseSpread, DamageType, FatalBias, FireMode, Kickback, Stable, Weapon, WeaponDamage,
    WeaponName, WeaponPunch, WeaponShred,
};
use crate::magazine::Magazine;

/// A transient **borrow-view** of a weapon's stats — refs assembled at the call
/// site from the individual weapon components, the read-shape the §1/§6 readers
/// take in place of the old `&Weapon` data struct (GTW-200).
///
/// This is **NOT** a stored `Component` — it is a short-lived bundle of borrows a
/// caller (a Bevy system, or the E4.5 `fire()` act) builds from a queried entity's
/// weapon stat components, exactly as [`crate::aim::Shooter`] /
/// [`crate::resolve_and_apply::TargetGanger`] do for ganger state. Grouping the
/// refs keeps [`crate::resolve_and_apply::resolve_and_apply`] and
/// [`crate::aim::cone_for`] under clippy's argument-count gate while letting the
/// weapon be plain ECS components. Every field is a borrowed weapon-number newtype
/// (no bare primitive); the view itself is a transparent borrow record, never a
/// wrapped domain scalar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponStats<'a> {
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread: &'a BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:    &'a Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:    &'a Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:  &'a FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:      &'a WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:       &'a WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:       &'a WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: &'a DamageType,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally
    /// (regardless of faced cover / stance); `false` is a normal weapon (braces
    /// only when the faced cover suits the stance).
    pub stable:      &'a Stable,
}

/// The **spawn bundle for an armed entity** — the [`Weapon`] marker plus the full
/// set of weapon stat components, inserted together (GTW-200).
///
/// A Bevy [`Bundle`] so spawning an armed (ganger) entity carries the marker and
/// every stat component in one `commands.spawn(...)` / `entity.insert(...)` call.
/// The ammo state is the [`crate::magazine::Magazine`] grouping component (GTW-275:
/// the [`MagazineSize`](super::MagazineSize) capacity, the per-weapon
/// [`ReloadTu`](crate::magazine::ReloadTu), and the live
/// [`LoadedRounds`](crate::magazine::LoadedRounds) count) — spawned FULL by
/// [`WeaponSpec::into_bundle`](super::WeaponSpec::into_bundle). Every field is a
/// weapon-number newtype, the [`Magazine`] grouping, the [`WeaponName`], or the marker
/// (no bare primitive); build one with [`WeaponBundle::new`].
///
/// **Not `Copy`** — it carries a [`WeaponName`] ([`String`]) and a [`FireMode`]
/// (which holds a `Vec`); the bundle is `Clone`.
#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct WeaponBundle {
    /// The [`Weapon`] marker tagging the entity as armed.
    pub marker:      Weapon,
    /// The weapon's human-facing name (its own sibling component).
    pub name:        WeaponName,
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread: BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:    Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:    Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:  FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:      WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:       WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: DamageType,
    /// The ammo state — the [`Magazine`] grouping (capacity + per-weapon reload cost +
    /// the live loaded-rounds count, spawned full).
    pub magazine:    Magazine,
    /// The authored fire-mode selector and its per-mode numbers.
    pub fire_mode:   FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:      Stable,
}

/// A weapon's **damage block** for spawning — the three per-hit damage numbers plus
/// the emitted [`DamageType`] (`weapons-and-armor.md` §"Weapon stats" / matchup.md
/// §"The 7 types"), handed to [`WeaponBundle::new`] as one cohesive value.
///
/// An owned ctor-input grouping (the [`FireModeSpec`](super::FireModeSpec)
/// precedent: related data travels as one named record, not a loose tuple) so
/// [`WeaponBundle::new`] stays under clippy's argument-count gate. Distinct from the
/// borrow-view [`WeaponStats`]: this owns its four newtypes (spawn-side input), the
/// view borrows the live components (read-side). Every field is a weapon-number
/// newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageProfile {
    /// The base damage a hit deals before armor.
    pub damage:      WeaponDamage,
    /// The armor protection a hit ignores — penetration.
    pub punch:       WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability.
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: DamageType,
}

impl DamageProfile {
    /// Build a damage block from the three per-hit numbers and the emitted
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

/// A weapon's **handling block** for spawning — its [`Magazine`] grouping (capacity +
/// per-weapon reload cost + the spawn-full loaded count), authored [`FireMode`]
/// selector, and [`Stable`] tag, handed to [`WeaponBundle::new`] as one cohesive
/// value.
///
/// An owned ctor-input grouping (the [`DamageProfile`] /
/// [`FireModeSpec`](super::FireModeSpec) precedent) so [`WeaponBundle::new`] stays
/// under clippy's argument-count gate. Every field is a weapon-number grouping / the
/// [`FireMode`] selector.
///
/// **Not `Copy`** — it owns a [`FireMode`] (which holds a `Vec` of specs); it is
/// `Clone`.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingProfile {
    /// The ammo state — the [`Magazine`] grouping (capacity + reload cost + loaded
    /// count).
    pub magazine:  Magazine,
    /// The authored fire-mode selector and its per-mode numbers.
    pub fire_mode: FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:    Stable,
}

impl HandlingProfile {
    /// Build a handling block from a weapon's [`Magazine`] grouping, fire-mode
    /// selector, and `stable` tag.
    #[must_use]
    pub const fn new(magazine: Magazine, fire_mode: FireMode, stable: Stable) -> Self {
        Self {
            magazine,
            fire_mode,
            stable,
        }
    }
}

impl WeaponBundle {
    /// Build an armed-entity bundle from a weapon's full stat set — the [`Weapon`]
    /// marker is supplied automatically; the stats are handed in as the weapon's
    /// [`WeaponName`], the §1/§6 cone/severity numbers, a [`DamageProfile`], and a
    /// [`HandlingProfile`].
    ///
    /// Takes the cohesive groups (the [`DamageProfile`] / [`HandlingProfile`]
    /// precedent) rather than a dozen loose params, keeping the ctor under clippy's
    /// argument-count gate while every stat lands as its own component on the
    /// spawned entity.
    #[must_use]
    pub fn new(
        name: WeaponName,
        base_spread: BaseSpread,
        accuracy: Accuracy,
        kickback: Kickback,
        fatal_bias: FatalBias,
        damage: DamageProfile,
        handling: HandlingProfile,
    ) -> Self {
        Self {
            marker: Weapon,
            name,
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            magazine: handling.magazine,
            fire_mode: handling.fire_mode,
            stable: handling.stable,
        }
    }

    /// Assemble a transient [`WeaponStats`] borrow-view over this bundle's stat
    /// components — the read-shape the §1/§6 readers take. A convenience for callers
    /// holding a whole bundle; a query-based system assembles a [`WeaponStats`] from
    /// its individually-queried components instead.
    #[must_use]
    pub const fn stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
        }
    }
}
