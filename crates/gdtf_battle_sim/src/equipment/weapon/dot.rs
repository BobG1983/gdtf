//! The **damage-over-time** model (GTW-544, child GTW-41e) — the weapon-side
//! [`DotProfile`] a DOT weapon carries and the per-ganger [`Dot`] affliction a
//! penetrating hit from such a weapon attaches.
//!
//! A DOT weapon (a chem sprayer, a plasma torch) does not spend all its harm at the
//! instant of the hit: on a hit that **penetrates armor** it seeds a lingering
//! affliction that eats the struck ganger's [`Hp`](crate::ganger::Hp) a flat amount per
//! turn for a fixed number of turns (`docs/combat/resolution.md` — the DOT beat of
//! GTW-41). The two shapes here split cleanly along the model/runtime line:
//!
//! - [`DotProfile`] is the **weapon authoring** side — the `{ damage, DamageType, turns }`
//!   an `assets/content/weapons/ranged/*.weapon.ron` optionally authors (the `dot:` field
//!   on [`WeaponSpec`](super::WeaponSpec)). It rides on the armed entity as an
//!   `Option<DotProfile>` sibling component (like the GTW-542 [`Scoped`](super::Scoped)
//!   attachment), so a weapon without a DOT profile is byte-identical to before this slice.
//! - [`Dot`] is the **battle-state** side — the `{ remaining_turns, per_turn_damage,
//!   damage_type }` component the fire path attaches (or REFRESHES) onto a struck ganger
//!   when a penetrating hit lands. The per-turn drain + terminal gate is
//!   [`tick_dot`](crate::acts_runtime::dot::tick_dot); this module owns only the two data
//!   types (no math, no world access).
//!
//! **Refresh-not-stack** ([`Dot::refresh_from`]): a fresh penetrating DOT hit RESETS the
//! affliction's `remaining_turns` + `per_turn_damage` to the new profile — DOTs do not
//! stack (`docs/combat/resolution.md` — the DOT beat). A fully-soaked hit (penetrating
//! damage `0`) attaches NOTHING, even though the hit may still bruise HP.

use bevy::prelude::{Component, Deref};
use serde::Deserialize;

use super::DamageType;

/// The **per-turn HP damage** a [`Dot`] deals each turn it ticks — the flat amount the
/// affliction eats from the struck ganger's [`Hp`](crate::ganger::Hp) pool every turn,
/// bypassing armor entirely (`docs/combat/resolution.md` — the DOT beat: "decrements the
/// ganger HP DIRECTLY").
///
/// A DOT damage NUMBER (a small per-turn count, `u16` to match the [`Hp`](crate::ganger::Hp)
/// inner). A no-bare-types newtype: private inner + derived [`Deref`];
/// `#[serde(transparent)]` so a weapon's authored [`DotProfile`] `.ron` names it as a bare
/// integer. Distinct from [`crate::weapon::WeaponDamage`] (the at-impact damage) — a DOT's
/// per-turn tick is its OWN quantity, never the weapon's base damage.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct DotDamage(u16);

impl DotDamage {
    /// Build a per-turn DOT damage from its count.
    #[must_use]
    pub const fn new(damage: u16) -> Self {
        Self(damage)
    }
}

/// The **duration in turns** a [`Dot`] lingers — how many turns the affliction ticks
/// before it is removed (`docs/combat/resolution.md` — the DOT beat: "for the profile
/// turn count").
///
/// A DOT duration NUMBER (a small turn count, `u8` — a DOT lasts a handful of turns). A
/// no-bare-types newtype: private inner + derived [`Deref`]; `#[serde(transparent)]` so a
/// weapon's authored [`DotProfile`] `.ron` names it as a bare integer. Distinct from every
/// [`Tu`](crate::ganger::Tu)-domain count — this is a count of TURNS the clock runs, not a
/// TU cost.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct DotTurns(u8);

impl DotTurns {
    /// Build a DOT duration from its turn count.
    #[must_use]
    pub const fn new(turns: u8) -> Self {
        Self(turns)
    }
}

