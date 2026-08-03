use bevy::prelude::Deref;

use super::MovementCostFactor;
use crate::injuries::{BleedAfflicted, StatDeltaLedger};

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandDisabling(bool);

impl HandDisabling {
    #[must_use]
    pub const fn new(disables: bool) -> Self {
        Self(disables)
    }
}

pub struct LedgerAccumulators<'a> {
    pub deltas:   &'a mut StatDeltaLedger,
    pub bleed:    &'a mut BleedAfflicted,
    pub movement: &'a mut MovementCostFactor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealError {
    NeedsRefold,
}

pub trait ApplyInjuryEffect {
    fn fold_on_gain(&self, accumulators: &mut LedgerAccumulators<'_>);

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
