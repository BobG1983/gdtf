//! Ganger placements decided before generation ran, used instead of deploying rosters.

use bevy::prelude::*;
use gdtf_battle_sim::situation::PlacedGanger;

crate::support_item! {
    /// Placements a caller settled ahead of generation, so the roster deploy is skipped.
    #[derive(Resource, Deref, Debug, Clone, Default, PartialEq, Eq)]
    struct PreplacedGangers(Vec<PlacedGanger>);
}

#[cfg(feature = "headless_test")]
impl PreplacedGangers {
    crate::support_item! {
        /// Carry placements that were settled before generation.
        #[must_use]
        const fn new(placements: Vec<PlacedGanger>) -> Self {
            Self(placements)
        }
    }
}
