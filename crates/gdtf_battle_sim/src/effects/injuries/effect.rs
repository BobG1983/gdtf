//! The closed **injury-effect vocabulary** — the [`InjuryEffect`] an
//! [`InjuryDef`](crate::injuries::InjuryDef) lists, one entry per authored effect
//! (GTW-405; GTW-550 re-homes it into the [`effects`](crate::effects) palette).
//!
//! ## The serde name↔type bridge + effect isolation (GTW-550)
//!
//! This vocabulary is a closed, serde-deserializable enum (RON cannot deserialize trait
//! objects, so the on-disk shape is this closed enum keyed by variant name). Each
//! variant's BEHAVIOUR is a CONCEPTUALLY-ISOLATED type in its OWN sibling file impl-ing
//! the [`ApplyInjuryEffect`] trait (its verbs ARE its behaviour — no central logic
//! `match`, no ledger arm, no authoring step scattered across the tree), and this
//! enum's own [`ApplyInjuryEffect`] impl delegates every verb through the ONE
//! purely-mechanical `with_behaviour` match — the ONLY sim-side match over this
//! vocabulary. GTW-550 IS the "registry seam" the GTW-437 spike deferred: the palette
//! trait replaces the central exhaustive match that made every new effect a multi-file
//! ritual.
//!
//! Adding a new effect means ONE new per-effect file + ONE variant here + ONE
//! delegation arm + ONE `mod` line — compile-checked (the delegation match is
//! exhaustive, so a new variant without an arm is a build error, never a silent
//! no-op or a denied panic).

use serde::Deserialize;

use super::{
    ApplyBleeding, ApplyDisableHand, ApplyInjuryEffect, ApplyModify, ApplyMovementCostMul,
    BleedAmount, HealError, LedgerAccumulators, MovementCostFactor, StatDelta,
};
use crate::injuries::StatTarget;

/// One **effect** of an injury — the atomic, authored mutation an injury carries
/// (`docs/combat/resolution.md` injury tables). An [`InjuryDef`](crate::injuries::InjuryDef)
/// carries a `Vec<InjuryEffect>` (≥ 1; an injury MAY carry more than one).
///
/// A serde enum so authoring is human-modifiable (the RON name↔type bridge). Each
/// variant's behaviour lives in its isolated sibling per-effect file impl-ing
/// [`ApplyInjuryEffect`]; this enum's own impl forwards every verb through the one
/// mechanical `with_behaviour` delegation match, and the ledger's
/// [`gain`](crate::injuries::InflictedInjuries::gain) invokes the trait generically —
/// so adding a new effect kind is ONE per-effect file + ONE variant + ONE delegation
/// arm + ONE `mod` line (GTW-550), compile-checked end to end.
///
/// Four effects exist today: [`Modify`](InjuryEffect::Modify) (a modifier-layer
/// delta re-summed by the projector, hitting BOTH attributes and derived stats),
/// [`Bleeding`](InjuryEffect::Bleeding) (a separate per-turn HP drain, NOT a
/// `Modify`), [`DisableHand`](InjuryEffect::DisableHand) (GTW-443 — disables the
/// hand on the injury's struck arm, folded into the read-derived hand count, NOT a
/// stored delta), and [`MovementCostMul`](InjuryEffect::MovementCostMul) (GTW-444 — the
/// "Hampered" per-step movement-TU multiplier, folded into a dedicated MULTIPLICATIVE
/// accumulator, NOT a summed delta). Future variants (Stun / Knockback / `MoraleHit` /
/// Disarm) are each ONE new per-effect palette file + one variant + one delegation arm
/// + one `mod` line.
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
    /// derived stats; for a pool target it docks the MAX. Behaviour:
    /// [`ApplyModify`].
    Modify {
        /// Which stat this delta shifts (attribute or derived).
        stat:   StatTarget,
        /// The signed shift (debuff negative, boost positive).
        amount: StatDelta,
    },
    /// Inflict a per-turn HP bleed of [`BleedAmount`] — accrued onto the ledger's
    /// bleed total and drained each turn by the existing bleed runtime. Behaviour:
    /// [`ApplyBleeding`].
    Bleeding {
        /// The per-turn HP drained while this injury persists.
        amount: BleedAmount,
    },
    /// **Disable the hand** on the injury's struck arm (GTW-443) — a FIELDLESS
    /// variant, INERT at gain and surfaced through the read-side hand projection
    /// (the disabled SIDE derives from the entry's struck part, never from the
    /// effect). Behaviour: [`ApplyDisableHand`].
    DisableHand,
    /// **Multiply the per-step movement TU cost** by a [`MovementCostFactor`]
    /// (GTW-444 — the "Hampered" effect; `>= 1.0` slows, `1.0` is no effect; stacked
    /// factors MULTIPLY). Behaviour: [`ApplyMovementCostMul`].
    MovementCostMul(MovementCostFactor),
}

impl InjuryEffect {
    /// Run `visit` over this variant's isolated behaviour type — THE one delegation
    /// match over the vocabulary (P2/P3). Every arm is a one-line mechanical
    /// construction of the variant's [`ApplyInjuryEffect`] type; NO logic lives here,
    /// and every trait verb below forwards through this single match (so adding a
    /// variant touches exactly one arm).
    fn with_behaviour<R>(self, visit: impl FnOnce(&dyn ApplyInjuryEffect) -> R) -> R {
        match self {
            Self::Modify { stat, amount } => visit(&ApplyModify::new(stat, amount)),
            Self::Bleeding { amount } => visit(&ApplyBleeding::new(amount)),
            Self::DisableHand => visit(&ApplyDisableHand),
            Self::MovementCostMul(factor) => visit(&ApplyMovementCostMul::new(factor)),
        }
    }
}

impl ApplyInjuryEffect for InjuryEffect {
    /// Fold this effect into the ledger's accumulators by DELEGATING to its isolated
    /// behaviour type (via the one `with_behaviour` match — no logic here).
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>) {
        self.with_behaviour(|behaviour| behaviour.fold_on_gain(accumulators));
    }

    /// The read-side hand projection, DELEGATED to the isolated behaviour type (only
    /// [`ApplyDisableHand`] overrides the defaulted `false`).
    fn disables_hand(&self) -> bool {
        self.with_behaviour(|behaviour| behaviour.disables_hand())
    }

    /// Heal this effect out of the accumulators by DELEGATING to its isolated
    /// behaviour type (via the one `with_behaviour` match — no logic here).
    fn heal(&self, accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError> {
        self.with_behaviour(|behaviour| behaviour.heal(accumulators))
    }
}
