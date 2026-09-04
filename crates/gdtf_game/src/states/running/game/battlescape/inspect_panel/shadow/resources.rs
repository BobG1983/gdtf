use bevy::{platform::collections::HashMap, prelude::*};
use gdtf_battle_sim::{
    cover::CoverLedger,
    emplacement::{EmplacementState, MountedWeaponKey},
    prelude::{CellLevel, OccupancyGrid},
};

crate::support_item! {
    /// The occupancy grid the screen is drawing, which lags the sim while an act plays out.
    #[derive(Resource, Debug, Clone, Default, Deref)]
    pub(crate) struct ShownOccupancyGrid(OccupancyGrid);
}

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

/// What an emplacement cell shows: whether it is manned, and the weapon it mounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShownEmplacement {
    state:  EmplacementState,
    weapon: MountedWeaponKey,
}

impl ShownEmplacement {
    pub(crate) const fn new(state: EmplacementState, weapon: MountedWeaponKey) -> Self {
        Self { state, weapon }
    }

    #[must_use]
    pub(crate) const fn state(&self) -> EmplacementState {
        self.state
    }

    #[must_use]
    pub(crate) const fn weapon(&self) -> &MountedWeaponKey {
        &self.weapon
    }
}

/// The emplacements the screen is drawing, by the cell each one stands on.
#[derive(Resource, Debug, Clone, Default, Deref)]
pub(crate) struct ShownEmplacements(HashMap<CellLevel, ShownEmplacement>);

impl ShownEmplacements {
    pub(crate) fn promote(&mut self, live: impl Iterator<Item = (CellLevel, ShownEmplacement)>) {
        self.0 = live.collect();
    }

    #[must_use]
    pub(crate) fn peek(&self, cell: &CellLevel) -> Option<&ShownEmplacement> {
        self.0.get(cell)
    }
}
