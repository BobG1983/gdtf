//! The root seed the live battle was actually generated from.

use bevy::prelude::*;
use gdtf_battle_sim::rng::BattleSeed;

crate::support_item! {
    /// The root seed this battle used, recorded wherever the seed is resolved.
    #[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct ResolvedBattleSeed(BattleSeed);
}

impl ResolvedBattleSeed {
    crate::support_item! {
        /// Record a resolved root seed.
        #[must_use]
        const fn new(seed: BattleSeed) -> Self {
            Self(seed)
        }
    }
}
