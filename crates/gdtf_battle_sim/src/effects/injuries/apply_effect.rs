//! Trait and helpers for folding injury effects into ledgers.

use bevy::prelude::Deref;

use super::MovementCostFactor;
use crate::injuries::{BleedAfflicted, StatDeltaLedger};

/// Whether an injury disables a hand.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandDisabling(bool);

impl HandDisabling {
    /// Wrap the flag.
    #[must_use]
    pub const fn new(disables: bool) -> Self {
        Self(disables)
    }
}

/// Mutable ledger handles used while folding injury effects.
pub struct LedgerAccumulators<'a> {
    /// Stat delta ledger.
    pub deltas:   &'a mut StatDeltaLedger,
    /// Bleed accrual.
    pub bleed:    &'a mut BleedAfflicted,
    /// Movement cost factor.
    pub movement: &'a mut MovementCostFactor,
}

/// Why a heal could not be applied as a simple reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealError {
    /// Caller must rebuild the ledger from remaining injuries.
    NeedsRefold,
}

/// Behaviour an injury effect applies on gain and heal.
pub trait ApplyInjuryEffect {
    /// Fold this effect into the accumulators on gain.
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>);

    /// Whether this effect disables a hand.
    fn disables_hand(&self) -> HandDisabling {
        HandDisabling(false)
    }

    /// Undo this effect on heal.
    ///
    /// # Errors
    ///
    /// Returns [`HealError::NeedsRefold`] when the ledger must be rebuilt from remaining injuries
    /// instead of a simple reverse delta.
    fn heal(&self, accumulators: &mut LedgerAccumulators<'_>) -> Result<(), HealError>;
}
