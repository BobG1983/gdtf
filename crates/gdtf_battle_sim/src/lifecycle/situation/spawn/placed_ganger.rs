use serde::Deserialize;

use crate::{
    ganger::{Aiming, Facing, Faction, GangName, GangerName, LifeState, Stance},
    metric::CellLevel,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlacedGanger {
                        /// error (no panic). Authored as a bare string ([`GangName`] is `#[serde(transparent)]`).
    pub gang:       GangName,
                    /// error (no panic). Authored as a bare string ([`GangerName`] is `#[serde(transparent)]`).
    pub member:     GangerName,
        pub at:         CellLevel,
            pub faction:    Faction,
        pub facing:     Facing,
        pub stance:     Stance,
        pub aiming:     Aiming,
        pub life_state: LifeState,
}

impl PlacedGanger {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
        pub at:         CellLevel,
        pub faction:    Faction,
        pub facing:     Facing,
        pub stance:     Stance,
        pub aiming:     Aiming,
        pub life_state: LifeState,
}

impl Placement {
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
