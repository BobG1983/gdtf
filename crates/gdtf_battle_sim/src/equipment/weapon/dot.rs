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
//!   `Option<DotProfile>` sibling component (like the [`Silenced`](super::Silenced)
//!   attachment tag), so a weapon without a DOT profile is byte-identical to before this slice.
//! - [`Dot`] is the **battle-state** side — the `{ remaining_turns, per_turn_damage,
//!   damage_type }` component the fire path attaches (or REFRESHES) onto a struck ganger
//!   when a penetrating hit lands. The per-turn drain + terminal gate is
//!   [`tick_dot`](crate::effects::dot::tick_dot); this module owns only the two data
//!   types (no math, no world access).
//!
//! **Zero turns is UNREPRESENTABLE** (GTW-643 user ruling: "It doesn't make sense for a
//! dot to have a length of zero"): [`DotTurns`] wraps [`NonZeroU8`], so an inert-from-creation
//! DOT cannot exist — an authored `turns: 0` FAILS DESERIALIZATION LOUDLY (the per-file
//! salvage rejects the file into the `ContentIntegrityReport`; no silent clamp-to-1), and
//! runtime expiry is decrement-or-REMOVE ([`DotTurns::decremented`] — a zero-valued [`Dot`]
//! is never stored between ticks).
//!
//! **Refresh-not-stack** ([`Dot::refresh_from`]): a fresh penetrating DOT hit RESETS the
//! affliction's `remaining_turns` + `per_turn_damage` to the new profile — DOTs do not
//! stack (`docs/combat/resolution.md` — the DOT beat). A fully-soaked hit (penetrating
//! damage `0`) attaches NOTHING, even though the hit may still bruise HP.

use std::num::NonZeroU8;

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
/// A DOT duration NUMBER (a small POSITIVE turn count — a DOT lasts a handful of turns,
/// and a zero-turn DOT is UNREPRESENTABLE: GTW-643's user ruling, "it doesn't make sense
/// for a dot to have a length of zero"). A no-bare-types newtype wrapping [`NonZeroU8`]:
/// private inner + derived [`Deref`]; `#[serde(transparent)]` so a weapon's authored
/// [`DotProfile`] `.ron` names it as a bare integer — and an authored `0` FAILS
/// DESERIALIZATION LOUDLY (serde's [`NonZeroU8`] impl rejects it), flowing through the
/// per-file salvage into a `MalformedFile` finding; there is NO silent clamp-to-1.
/// Distinct from every [`Tu`](crate::ganger::Tu)-domain count — this is a count of TURNS
/// the clock runs, not a TU cost.
///
/// A bare `0` literal cannot even be spelled (GTW-643 — unrepresentable by type):
///
/// ```compile_fail
/// use gdtf_battle_sim::weapon::DotTurns;
/// // A zero-turn DOT duration is a TYPE error, not a runtime state.
/// let zero = DotTurns::new(0);
/// ```
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct DotTurns(NonZeroU8);

impl DotTurns {
    /// Build a DOT duration from its (necessarily positive) turn count.
    #[must_use]
    pub const fn new(turns: NonZeroU8) -> Self {
        Self(turns)
    }

