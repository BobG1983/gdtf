//! The [`InjuryEffect`] vocabulary — one authored effect of an injury — and its
//! three payload newtypes ([`StatDelta`] / [`BleedAmount`] / [`MovementCostFactor`]).

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

/// The **per-step movement TU-cost multiplier** one [`InjuryEffect::MovementCostMul`]
/// applies (GTW-444 — the "Hampered" effect; `docs/combat/resolution.md` injury tables).
///
/// A factor `>= 1.0` means SLOWER (it MULTIPLIES the GTW-396 terrain per-step floor cost
/// up): `1.0` = no effect (the uninjured identity), `1.5` = each step costs 50 % more TU,
/// `2.0` = double. A no-bare-types newtype (a movement multiplier is a domain value,
/// distinct from any other `f32`): private inner + derived [`Deref`];
/// `#[serde(transparent)]` lets an authored effect name it as a bare RON number
/// (`MovementCostMul(1.5)`).
///
/// Inner `f32` (a continuous multiplier, not an integer count). It derives [`PartialEq`]
/// but NOT [`Eq`] / [`Hash`] (an `f32` has neither) — the same reason
/// [`InflictedInjuries`](super::InflictedInjuries), which now folds these into a
/// multiplicative accumulator field, drops its own `Eq` derive (the `f32` cannot be `Eq`;
/// a fixed-point rep would forfeit the exact, deterministic multiply this needs, so the
/// model keeps the `f32` and the `PartialEq`-only derive). The accumulated product folds
/// through [`InflictedInjuries::movement_cost_factor`](super::InflictedInjuries::movement_cost_factor),
/// and the pathfinder + the committed walk both scale each per-step cost by it (the
/// preview==charge consistency, GTW-444 C3).
#[derive(Deref, Clone, Copy, PartialEq, Debug, Deserialize)]
#[serde(transparent)]
pub struct MovementCostFactor(f32);

impl MovementCostFactor {
    /// The identity factor (`1.0`) — no slowdown. The default a ledger with no
    /// [`MovementCostMul`](InjuryEffect::MovementCostMul) reports, and the neutral element
    /// of the multiplicative fold.
    pub const IDENTITY: Self = Self(1.0);

    /// Build a movement-cost factor from its raw multiplier (`>= 1.0` = slower; `1.0` =
    /// no effect). A factor below `1.0` would speed the ganger up, which the "Hampered"
    /// semantics never author, but the type does not clamp — the authored data is
    /// expected to keep it `>= 1.0` (the `.injury.ron` convention).
    #[must_use]
    pub const fn new(factor: f32) -> Self {
        Self(factor)
    }

    /// The product of this factor and `other` — the multiplicative fold of two stacked
    /// [`MovementCostMul`](InjuryEffect::MovementCostMul) effects (GTW-444 C4).
    ///
    /// STACKING MULTIPLIES (the locked default — NOT additive, NOT capped): two leg
    /// injuries of `1.5` and `2.0` combine to `3.0`, NOT `3.5`. Multiplication is
    /// commutative and associative over `f32`, so the fold is order-independent and
    /// deterministic for a fixed set of factors (the GTW-441 authoring guide owes this
    /// rule to authors).
    #[must_use]
    pub fn times(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }

    /// The raw multiplier as a bare `f32` — the accessor the per-step cost scaling reads
    /// (the derived [`Deref`] also yields it; this is the explicit form the cost-scale
    /// helper uses).
    #[must_use]
    pub const fn raw(self) -> f32 {
        self.0
    }
}

impl Default for MovementCostFactor {
    /// The identity factor (`1.0`) — no slowdown (an uninjured mover, or a ledger with no
    /// [`MovementCostMul`](InjuryEffect::MovementCostMul)).
    fn default() -> Self {
        Self::IDENTITY
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
/// Four effects exist today: [`Modify`](InjuryEffect::Modify) (a modifier-layer
/// delta re-summed by the projector, hitting BOTH attributes and derived stats),
/// [`Bleeding`](InjuryEffect::Bleeding) (a separate per-turn HP drain, NOT a
/// `Modify`), [`DisableHand`](InjuryEffect::DisableHand) (GTW-443 — disables the
/// hand on the injury's struck arm, folded into the read-derived hand count, NOT a
/// stored delta), and [`MovementCostMul`](InjuryEffect::MovementCostMul) (GTW-444 — the
/// "Hampered" per-step movement-TU multiplier, folded into a dedicated MULTIPLICATIVE
/// accumulator, NOT a summed delta). Future variants (Stun / Knockback / `MoraleHit` /
/// Disarm) are each a new variant plus one match arm.
///
/// This enum derives [`PartialEq`] but NOT [`Eq`] — the
/// [`MovementCostMul`](InjuryEffect::MovementCostMul) payload is an `f32`
/// ([`MovementCostFactor`]), which is not `Eq`. Tests compare effects with `matches!` /
/// `==` (`PartialEq`), never as a `HashSet`/`BTreeSet` key.
#[derive(Clone, Copy, PartialEq, Debug, Deserialize)]
pub enum InjuryEffect {
    /// Shift a stat by a signed [`StatDelta`] — a modifier-layer delta the projector
    /// re-sums every projection (so a `stat.tuning.ron` hot-reload re-applies it
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
    /// **Multiply the per-step movement TU cost** by a [`MovementCostFactor`] (GTW-444 —
    /// the "Hampered" effect). A factor `>= 1.0` SLOWS the ganger (each step costs more
    /// TU); `1.0` is no effect.
    ///
    /// Unlike [`Modify`](InjuryEffect::Modify) (which SUMS into the per-stat delta store)
    /// and [`Bleeding`](InjuryEffect::Bleeding) (which SUMS into the bleed accrual), this
    /// folds MULTIPLICATIVELY into the ledger's dedicated
    /// [`movement`](super::InflictedInjuries) accumulator field — two stacked factors
    /// MULTIPLY (`1.5 × 2.0 = 3.0`), they do NOT add. The accumulated product is read by
    /// [`InflictedInjuries::movement_cost_factor`](super::InflictedInjuries::movement_cost_factor),
    /// and BOTH the pathfinder cost-function (move-range / path preview) and the committed
    /// walk's per-step TU charge scale each step by it — so a Hampered unit's previewed
    /// path cost equals the TU actually charged (GTW-444 C3, preview==charge).
    ///
    /// "Hampered" is the player-facing status term; the authored injury's wound NAME
    /// (e.g. "Shattered Knee") carries the flavor, while `MovementCostMul` is the neutral
    /// code name. A `MovementCostMul` is part-agnostic — it slows the ganger regardless of
    /// which body part the injury struck (a Leg injury is the natural author, but the
    /// fold does not key on the part).
    MovementCostMul(MovementCostFactor),
}
