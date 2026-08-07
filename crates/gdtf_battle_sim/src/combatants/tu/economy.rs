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

/// How much TU a spend asked for beyond the pool.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuShortfall(u8);

impl TuShortfall {
    /// Wrap a shortfall magnitude.
    #[must_use]
    pub const fn new(short_by: u8) -> Self {
        Self(short_by)
    }
}

/// True if `tu` is at least `cost`.
#[must_use]
pub fn can_spend_tu(tu: &Tu, cost: Tu) -> TuAffordable {
    TuAffordable::new(**tu >= *cost)
}

/// Subtract `cost` from `tu`, or report the shortfall and leave the pool alone.
///
/// # Errors
///
/// Returns [`TuShortfall`] when the pool cannot cover `cost`.
pub fn spend_tu(tu: &mut Tu, cost: Tu) -> Result<(), TuShortfall> {
    match (**tu).checked_sub(*cost) {
        Some(left) => {
            *tu = Tu::new(left);
            Ok(())
        }
        None => Err(TuShortfall::new((*cost).saturating_sub(**tu))),
    }
}

/// Set `tu` back to `max`.
pub fn reset_tu(tu: &mut Tu, max: &TuMax) {
    *tu = Tu::new(**max);
}
