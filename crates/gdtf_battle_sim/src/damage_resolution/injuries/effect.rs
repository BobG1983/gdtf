//! The [`InjuryEffect`] vocabulary — one authored effect of an injury — and its
//! two payload newtypes ([`StatDelta`] / [`BleedAmount`]).

use bevy::prelude::Deref;
use serde::Deserialize;

use super::StatTarget;

/// The signed stat delta one [`InjuryEffect::Modify`] applies to a
/// [`StatTarget`] (`docs/combat/resolution.md` injury tables).
///
/// Normally a debuff (negative — an injury weakens the ganger), but a boost
/// (positive) is legal, so the inner is **signed** `i8` (`-128..=127`, ample for any
/// authored single-stat shift). A no-bare-types newtype (a stat shift is a domain
/// value, never a bare integer): private inner + derived [`Deref`];
/// `#[serde(transparent)]` lets an authored effect name it as a bare RON number
/// (`amount: -2`). The summed-delta store widens to `i16` so many stacked deltas
/// can never overflow — see [`StatDeltaSum`](super::StatDeltaSum).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(transparent)]
pub struct StatDelta(i8);

impl StatDelta {
    /// Build a stat delta from its signed magnitude (negative = debuff, positive =
    /// boost).
    #[must_use]
    pub const fn new(amount: i8) -> Self {
        Self(amount)
    }

    /// The signed magnitude as a bare `i8` — a `const` accessor the summed-delta
    /// fold needs (the derived [`Deref`] is not usable in a `const fn`).
    #[must_use]
    pub const fn raw(self) -> i8 {
        self.0
    }
}

/// The per-turn HP a [`InjuryEffect::Bleeding`] injury drains, through the existing
/// bleed runtime (`docs/combat/resolution.md` §9).
///
/// An unsigned `u8` (a bleed only ever drains; the per-turn amount is small). A
/// no-bare-types newtype (a bleed rate is a domain value): private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON number (`amount: 1`). The
/// accrued total widens to `u16` so stacked bleeds can never overflow — see
/// [`BleedAfflicted`](super::BleedAfflicted). Distinct from the Downed Wounds
/// bleed-out: this drains HP and can down but never kill.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(transparent)]
pub struct BleedAmount(u8);

impl BleedAmount {
    /// Build a per-turn bleed amount from its magnitude (HP drained each turn).
    #[must_use]
    pub const fn new(amount: u8) -> Self {
        Self(amount)
    }

    /// The magnitude as a bare `u8` — a `const` accessor the bleed accrual needs
    /// (the derived [`Deref`] is not usable in a `const fn`).
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.0
    }
}

/// One **effect** of an injury — the atomic, authored mutation an injury carries
/// (`docs/combat/resolution.md` injury tables). An [`InjuryDef`](super::InjuryDef)
/// carries a `Vec<InjuryEffect>` (≥ 1; an injury MAY carry more than one).
///
/// A serde enum so authoring is human-modifiable. The apply boundary (GTW-437)
/// matches it EXHAUSTIVELY — adding a new effect kind is "add a variant + add one
/// match arm", compile-checked (never a silent no-op or a denied panic). The
/// intermediate command-registry enum the spike considered is deliberately
/// collapsed into that one match; reintroduce it only when a third effect needs a
/// genuinely new mutation kind.
///
/// Three effects exist today: [`Modify`](InjuryEffect::Modify) (a modifier-layer
/// delta re-summed by the projector, hitting BOTH attributes and derived stats),
/// [`Bleeding`](InjuryEffect::Bleeding) (a separate per-turn HP drain, NOT a
/// `Modify`), and [`DisableHand`](InjuryEffect::DisableHand) (GTW-443 — disables the
/// hand on the injury's struck arm, folded into the read-derived hand count, NOT a
/// stored delta). Future variants (Stun / Knockback / `MoraleHit` / Disarm) are each a
/// new variant plus one match arm.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
pub enum InjuryEffect {
    /// Shift a stat by a signed [`StatDelta`] — a modifier-layer delta the projector
    /// re-sums every projection (so a `stat_tuning.ron` hot-reload re-applies it
    /// rather than wiping it). Applies to BOTH the eight attributes and the eight
    /// derived stats; for a pool target it docks the MAX.
    Modify {
        /// Which stat this delta shifts (attribute or derived).
        stat:   StatTarget,
        /// The signed shift (debuff negative, boost positive).
        amount: StatDelta,
    },
    /// Inflict a per-turn HP bleed of [`BleedAmount`] — accrued onto
    /// [`BleedAfflicted`](super::BleedAfflicted) and drained each turn by the
    /// existing bleed runtime. NOT a [`Modify`](InjuryEffect::Modify): it drains the
    /// current HP pool, distinct from any stat ceiling.
    Bleeding {
        /// The per-turn HP drained while this injury persists.
        amount: BleedAmount,
    },
    /// **Disable the hand** on the injury's struck arm (GTW-443) — a FIELDLESS variant.
    ///
    /// The disabled SIDE is NOT carried here; it is derived from the
    /// [`GainedInjury::part`](super::GainedInjury::part) already on each ledger entry
    /// ([`LeftArm`](crate::armor::BodyPart::LeftArm) → the left hand,
    /// [`RightArm`](crate::armor::BodyPart::RightArm) → the right hand; a
    /// `DisableHand` carried by a Head / Torso / Leg injury is INERT — there is no hand
    /// to disable). Unlike [`Modify`](InjuryEffect::Modify) / [`Bleeding`](InjuryEffect::Bleeding),
    /// the ledger's `gain` accumulates NOTHING for this variant: the hand count is read
    /// by FOLDING [`gained`](super::InflictedInjuries::gained) on demand
    /// ([`InflictedInjuries::hands_available`](super::InflictedInjuries::hands_available)),
    /// not by docking a stored stat — so two same-side `DisableHand` injuries still
    /// disable exactly ONE hand (a set over distinct arm-sides, not a count), the
    /// single-source-of-truth that a stored counter could not give without de-duping.
    /// The 1H aim PENALTY a hand-disabling injury also carries rides as a SEPARATE
    /// [`Modify`](InjuryEffect::Modify)`(Shooting, -N)` effect in the same injury's
    /// effects `Vec` — so the penalty flows through the normal modifier layer while this
    /// variant only gates two-handed fire.
    DisableHand,
}
