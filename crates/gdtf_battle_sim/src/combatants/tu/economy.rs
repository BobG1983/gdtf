//! Check, spend, and refill a ganger's time units.

use bevy::prelude::Deref;

use crate::ganger::{Tu, TuMax};

/// Whether a TU cost can be paid.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuAffordable(bool);

impl TuAffordable {
    /// Wrap a boolean.
    #[must_use]
    pub const fn new(affordable: bool) -> Self {
        Self(affordable)
    }
}

/// True if `tu` is at least `cost`.
#[must_use]
pub fn can_spend_tu(tu: &Tu, cost: Tu) -> TuAffordable {
    TuAffordable::new(**tu >= *cost)
}

/// Subtract `cost` from `tu` (saturating).
pub fn spend_tu(tu: &mut Tu, cost: Tu) {
    *tu = Tu::new(tu.saturating_sub(*cost));
}

/// Set `tu` back to `max`.
pub fn reset_tu(tu: &mut Tu, max: &TuMax) {
    *tu = Tu::new(**max);
}
