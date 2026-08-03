//! Choose which fog set a mover uses for path planning.

use bevy::prelude::{Deref, Resource};

use crate::{ganger::Faction, visibility::SquadVisibility};

/// Omniscient fog resource used by AI movers.
#[derive(Resource, Deref, Debug, Clone, PartialEq, Eq)]
pub struct OmniscientFog(SquadVisibility);

impl OmniscientFog {
    #[must_use]
    pub const fn new(fog: SquadVisibility) -> Self {
        Self(fog)
    }
}

/// Player movers use player fog; enemy movers use omniscient fog.
#[must_use]
pub fn move_fog<'a>(
    mover_faction: Faction,
    player_faction: Faction,
    player_fog: &'a SquadVisibility,
    omniscient: &'a SquadVisibility,
) -> &'a SquadVisibility {
    if mover_faction == player_faction {
        player_fog
    } else {
        omniscient
    }
}
