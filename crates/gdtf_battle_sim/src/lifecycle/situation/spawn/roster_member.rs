//! and on WHICH side, with NO authored placement (procgen derives the cell).

use serde::Deserialize;

use crate::ganger::{Faction, GangName, GangerName};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RosterMember {
                /// bare string ([`GangName`] is `#[serde(transparent)]`).
    pub gang:    GangName,
            /// `#[serde(transparent)]`).
    pub member:  GangerName,
                /// gang index ([`Faction`] is `#[serde(transparent)]`).
    pub faction: Faction,
}

impl RosterMember {
        #[must_use]
    pub const fn new(gang: GangName, member: GangerName, faction: Faction) -> Self {
        Self {
            gang,
            member,
            faction,
        }
    }
}
