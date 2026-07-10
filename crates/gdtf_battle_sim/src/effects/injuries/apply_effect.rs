//! The **shared apply contract** for the injury-effect palette — the
//! [`ApplyInjuryEffect`] trait whose verbs ARE an effect's behaviour, the mutable
//! [`LedgerAccumulators`] fold surface the ledger lends it, and the [`HealError`] a
//! non-invertible fold reports (GTW-550, the injury sibling of the GTW-558
//! attachment palette).
//!
//! Every isolated per-effect `ApplyX` type (one per file in this module) impls this
//! trait; the closed [`InjuryEffect`](super::InjuryEffect) serde enum impls it too, by
//! THIN mechanical delegation to each variant's isolated type. The mechanics never
//! match on the enum — the ledger's sole mutator
//! ([`InflictedInjuries::gain`](crate::injuries::InflictedInjuries::gain)) invokes this
//! trait generically, DIRECTLY on the [`apply_injury`](crate::acts::apply_injury)
//! message-drain path (synchronous — never a deferred command — so the gain trips
//! `Changed<InflictedInjuries>` the same tick the GTW-436 projector re-derives on).

use bevy::prelude::Deref;

use super::MovementCostFactor;
use crate::injuries::{BleedAfflicted, StatDeltaLedger};

/// Whether an injury effect **disables the hand** on its injury's struck arm — the
/// read-side answer [`ApplyInjuryEffect::disables_hand`] returns (folded into
/// [`InflictedInjuries::hands_available`](crate::injuries::InflictedInjuries::hands_available)).
///
/// A named predicate newtype (no-bare-types: "the effect disables a hand" is a domain
/// answer, not a bare `bool`). Private inner, read through the derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandDisabling(bool);

impl HandDisabling {
    /// Wrap a computed hand-disabling answer — the hand-disabling effect builds one from
    /// its resolved `bool` (an implementor lives in a sibling file, so the private inner
    /// is set through this constructor, never a tuple literal).
    #[must_use]
    pub const fn new(disables: bool) -> Self {
        Self(disables)
    }
}

/// The mutable **fold surface** an injury effect folds into — the three accumulators of
/// one ganger's [`InflictedInjuries`](crate::injuries::InflictedInjuries) ledger, lent
/// out by its sole mutator ([`gain`](crate::injuries::InflictedInjuries::gain)) for the
/// duration of one fold (GTW-550).
///
/// STORAGE STAYS ON THE LEDGER (the single `Changed<InflictedInjuries>` source the
/// GTW-436 projector filters on): this is a borrowed VIEW over the ledger's private
/// accumulator fields, never an effect-owned component — an effect that owned its own
/// accumulator would fragment the projector's change-detection filter (the scatter this
/// palette exists to kill). An effect that needs a genuinely NEW accumulator adds the
/// field to the ledger and surfaces it here (one new view field), keeping the ledger the
/// one source of truth.
pub struct LedgerAccumulators<'a> {
    /// The per-[`StatTarget`](crate::injuries::StatTarget) summed-delta store — the
    /// modifier-layer delta source the GTW-436 projector re-sums every projection.
    pub deltas:   &'a mut StatDeltaLedger,
    /// The accrued per-turn HP bleed — mirrored onto the standalone
    /// [`BleedAfflicted`] component the bleed runtime queries.
    pub bleed:    &'a mut BleedAfflicted,
    /// The accumulated MULTIPLICATIVE movement-cost factor (GTW-444) — read by the
    /// pathfinder cost-scale and the committed walk's per-step TU charge.
    pub movement: &'a mut MovementCostFactor,
}