    /// One turn ticked off — `Some` of the shortened duration while turns remain, or
    /// `None` when this was the LAST turn. Expiry is **decrement-or-REMOVE** (GTW-643):
    /// the `None` arm means "remove the [`Dot`] now" — a zero-valued duration is never
    /// stored.
    #[must_use]
    pub const fn decremented(self) -> Option<Self> {
        match NonZeroU8::new(self.0.get() - 1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
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
/// same seven wheel nodes), and the [`DotTurns`] duration (positive by construction —
/// GTW-643). Derives [`Deserialize`] so the loose `.ron` parses it — an authored
/// `turns: 0` fails the parse loudly — and [`Component`] so it rides on the weapon entity.
/// NOT a stat the matchup wheel touches — the per-turn tick bypasses armor entirely, so the
/// `DamageType` here is a presentation / flavour tag, never a soak lookup.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct DotProfile {
    /// The per-turn HP damage each tick deals (bypasses armor).
    pub damage:      DotDamage,
    /// The damage type the DOT inflicts — the weapon's wheel-node flavour (presentation).
    pub damage_type: DamageType,
    /// How many turns the DOT lingers before it is removed (positive by construction).
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

impl Default for DotProfile {
    /// The sentinel the GTW-542 `template_value` spawn seam requires (`Clone + Default +
    /// Unpin`): a zero-damage, MINIMAL one-turn no-op profile — one turn, not zero,
    /// because a zero-turn duration is unrepresentable (GTW-643). Never authored and never
    /// attached (an omitted `dot:` field is `None`, not a default profile); hand-written
    /// because [`NonZeroU8`] has no `Default`.
    fn default() -> Self {
        Self {
            damage:      DotDamage::default(),
            damage_type: DamageType::default(),
            turns:       DotTurns::new(NonZeroU8::MIN),
        }
    }
}

/// A ganger's **active damage-over-time affliction** — the `{ remaining_turns,
/// per_turn_damage, damage_type }` a penetrating hit from a DOT weapon attached (GTW-544).
///
/// The battle-state side of the DOT model: a `#[derive(Component)]` the fire path attaches
/// (or REFRESHES) on a struck ganger when a hit PENETRATES armor
/// ([`PenetratingDamage`](crate::resolve_hit::PenetratingDamage) `> 0`) from a weapon
/// carrying a [`DotProfile`]. Each turn [`tick_dot`](crate::effects::dot::tick_dot)
/// decrements the ganger's [`Hp`](crate::ganger::Hp) DIRECTLY by
/// [`per_turn_damage`](Dot::per_turn_damage) (no armor matchup, no injury roll, no RNG),
/// then DECREMENTS-or-REMOVES the affliction ([`DotTurns::decremented`], GTW-643): the
/// tick that spends the last turn removes the component outright, so a zero-valued `Dot`
/// never exists between ticks — [`remaining_turns`](Dot::remaining_turns) is positive by
/// construction ([`NonZeroU8`] inner).
///
/// **Refresh-not-stack** ([`refresh_from`](Dot::refresh_from)): a second penetrating DOT
/// hit RESETS the affliction to the new profile — DOTs do not stack. A `Copy` value object
/// of named domain types (no bare primitive); NOT [`Deserialize`] (battle-state, never
/// authored). Carries the sentinel [`Default`] (via [`DotProfile::default`] — a
/// zero-damage, one-turn no-op) only for the `bsn!` spawn path's bounds; a `Dot` is only
/// ever attached at runtime, never spawn-seeded.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot {
    /// How many more turns this DOT ticks before it is removed (positive by construction —
    /// the expiring tick REMOVES the component instead of storing zero).
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
}

impl Default for Dot {
    /// The sentinel `Default` the `bsn!` spawn path's bounds would need (it never does —
    /// a `Dot` is only ever attached at runtime): the [`DotProfile::default`] no-op
    /// affliction (zero damage, the minimal ONE turn — zero turns is unrepresentable,
    /// GTW-643). Hand-written because [`NonZeroU8`] has no `Default`.
    fn default() -> Self {
        Self::from_profile(DotProfile::default())
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroU8;

    use super::{DotProfile, DotTurns};

    /// GTW-643 (C2): an authored `turns: 0` FAILS deserialization loudly — the serde
    /// [`NonZeroU8`] impl rejects it at the parse, so a zero-turn [`DotProfile`] can never
    /// come out of a `.ron` (no silent clamp-to-1; the full loader-path rejection is
    /// pinned by `gdtf_app/tests/load_ref_salvage.rs`).
    #[test]
    fn an_authored_zero_turn_count_fails_deserialization() {
        let parsed = ron::from_str::<DotProfile>("(damage: 4, damage_type: Plasma, turns: 0)");
        assert!(
            parsed.is_err(),
            "an authored zero-turn DOT profile must FAIL the parse (got {parsed:?})",
        );
    }

    /// The identity control: the same profile with a POSITIVE turn count parses, and the
    /// parsed duration is the authored count (no clamp, no offset).
    #[test]
    fn an_authored_positive_turn_count_parses_unchanged() {
        let parsed = ron::from_str::<DotProfile>("(damage: 4, damage_type: Plasma, turns: 3)");
        assert_eq!(
            parsed.ok().map(|profile| profile.turns.get()),
            Some(3),
            "a positive authored turn count must parse to exactly itself",
        );
    }

    /// GTW-643 (C3): expiry is decrement-or-REMOVE — [`DotTurns::decremented`] steps
    /// `3 → 2 → 1 → None`; the `None` arm is the removal signal, and no zero-valued
    /// duration is ever produced.
    #[test]
    fn decremented_counts_down_to_removal_never_zero() {
        let three = NonZeroU8::new(3).map(DotTurns::new);
        let two = three.and_then(DotTurns::decremented);
        let one = two.and_then(DotTurns::decremented);
        let expired = one.and_then(DotTurns::decremented);
        assert_eq!(
            (
                two.map(|t| t.get()),
                one.map(|t| t.get()),
                expired.map(|t| t.get())
            ),
            (Some(2), Some(1), None),
            "decremented() must step 3 → 2 → 1 → None (remove), never storing zero",
        );
    }
}
