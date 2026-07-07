//! The **Turns/Permanent duration** field consequence (GTW-545; GTW-553
//! one-file-per-consequence) — the [`FieldTurns`] payload newtype, the [`FieldDuration`]
//! lifetime enum, and the isolated [`ApplyDuration`] behaviour that seeds a placement's
//! countdown and counts it down one turn per round (a `Permanent` field never expires).

use std::num::NonZeroU8;

use bevy::prelude::Deref;
use serde::Deserialize;

use super::ApplyFieldEffect;

/// The **duration in turns** a [`FieldDuration::Turns`] field lingers — how many turns the
/// zone ticks before it is removed (`docs/combat/resolution.md` — the area-damage-field beat).
///
/// A field duration NUMBER (a small POSITIVE turn count — a field lasts a handful of turns,
/// and a zero-turn field is UNREPRESENTABLE: GTW-659, the [`crate::weapon::DotTurns`]
/// precedent — pre-GTW-659 an authored `Turns(0)` parsed and then behaved as `Turns(1)`, the
/// authored value lying by one). A no-bare-types newtype wrapping [`NonZeroU8`]: private
/// inner + derived [`Deref`]; `#[serde(transparent)]` so a field's authored
/// [`FieldDuration::Turns`] `.ron` names it as a bare integer — and an authored `0` FAILS
/// DESERIALIZATION LOUDLY (serde's [`NonZeroU8`] impl rejects it), flowing through the
/// per-file salvage into a `MalformedFile` finding; there is NO silent clamp-to-1. Distinct
/// from every [`Tu`](crate::ganger::Tu)-domain count — this is a count of TURNS the field
/// runs, not a TU cost, and distinct from [`crate::weapon::DotTurns`] (the DOT clock).
/// Derives [`Ord`] so the placement seed can pick the lifetime consequence's countdown out
/// of the consequence fold ([`PlacedField::from_def`](crate::effects::fields::PlacedField::from_def)).
///
/// A bare `0` literal cannot even be spelled (unrepresentable by type):
///
/// ```compile_fail
/// use gdtf_battle_sim::effects::fields::FieldTurns;
/// // A zero-turn field duration is a TYPE error, not a runtime state.
/// let zero = FieldTurns::new(0);
/// ```
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct FieldTurns(NonZeroU8);

impl FieldTurns {
    /// Build a field duration from its (necessarily positive) turn count.
    #[must_use]
    pub const fn new(turns: NonZeroU8) -> Self {
        Self(turns)
    }