/// Why one effect's [`heal`](ApplyInjuryEffect::heal) could NOT inverse-fold its gain
/// out of the accumulators (GTW-550 — the extensible heal seam GTW-23 Healing will
/// drive).
///
/// A real, well-defined verdict — never a panic and never a silent wrong answer: a
/// healer that receives an `Err` knows the incremental inverse is unavailable and must
/// fall back to REBUILDING the accumulators (remove the healed
/// [`GainedInjury`](crate::injuries::GainedInjury) entry, then re-fold the remaining
/// entries through [`fold_on_gain`](ApplyInjuryEffect::fold_on_gain) — deterministic,
/// since the ledger list is ordered).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealError {
    /// The effect's accumulator folds NON-INVERTIBLY — e.g. an `f32` running product,
    /// where IEEE-754 division is not the exact algebraic inverse of the gain-time
    /// multiply — so the healer must re-fold the remaining ledger entries instead.
    NeedsRefold,
}

/// One injury effect's **isolated behaviour** — the trait each concrete effect type
/// impls, its verbs BEING the effect (GTW-550; the effect-isolation contract, the
/// GTW-558 attachment-palette precedent).
///
/// The whole point: an effect's logic lives in exactly ONE place — its own type's impl
/// in its own palette file — not in a central `match`, a ledger arm, or a scatter of
/// authoring steps. The [`InjuryEffect`](super::InjuryEffect) serde enum impls this by
/// DELEGATING each variant to its isolated type (the palette's ONE mechanical match),
/// and the ledger's [`gain`](crate::injuries::InflictedInjuries::gain) invokes it
/// generically over the borrowed [`LedgerAccumulators`] fold surface.
///
/// The verbs mirror what injury effects actually do:
/// - **fold** ([`fold_on_gain`](Self::fold_on_gain)) — the gain-time accumulator fold
///   (summed deltas, bleed accrual, multiplicative factors). Read-projected effects
///   (e.g. `DisableHand`) fold NOTHING here and surface through a projection instead.
/// - **project** ([`disables_hand`](Self::disables_hand)) — the read-side projections
///   the ledger's on-demand folds ask of each effect (defaulted, so only the effect
///   that carries the projection overrides it).
/// - **heal** ([`heal`](Self::heal)) — the extensible second verb (the GTW-23 seam):
///   the exact inverse of the gain fold where one exists, a documented no-op `Ok` where
///   nothing was accumulated, or [`HealError::NeedsRefold`] where the fold is
///   non-invertible. Never a `todo!`/`unimplemented!`.
pub trait ApplyInjuryEffect {
    /// Fold this effect into the ledger's accumulators at GAIN time — its whole
    /// gain-side behaviour. An effect whose behaviour is read-projected (not
    /// accumulated) implements this as a documented no-op.
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>);

    /// Whether this effect **disables the hand** on its injury's struck arm — the
    /// read-side projection
    /// [`InflictedInjuries::hands_available`](crate::injuries::InflictedInjuries::hands_available)
    /// folds over the ledger's entries (the struck SIDE comes from each entry's
    /// [`part`](crate::injuries::GainedInjury::part), never from the effect).
    ///
    /// Defaulted to [`HandDisabling`]`(false)` so only the hand-disabling effect overrides
    /// it — a new effect file never has to name it.
    fn disables_hand(&self) -> HandDisabling {
        HandDisabling(false)
    }

    /// **Heal** this effect back OUT of the accumulators — the inverse of
    /// [`fold_on_gain`](Self::fold_on_gain), the extensible second verb the future
    /// GTW-23 Healing runtime drives (no runtime caller exists yet; the seam is
    /// exercised by the palette's unit tests).
    ///
    /// Returns `Ok(())` when the inverse fold was applied (or when there is,
    /// by construction, nothing to heal — a documented no-op, e.g. a read-projected
    /// effect healed by removing its ledger entry), and
    /// [`Err(HealError::NeedsRefold)`](HealError::NeedsRefold) when the accumulator
    /// folds non-invertibly and the healer must re-fold the remaining entries.
    ///
    /// # Errors
    ///
    /// [`HealError::NeedsRefold`] — the accumulator cannot be exactly inverse-folded
    /// (e.g. an `f32` running product); rebuild from the remaining ledger entries.
    fn heal(&self, accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError>;
}
