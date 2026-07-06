//! The **Turns/Permanent duration** field consequence (GTW-545; GTW-553
//! one-file-per-consequence) — the [`FieldTurns`] payload newtype, the [`FieldDuration`]
//! lifetime enum, and the isolated [`ApplyDuration`] behaviour that seeds a placement's
//! countdown and counts it down one turn per round (a `Permanent` field never expires).

use bevy::prelude::Deref;
use serde::Deserialize;

use super::ApplyFieldEffect;

/// The **duration in turns** a [`FieldDuration::Turns`] field lingers — how many turns the
/// zone ticks before it is removed (`docs/combat/resolution.md` — the area-damage-field beat).
///
/// A field duration NUMBER (a small turn count, `u8` — a field lasts a handful of turns). A
/// no-bare-types newtype: private inner + derived [`Deref`]; `#[serde(transparent)]` so a
/// field's authored [`FieldDuration::Turns`] `.ron` names it as a bare integer. Distinct from
/// every [`Tu`](crate::ganger::Tu)-domain count — this is a count of TURNS the field runs, not
/// a TU cost, and distinct from [`crate::weapon::DotTurns`] (the DOT clock). Derives
/// [`Ord`] so the placement seed can pick the lifetime consequence's countdown out of the
/// consequence fold ([`PlacedField::from_def`](crate::effects::fields::PlacedField::from_def)).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Deserialize)]
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
/// [`tick_fields`](crate::effects::fields::tick_fields) round and is removed at zero (a thrown gas
/// grenade's dissipating cloud); a [`Permanent`](FieldDuration::Permanent) field NEVER expires
/// (a toxic-waste pool seeded as fixed terrain). A named domain enum (no-bare-types: a field
/// lifetime is a domain value, not a bare `Option<u8>`). Behaviour: [`ApplyDuration`].
///
/// Derives [`Deserialize`] so an authored `.ron` writes `duration: Turns(3)` or
/// `duration: Permanent`. Derives the sentinel [`Default`] ([`Turns`](FieldDuration::Turns)
/// of zero — an immediately-expiring no-op) so a [`FieldDef`](crate::effects::fields::FieldDef)
/// composes cleanly; the default is never authored (a real field authors either a positive
/// `Turns` count or `Permanent`).
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
    /// [`Permanent`](FieldDuration::Permanent); the default exists only so
    /// [`FieldDef`](crate::effects::fields::FieldDef) derives [`Default`] cleanly.
    fn default() -> Self {
        Self::Turns(FieldTurns::new(0))
    }
}

/// **Duration** — the Turns/Permanent lifetime field consequence (GTW-545; isolated per
/// GTW-553).
///
/// Owns BOTH lifetime verbs: the placement-time countdown seed
/// ([`initial_countdown`](ApplyFieldEffect::initial_countdown) — a
/// [`Turns`](FieldDuration::Turns) field starts at its authored count, a
/// [`Permanent`](FieldDuration::Permanent) field at zero, which it never reads) and the
/// per-round expiry step ([`count_down_one_turn`](ApplyFieldEffect::count_down_one_turn) —
/// a `Turns` field decrements one per round, saturating, and expires at zero; a
/// `Permanent` field never counts down and never expires).
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
    /// The countdown a fresh placement starts from: the authored
    /// [`Turns`](FieldDuration::Turns) count, or zero for a
    /// [`Permanent`](FieldDuration::Permanent) field (which never reads it).
    fn initial_countdown(&self) -> FieldTurns {
        match self.duration {
            FieldDuration::Turns(turns) => turns,
            FieldDuration::Permanent => FieldTurns::new(0),
        }
    }

    /// Decrement the countdown one turn (saturating at zero — no underflow) for a
    /// [`Turns`](FieldDuration::Turns) field; a [`Permanent`](FieldDuration::Permanent)
    /// field is left untouched. Returns `true` iff the placement is now EXPIRED (a
    /// `Turns` field whose countdown reached zero); a `Permanent` field always returns
    /// `false`.
    fn count_down_one_turn(&self, remaining: &mut FieldTurns) -> bool {
        match self.duration {
            FieldDuration::Turns(_) => {
                *remaining = FieldTurns::new(remaining.saturating_sub(1));
                **remaining == 0
            }
            // A Permanent field never counts down and never expires.
            FieldDuration::Permanent => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplyDuration, FieldDuration, FieldTurns};
    use crate::effects::fields::ApplyFieldEffect;

    /// A Turns field seeds its countdown at the authored count; a Permanent field at
    /// zero (which it never reads).
    #[test]
    fn the_initial_countdown_is_the_authored_turns_count() {
        assert_eq!(
            *ApplyDuration::new(FieldDuration::Turns(FieldTurns::new(3))).initial_countdown(),
            3,
            "a Turns field starts at its authored count"
        );
        assert_eq!(
            *ApplyDuration::new(FieldDuration::Permanent).initial_countdown(),
            0,
            "a Permanent field carries no countdown"
        );
    }

    /// A Turns field decrements one per round and expires exactly when the countdown
    /// reaches zero (saturating — a further round stays at zero).
    #[test]
    fn a_turns_field_counts_down_and_expires_at_zero() {
        let lifetime = ApplyDuration::new(FieldDuration::Turns(FieldTurns::new(2)));
        let mut remaining = lifetime.initial_countdown();
        assert!(
            !lifetime.count_down_one_turn(&mut remaining),
            "2 → 1: not yet expired"
        );
        assert_eq!(*remaining, 1, "the countdown decremented one turn");
        assert!(
            lifetime.count_down_one_turn(&mut remaining),
            "1 → 0: expired exactly when the count runs out"
        );
        assert_eq!(*remaining, 0, "the countdown floors at zero (saturating)");
    }

    /// A Permanent field never counts down and never expires, however many rounds tick.
    #[test]
    fn a_permanent_field_never_counts_down_and_never_expires() {
        let lifetime = ApplyDuration::new(FieldDuration::Permanent);
        let mut remaining = FieldTurns::new(0);
        for _ in 0..10 {
            assert!(
                !lifetime.count_down_one_turn(&mut remaining),
                "a Permanent field never expires"
            );
        }
        assert_eq!(
            *remaining, 0,
            "a Permanent field never touches the countdown"
        );
    }
}