    /// One turn ticked off — `Some` of the shortened duration while turns remain, or
    /// `None` when this was the LAST turn. Expiry is **decrement-or-REMOVE** (the
    /// [`crate::weapon::DotTurns::decremented`] shape): the `None` arm means "the
    /// placement is expired — remove it now" — a zero-valued duration is never stored.
    #[must_use]
    pub const fn decremented(self) -> Option<Self> {
        match NonZeroU8::new(self.0.get() - 1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

/// How long an area-damage field persists — a fixed number of [`FieldTurns`], or forever.
///
/// The lifetime arm of the field model (`docs/combat/resolution.md` — the area-damage-field
/// beat): a [`Turns`](FieldDuration::Turns) field counts down one turn per
/// [`tick_fields`](crate::effects::fields::tick_fields) round and is removed at zero (a thrown gas
/// grenade's dissipating cloud); a [`Permanent`](FieldDuration::Permanent) field NEVER expires
/// (a toxic-waste pool seeded as fixed terrain). A named domain enum (no-bare-types: a field
/// lifetime is a domain value, not a bare `Option<u8>`). Behaviour: [`ApplyDuration`].
///
/// Derives [`Deserialize`] so an authored `.ron` writes `duration: Turns(3)` or
/// `duration: Permanent`. Deliberately NO [`Default`] (GTW-659): the old sentinel default
/// (`Turns(0)` — "never authored") had ZERO consumers — an authored `.ron` must spell its
/// `duration:` (no `#[serde(default)]`), and nothing in the workspace constructed a
/// defaulted [`FieldDef`](crate::effects::fields::FieldDef) — so the magic zero is simply
/// gone; a "never authored" duration is unrepresentable, not a sentinel value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum FieldDuration {
    /// The field ticks for exactly this many turns, then is removed.
    Turns(FieldTurns),
    /// The field never expires — a fixed terrain hazard (a toxic-waste pool).
    Permanent,
}

/// **Duration** — the Turns/Permanent lifetime field consequence (GTW-545; isolated per
/// GTW-553).
///
/// Owns BOTH lifetime verbs: the placement-time countdown seed
/// ([`initial_countdown`](ApplyFieldEffect::initial_countdown) — a
/// [`Turns`](FieldDuration::Turns) field starts at its authored count, a
/// [`Permanent`](FieldDuration::Permanent) field carries NO countdown at all) and the
/// per-round expiry step ([`count_down_one_turn`](ApplyFieldEffect::count_down_one_turn) —
/// a `Turns` field decrements-or-EXPIRES one per round, GTW-659; a `Permanent` field never
/// counts down and never expires).
pub struct ApplyDuration {
    /// The authored lifetime this consequence enforces.
    duration: FieldDuration,
}

impl ApplyDuration {
    /// Build the lifetime behaviour from its authored duration.
    #[must_use]
    pub const fn new(duration: FieldDuration) -> Self {
        Self { duration }
    }
}

impl ApplyFieldEffect for ApplyDuration {
    /// The countdown a fresh placement starts from: `Some` of the authored
    /// [`Turns`](FieldDuration::Turns) count, or `None` for a
    /// [`Permanent`](FieldDuration::Permanent) field — which carries NO countdown (GTW-659:
    /// the old zero was a magic sentinel; "no countdown" is now explicit).
    fn initial_countdown(&self) -> Option<FieldTurns> {
        match self.duration {
            FieldDuration::Turns(turns) => Some(turns),
            FieldDuration::Permanent => None,
        }
    }

    /// Decrement-or-EXPIRE the countdown one turn (GTW-659, the
    /// [`DotTurns`](crate::weapon::DotTurns) decrement-or-remove shape) for a
    /// [`Turns`](FieldDuration::Turns) field; a [`Permanent`](FieldDuration::Permanent)
    /// field is left untouched. Returns `true` iff the placement is now EXPIRED — a
    /// `Turns` field whose countdown spent its LAST round (a zero-valued countdown is
    /// never stored between rounds; a finite field that somehow carries no countdown is
    /// expired fail-closed). A `Permanent` field always returns `false`.
    fn count_down_one_turn(&self, remaining: &mut Option<FieldTurns>) -> bool {
        match self.duration {
            FieldDuration::Turns(_) => match (*remaining).and_then(FieldTurns::decremented) {
                Some(next) => {
                    *remaining = Some(next);
                    false
                }
                None => true,
            },
            // A Permanent field never counts down and never expires.
            FieldDuration::Permanent => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyDuration, FieldDuration};
    use crate::{effects::fields::ApplyFieldEffect, test_support::field_turns};

    /// GTW-659: an authored `Turns(0)` FAILS deserialization loudly — the serde
    /// [`std::num::NonZeroU8`] impl rejects it at the parse, so a zero-turn
    /// [`FieldDuration`] can never come out of a `.ron` (no silent clamp-to-1; the full
    /// loader-path rejection is pinned by `gdtf_app/tests/load_ref_salvage.rs`).
    #[test]
    fn an_authored_zero_turn_duration_fails_deserialization() {
        let parsed = ron::from_str::<FieldDuration>("Turns(0)");
        assert!(
            parsed.is_err(),
            "an authored zero-turn field duration must FAIL the parse (got {parsed:?})",
        );
    }

    /// The identity control: a POSITIVE authored turn count parses, and the parsed
    /// duration is the authored count (no clamp, no offset).
    #[test]
    fn an_authored_positive_turn_count_parses_unchanged() {
        let parsed = ron::from_str::<FieldDuration>("Turns(3)");
        assert_eq!(
            parsed.ok(),
            Some(FieldDuration::Turns(field_turns(3))),
            "a positive authored turn count must parse to exactly itself",
        );
    }

    /// A Turns field seeds its countdown at the authored count; a Permanent field
    /// carries NO countdown (`None` — explicit, not a magic zero; GTW-659).
    #[test]
    fn the_initial_countdown_is_the_authored_turns_count() {
        assert_eq!(
            ApplyDuration::new(FieldDuration::Turns(field_turns(3))).initial_countdown(),
            Some(field_turns(3)),
            "a Turns field starts at its authored count"
        );
        assert_eq!(
            ApplyDuration::new(FieldDuration::Permanent).initial_countdown(),
            None,
            "a Permanent field carries no countdown"
        );
    }

    /// A Turns field decrements one per round and expires exactly when the countdown
    /// spends its LAST round — decrement-or-EXPIRE, never storing a zero (GTW-659).
    #[test]
    fn a_turns_field_counts_down_and_expires_on_its_last_round() {
        let lifetime = ApplyDuration::new(FieldDuration::Turns(field_turns(2)));
        let mut remaining = lifetime.initial_countdown();
        assert!(
            !lifetime.count_down_one_turn(&mut remaining),
            "2 → 1: not yet expired"
        );
        assert_eq!(
            remaining,
            Some(field_turns(1)),
            "the countdown decremented one turn"
        );
        assert!(
            lifetime.count_down_one_turn(&mut remaining),
            "1 → expired: the round that spends the last turn expires the placement"
        );
    }

    /// A Permanent field never counts down and never expires, however many rounds tick —
    /// its countdown stays the explicit `None` throughout.
    #[test]
    fn a_permanent_field_never_counts_down_and_never_expires() {
        let lifetime = ApplyDuration::new(FieldDuration::Permanent);
        let mut remaining = lifetime.initial_countdown();
        for _ in 0..10 {
            assert!(
                !lifetime.count_down_one_turn(&mut remaining),
                "a Permanent field never expires"
            );
        }
        assert_eq!(
            remaining, None,
            "a Permanent field never touches the countdown"
        );
    }
}