/// A weapon's **damage-over-time profile** — the `{ damage, DamageType, turns }` a DOT
/// weapon carries, seeding a lingering [`Dot`] on a penetrating hit (GTW-544).
///
/// The weapon-authoring side of the DOT model: an `Option<DotProfile>` sibling component on
/// the armed entity (the `dot:` field on [`WeaponSpec`](super::WeaponSpec), `#[serde(default)]`
/// so an omitted field is `None` — a non-DOT weapon, byte-identical to before this slice).
/// When a shot from a DOT weapon lands a hit that PENETRATES armor
/// ([`PenetratingDamage`](crate::resolve_hit::PenetratingDamage) `> 0`), the fire path
/// builds a [`Dot`] from this profile and attaches (or REFRESHES) it on the struck ganger.
/// A fully-soaked hit attaches nothing.
///
/// A `Copy` value object of named domain types (no bare primitive): the per-turn
/// [`DotDamage`], the [`DamageType`] the DOT inflicts (reused — a DOT is a flavour of the
/// same seven wheel nodes), and the [`DotTurns`] duration. Derives [`Deserialize`] so the
/// loose `.ron` parses it, and [`Component`] so it rides on the weapon entity. NOT a
/// stat the matchup wheel touches — the per-turn tick bypasses armor entirely, so the
/// `DamageType` here is a presentation / flavour tag, never a soak lookup.
/// Derives the sentinel [`Default`] (a zero-damage, zero-turn no-op profile) so the GTW-542
/// `template_value` spawn seam (`Clone + Default + Unpin`) composes it as an optional
/// weapon-entity sibling; the `Default` is never authored (an omitted `dot:` field is `None`,
/// not a default profile).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
pub struct DotProfile {
    /// The per-turn HP damage each tick deals (bypasses armor).
    pub damage:      DotDamage,
    /// The damage type the DOT inflicts — the weapon's wheel-node flavour (presentation).
    pub damage_type: DamageType,
    /// How many turns the DOT lingers before it is removed.
    pub turns:       DotTurns,
}

impl DotProfile {
    /// Build a DOT profile from its per-turn damage, damage type, and turn count.
    #[must_use]
    pub const fn new(damage: DotDamage, damage_type: DamageType, turns: DotTurns) -> Self {
        Self {
            damage,
            damage_type,
            turns,
        }
    }
}

/// A ganger's **active damage-over-time affliction** — the `{ remaining_turns,
/// per_turn_damage, damage_type }` a penetrating hit from a DOT weapon attached (GTW-544).
///
/// The battle-state side of the DOT model: a `#[derive(Component)]` the fire path attaches
/// (or REFRESHES) on a struck ganger when a hit PENETRATES armor
/// ([`PenetratingDamage`](crate::resolve_hit::PenetratingDamage) `> 0`) from a weapon
/// carrying a [`DotProfile`]. Each turn [`tick_dot`](crate::acts_runtime::dot::tick_dot)
/// decrements the ganger's [`Hp`](crate::ganger::Hp) DIRECTLY by
/// [`per_turn_damage`](Dot::per_turn_damage) (no armor matchup, no injury roll, no RNG),
/// decrements [`remaining_turns`](Dot::remaining_turns), and removes the component when it
/// reaches zero.
///
/// **Refresh-not-stack** ([`refresh_from`](Dot::refresh_from)): a second penetrating DOT
/// hit RESETS the affliction to the new profile — DOTs do not stack. A `Copy` value object
/// of named domain types (no bare primitive); NOT [`Deserialize`] (battle-state, never
/// authored) but derives the sentinel [`Default`] the `bsn!` spawn path would need if a
/// ganger ever spawned with one seeded (it never does — a DOT is only ever attached at
/// runtime; the `Default` is a harmless zero-turn no-op affliction).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Dot {
    /// How many more turns this DOT ticks before it is removed.
    pub remaining_turns: DotTurns,
    /// The flat HP damage each remaining tick deals (bypasses armor).
    pub per_turn_damage: DotDamage,
    /// The DOT's damage-type flavour (presentation — the tick bypasses the matchup wheel).
    pub damage_type:     DamageType,
}

impl Dot {
    /// Build an active DOT affliction from a weapon's [`DotProfile`] — the ATTACH shape
    /// (its full `turns` remain, its `damage` is the per-turn tick).
    #[must_use]
    pub const fn from_profile(profile: DotProfile) -> Self {
        Self {
            remaining_turns: profile.turns,
            per_turn_damage: profile.damage,
            damage_type:     profile.damage_type,
        }
    }

    /// **Refresh** this affliction from a fresh penetrating DOT hit — reset both the
    /// remaining turns AND the per-turn damage to the new `profile` (refresh-not-stack:
    /// a second DOT hit does NOT extend or add to the existing affliction, it REPLACES it;
    /// `docs/combat/resolution.md` — the DOT beat).
    pub const fn refresh_from(&mut self, profile: DotProfile) {
        *self = Self::from_profile(profile);
    }

    /// Whether this DOT still has a turn left to tick — `remaining_turns > 0`
    /// (the terminal gate [`tick_dot`](crate::acts_runtime::dot::tick_dot) removes the
    /// component on).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.remaining_turns.0 > 0
    }
}
