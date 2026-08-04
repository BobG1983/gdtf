use bevy::prelude::*;
use gdtf_battle_sim::{cover::CoverLedger, prelude::OccupancyGrid};

#[derive(Resource, Debug, Clone, Default, Deref)]
pub(crate) struct ShownOccupancyGrid(OccupancyGrid);

impl ShownOccupancyGrid {
    pub(crate) fn promote(&mut self, live: &OccupancyGrid) {
        self.0 = live.clone();
    }

    #[must_use]
    pub(crate) const fn grid(&self) -> &OccupancyGrid {
        &self.0
    }
}

#[derive(Resource, Debug, Clone, Default, Deref)]
pub(crate) struct ShownCoverLedger(CoverLedger);

impl ShownCoverLedger {
    pub(crate) fn promote(&mut self, live: &CoverLedger) {
        self.0 = live.clone();
    }

    #[must_use]
    pub(crate) const fn ledger(&self) -> &CoverLedger {
        &self.0
    }
}
