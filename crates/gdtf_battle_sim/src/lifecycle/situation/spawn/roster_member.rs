//! Roster entry without a fixed cell (procgen places it).

use serde::{Deserialize, Serialize};

use crate::ganger::{Faction, GangName, GangerName};

/// One ganger drawn from a gang roster for side placement.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RosterMember {
    /// Gang key.
    pub gang:    GangName,
    /// Member key within the gang.
    pub member:  GangerName,
    /// Faction / side index.
    pub faction: Faction,
}

impl RosterMember {
    /// Build a roster member.
    #[must_use]
    pub const fn new(gang: GangName, member: GangerName, faction: Faction) -> Self {
        Self {
            gang,
            member,
            faction,
        }
    }
}
