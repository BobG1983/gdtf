//! Explicitly placed ganger and its placement bag.

use serde::Deserialize;

use crate::{
    ganger::{Aiming, Facing, Faction, GangName, GangerName, LifeState, Stance},
    metric::CellLevel,
};

/// One ganger placed at a fixed cell from a gang roster.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlacedGanger {
    /// Gang key.
    pub gang: GangName,
    /// Member key within the gang.
    pub member: GangerName,
    /// Spawn cell.
    pub at: CellLevel,
    /// Faction index.
    pub faction: Faction,
    /// Facing.
    pub facing: Facing,
    /// Stance.
    pub stance: Stance,
    /// Aiming state.
    pub aiming: Aiming,
    /// Life state.
    pub life_state: LifeState,
}

impl PlacedGanger {
    /// From gang, member, and placement bag.
    #[must_use]
    pub const fn new(gang: GangName, member: GangerName, placement: Placement) -> Self {
        Self {
            gang,
            member,
            at: placement.at,
            faction: placement.faction,
            facing: placement.facing,
            stance: placement.stance,
            aiming: placement.aiming,
            life_state: placement.life_state,
        }
    }
}

/// Cell and combat state for a placed ganger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// Spawn cell.
    pub at: CellLevel,
    /// Faction index.
    pub faction: Faction,
    /// Facing.
    pub facing: Facing,
    /// Stance.
    pub stance: Stance,
    /// Aiming state.
    pub aiming: Aiming,
    /// Life state.
    pub life_state: LifeState,
}

impl Placement {
    /// Build a placement bag.
    #[must_use]
    pub const fn new(
        at: CellLevel,
        faction: Faction,
        facing: Facing,
        stance: Stance,
        aiming: Aiming,
        life_state: LifeState,
    ) -> Self {
        Self {
            at,
            faction,
            facing,
            stance,
            aiming,
            life_state,
        }
    }
}
