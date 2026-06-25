//! The TU-economy verb implementations — [`can_spend_tu`] / [`spend_tu`] / [`reset_tu`].
//! See the module docs (`super`) for the saturating-arithmetic contract.

use crate::ganger::{Tu, TuMax};

/// Whether a ganger's current pool can afford a cost — `true` iff `current >= cost`.
///
/// The affordability predicate the firing / posture / movement acts gate on before
/// charging TU (resolution.md §"What's pure math vs sim": `can_spend_tu` is
/// model-authoritative). The `cost` is a [`Tu`] (the named TU amount, not a bare `u8`).
/// Equality affords (a cost exactly equal to the pool is payable).
#[must_use]
pub fn can_spend_tu(tu: &Tu, cost: Tu) -> bool {
    **tu >= *cost
}

/// Charge `cost` against a ganger's current pool — **saturating** subtraction.
///
/// Spends the cost from [`Tu`], flooring at `0` when the cost exceeds the pool
/// (`u8::saturating_sub`): overspending leaves the pool empty, **never** wraps/underflows
/// to a large value. Callers that must not overspend gate with [`can_spend_tu`] first; this
/// verb stays total and safe regardless. resolution.md §"What's pure math vs sim":
/// `spend_tu` is model-authoritative.
pub fn spend_tu(tu: &mut Tu, cost: Tu) {
    *tu = Tu::new(tu.saturating_sub(*cost));
}

/// Restore a ganger's current pool to its round-start maximum.
///
/// Sets [`Tu`] equal to the ganger's [`TuMax`] — the round-start budget refill
/// (resolution.md §"What's pure math vs sim": `reset_tu` is model-authoritative; §8: the
/// `TU_max` denominator GTW-38's reaction score reads). Independent of the pre-reset pool:
/// a drained ganger is brought back to full.
pub fn reset_tu(tu: &mut Tu, max: &TuMax) {
    *tu = Tu::new(**max);
}
